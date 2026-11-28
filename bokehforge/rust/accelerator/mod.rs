mod device;
mod diagnostics;
mod kernels;

pub(crate) use diagnostics::register;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::LazyLock;

use cubecl::bytes::Bytes;
use cubecl::client::Client;
use cubecl::prelude::*;
use cubecl::server::Handle;
use cubecl::std::tensor::TensorHandle;

const SIGMAS: [f64; 4] = [0.85, 1.1, 2.5, 10.0];
const RADII: [usize; 4] = [3, 4, 10, 40];
const HALO: usize = RADII[3];
const TILE_PIXELS: usize = 4_000_000;
pub(crate) const MAX_RESIDENT_PIXELS: usize = 16_000_000;
const MAX_RESIDENT_HALO_PIXELS: usize = 18_000_000;

pub(crate) fn available(device: &str) -> bool {
    availability(device).is_ok()
}

pub(crate) fn cuda_resident_headroom() -> bool {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        use std::sync::{Arc, OnceLock};

        // Includes the retained tile maps, temporary device scratch, and
        // room for continued execution of a warmed Inspection workload.
        const RESERVE_BYTES: u64 = 4 << 30;
        static CONTEXT: OnceLock<Option<Arc<cudarc::driver::CudaContext>>> = OnceLock::new();
        CONTEXT
            .get_or_init(|| cudarc::driver::CudaContext::new(0).ok())
            .as_ref()
            .and_then(|context| context.mem_get_info().ok())
            .is_some_and(|(free, _)| free as u64 >= RESERVE_BYTES)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    false
}

pub(crate) fn release_idle_cuda_memory() {
    let _ = run("cuda", |client| {
        client.memory_cleanup().map_err(|error| error.to_string())
    });
}

pub(crate) fn availability(device: &str) -> Result<(), String> {
    device::client(device).map(|_| ())
}

pub(crate) fn lightness(
    device: &str,
    height: usize,
    width: usize,
    candidate: &[f32],
    target: &[f32],
    delta: &[f32],
    mask: Option<&[u8]>,
) -> Result<[Vec<f32>; 4], String> {
    let image = Image::checked(height, width, [candidate, target, delta], mask)?;
    run(device, |client| execute(client, image))
}

/// Return the four lightness families on the common four-pixel tile lattice.
/// Each cell contains the high and low words of the supported damage sum and
/// the count of supported pixels at or above the affected threshold. A supplied
/// Inspection plane multiplies damage before both quantities are measured.
pub(crate) fn lightness_cells(
    device: &str,
    height: usize,
    width: usize,
    planes: [&[f32]; 3],
    mask: Option<&[u8]>,
    heat: Option<&[f32]>,
    threshold: f32,
) -> Result<[Vec<[f32; 3]>; 4], String> {
    let image = Image::checked(height, width, planes, mask)?;
    if heat.is_some_and(|weights| weights.len() != height * width) {
        return Err("Inspection weight plane does not match the lightness grid".to_owned());
    }
    run(device, |client| {
        execute_cells(client, image, heat, threshold)
    })
}

/// Consume tile-major lightness allocations from Lab preparation directly.
/// The three signed planes in each allocation cover the plan's halo window.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lightness_packed_cells(
    device: &str,
    height: usize,
    width: usize,
    plan: &[TilePlan],
    packed: Vec<Vec<f32>>,
    mask: Option<&[u8]>,
    heat: Option<&[f32]>,
    threshold: f32,
) -> Result<[Vec<[f32; 3]>; 4], String> {
    let area = height
        .checked_mul(width)
        .ok_or("invalid lightness dimensions")?;
    if mask.is_some_and(|values| values.len() != area)
        || heat.is_some_and(|weights| weights.len() != area)
        || plan.len() != packed.len()
    {
        return Err("Packed lightness inputs do not match the overlap grid".to_owned());
    }
    let tiles = plan.iter().zip(packed).map(|(plan, samples)| {
        (
            Window::from_bounds(plan.core),
            TileInput {
                window: Window::from_bounds(plan.halo),
                image_width: width,
                mask,
                samples,
            },
        )
    });
    run(device, |client| {
        reduce_cells(client, height, width, tiles, heat, threshold)
    })
}

fn run<T>(device: &str, execute: impl FnOnce(&Client) -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(|| execute(&device::client(device)?))).map_err(|failure| {
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .unwrap_or("runtime panicked");
        format!("{device} damage accelerator failed: {message}")
    })?
}

fn weights() -> &'static [Vec<f32>; 4] {
    static KERNELS: LazyLock<[Vec<f32>; 4]> = LazyLock::new(|| {
        std::array::from_fn(|index| {
            let sigma = SIGMAS[index];
            let radius = RADII[index] as i32;
            let scale = -0.5 / (sigma * sigma);
            let gaussian = |distance: i32| (scale * f64::from(distance * distance)).exp();
            let normalizer: f64 = (-radius..=radius).map(gaussian).sum();
            (-radius..=0)
                .map(|distance| (gaussian(distance) / normalizer) as f32)
                .collect()
        })
    });
    &KERNELS
}

#[derive(Clone, Copy)]
struct Image<'a> {
    height: usize,
    width: usize,
    planes: [&'a [f32]; 3],
    mask: Option<&'a [u8]>,
}

impl<'a> Image<'a> {
    fn checked(
        height: usize,
        width: usize,
        planes: [&'a [f32]; 3],
        mask: Option<&'a [u8]>,
    ) -> Result<Self, String> {
        let area = height
            .checked_mul(width)
            .filter(|&size| size != 0 && planes.iter().all(|plane| plane.len() == size))
            .ok_or("accelerator lightness planes have invalid dimensions")?;
        if mask.is_some_and(|values| values.len() != area) {
            return Err("accelerator lightness mask has invalid dimensions".to_owned());
        }
        Ok(Self {
            height,
            width,
            planes,
            mask,
        })
    }
}

#[derive(Clone, Copy)]
struct Window {
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,
}

impl Window {
    fn bounds(self) -> [usize; 4] {
        [self.top, self.left, self.bottom, self.right]
    }

    fn from_bounds([top, left, bottom, right]: [usize; 4]) -> Self {
        Self {
            top,
            left,
            bottom,
            right,
        }
    }

    fn width(self) -> usize {
        self.right - self.left
    }

    fn height(self) -> usize {
        self.bottom - self.top
    }

    fn halo(self, height: usize, width: usize) -> Self {
        Self {
            top: self.top.saturating_sub(HALO),
            left: self.left.saturating_sub(HALO),
            bottom: self.bottom.saturating_add(HALO).min(height),
            right: self.right.saturating_add(HALO).min(width),
        }
    }

    fn rows(self, image_width: usize) -> impl Iterator<Item = std::ops::Range<usize>> {
        (self.top..self.bottom).map(move |row| {
            let start = row * image_width + self.left;
            start..start + self.width()
        })
    }
}

fn windows(height: usize, width: usize, align: usize) -> impl Iterator<Item = Window> {
    let area = width * height;

    // Keep every tile including its halo bounded. Align core edges to the
    // base-cell grid so a cell always belongs to exactly one dispatch.
    let (tile_height, tile_width) = if area <= TILE_PIXELS {
        (height, width)
    } else {
        let min_rows_with_halo = height.min(2 * HALO + align);
        let max_cols = (TILE_PIXELS / min_rows_with_halo).saturating_sub(2 * HALO);
        let tile_width = if max_cols >= width {
            width
        } else {
            max_cols / align * align
        };
        let padded_width = (tile_width + 2 * HALO).min(width);
        let max_rows = TILE_PIXELS / padded_width;
        let tile_height = if max_rows >= height {
            height
        } else {
            max_rows.saturating_sub(2 * HALO) / align * align
        };
        (tile_height, tile_width)
    };
    (0..height).step_by(tile_height).flat_map(move |top| {
        (0..width).step_by(tile_width).map(move |left| Window {
            top,
            left,
            bottom: top.saturating_add(tile_height).min(height),
            right: left.saturating_add(tile_width).min(width),
        })
    })
}

/// The committed cell region and its reflected filter input, in TLBR order.
#[derive(Clone, Copy)]
pub(crate) struct TilePlan {
    pub(crate) core: [usize; 4],
    pub(crate) halo: [usize; 4],
}

pub(crate) fn lightness_plan(height: usize, width: usize) -> Vec<TilePlan> {
    windows(height, width, 4)
        .map(|core| TilePlan {
            core: core.bounds(),
            halo: core.halo(height, width).bounds(),
        })
        .collect()
}

fn execute(client: &Client, image: Image<'_>) -> Result<[Vec<f32>; 4], String> {
    let Image { height, width, .. } = image;
    let area = width * height;
    let weights = weights()
        .each_ref()
        .map(|values| Buffer::upload(client, values.clone()));
    let mut output: [Vec<f32>; 4] = std::array::from_fn(|_| vec![0.; area]);
    for core in windows(height, width, 1) {
        let tile = core.halo(height, width);
        let maps =
            execute_tile(client, pack_lightness(image, tile), &weights, None)?.read(client)?;
        let planes = f32::from_bytes(&maps).chunks_exact(tile.height() * tile.width());
        let inner = Window {
            top: core.top - tile.top,
            left: core.left - tile.left,
            bottom: core.bottom - tile.top,
            right: core.right - tile.left,
        };
        for (output, samples) in output.iter_mut().zip(planes) {
            for (destination, source) in core.rows(width).zip(inner.rows(tile.width())) {
                output[destination].copy_from_slice(&samples[source]);
            }
        }
    }
    Ok(output)
}

struct CellReadout<'a> {
    core: Window,
    weights: Option<&'a [f32]>,
    threshold: f32,
}

fn execute_cells(
    client: &Client,
    image: Image<'_>,
    heat: Option<&[f32]>,
    threshold: f32,
) -> Result<[Vec<[f32; 3]>; 4], String> {
    let Image { height, width, .. } = image;
    let tiles = windows(height, width, 4)
        .map(|core| (core, pack_lightness(image, core.halo(height, width))));
    reduce_cells(client, height, width, tiles, heat, threshold)
}

fn reduce_cells<'a>(
    client: &Client,
    height: usize,
    width: usize,
    tiles: impl Iterator<Item = (Window, TileInput<'a>)>,
    heat: Option<&[f32]>,
    threshold: f32,
) -> Result<[Vec<[f32; 3]>; 4], String> {
    let columns = width.div_ceil(4);
    let count = height.div_ceil(4) * columns;
    let weights = weights()
        .each_ref()
        .map(|values| Buffer::upload(client, values.clone()));
    let mut output: [Vec<[f32; 3]>; 4] = std::array::from_fn(|_| vec![[0.; 3]; count]);
    for (core, input) in tiles {
        let data = execute_tile(
            client,
            input,
            &weights,
            Some(CellReadout {
                core,
                weights: heat,
                threshold,
            }),
        )?
        .read(client)?;
        write_cells(&mut output, columns, core, &data);
    }
    Ok(output)
}

fn write_cells(output: &mut [Vec<[f32; 3]>; 4], columns: usize, core: Window, data: &Bytes) {
    let core_columns = core.width().div_ceil(4);
    let core_count = core.height().div_ceil(4) * core_columns;
    let planes = f32::from_bytes(data)
        .as_chunks::<3>()
        .0
        .chunks_exact(core_count);
    let cell_window = Window {
        top: core.top / 4,
        left: core.left / 4,
        bottom: core.bottom.div_ceil(4),
        right: core.right.div_ceil(4),
    };
    for (output, samples) in output.iter_mut().zip(planes) {
        for (destination, source) in cell_window
            .rows(columns)
            .zip(samples.chunks_exact(core_columns))
        {
            output[destination].copy_from_slice(source);
        }
    }
}

struct ResidentTile {
    core: Window,
    halo: Window,
    damage: Buffer,
    mask: Option<Buffer>,
}

pub(crate) struct DeferredLightness {
    client: Client,
    device: String,
    height: usize,
    width: usize,
    tiles: Vec<ResidentTile>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lightness_deferred(
    device: &str,
    height: usize,
    width: usize,
    plan: &[TilePlan],
    packed: Vec<Vec<f32>>,
    mask: Option<&[u8]>,
) -> Result<DeferredLightness, String> {
    let pixels = height
        .checked_mul(width)
        .ok_or("invalid deferred lightness dimensions")?;
    if pixels == 0
        || pixels > MAX_RESIDENT_PIXELS
        || packed.len() != plan.len()
        || mask.is_some_and(|mask| mask.len() != pixels)
    {
        return Err("Deferred lightness inputs do not match the overlap grid".into());
    }
    // Check the raw buffers including repeated halo rows rather than
    // admitting unbounded extra temporary volumes on thin overlap grids.
    let resident_pixels: usize = plan
        .iter()
        .map(|entry| {
            let halo = Window::from_bounds(entry.halo);
            halo.height() * halo.width()
        })
        .sum();
    if resident_pixels > MAX_RESIDENT_HALO_PIXELS {
        return Err("Deferred lightness exceeds the resident tile budget".into());
    }

    run(device, |client| {
        let weights = weights()
            .each_ref()
            .map(|values| Buffer::upload(client, values.clone()));
        let mut tiles = Vec::with_capacity(plan.len());
        for (entry, samples) in plan.iter().zip(packed) {
            let core = Window::from_bounds(entry.core);
            let halo = Window::from_bounds(entry.halo);
            let mask_tile =
                mask.filter(|mask| halo.rows(width).any(|rows| mask[rows].contains(&0)));
            let input = TileInput {
                window: halo,
                image_width: width,
                mask: mask_tile,
                samples,
            };
            let damage = execute_tile(client, input, &weights, None)?;
            let mask = mask_tile.map(|mask| {
                Buffer::upload(
                    client,
                    halo.rows(width)
                        .flat_map(|row| {
                            mask[row]
                                .iter()
                                .map(|&valid| f32::from(u8::from(valid != 0)))
                        })
                        .collect(),
                )
            });
            // Keep only the completed output and support. Retire the
            // previous tile's scratch before submitting the next tile.
            cubecl::future::reader::read_sync(client.sync_buffers([&damage.handle]))
                .map_err(|error| error.to_string())?;
            tiles.push(ResidentTile {
                core,
                halo,
                damage,
                mask,
            });
        }
        Ok(DeferredLightness {
            client: client.clone(),
            device: device.to_owned(),
            height,
            width,
            tiles,
        })
    })
}

impl DeferredLightness {
    pub(crate) fn finish(
        self,
        heat: Option<&[f32]>,
        threshold: f32,
    ) -> Result<[Vec<[f32; 3]>; 4], String> {
        let Self {
            client,
            device,
            height,
            width,
            tiles,
        } = self;
        if heat.is_some_and(|map| map.len() != height * width) {
            return Err("Inspection weight plane does not match the deferred grid".into());
        }
        run(&device, |_| {
            let columns = width.div_ceil(4);
            let count = height.div_ceil(4) * columns;
            let mut result = std::array::from_fn(|_| vec![[0.; 3]; count]);
            for ResidentTile {
                core,
                halo,
                damage,
                mask,
            } in tiles
            {
                let area = halo.height() * halo.width();
                let job = Job {
                    client: &client,
                    height: halo.height(),
                    width: halo.width(),
                    area,
                    masked: mask.is_some(),
                };
                let weights = heat.map(|map| {
                    Buffer::upload(
                        &client,
                        core.rows(width)
                            .flat_map(|row| map[row].iter().copied())
                            .collect(),
                    )
                });
                let cells = core.height().div_ceil(4) * core.width().div_ceil(4);
                let compact = Buffer::empty(&client, 12 * cells);
                let (grid, block, span) = job.dispatch(cells);
                kernels::finish_cells::launch(
                    &client,
                    grid,
                    block,
                    mask.as_ref().unwrap_or(&damage).arg(),
                    damage.arg(),
                    damage.arg(),
                    damage.arg(),
                    weights.as_ref().unwrap_or(&damage).arg(),
                    compact.arg(),
                    halo.width(),
                    area,
                    core.top - halo.top,
                    core.left - halo.left,
                    core.height(),
                    core.width(),
                    threshold,
                    mask.is_some(),
                    weights.is_some(),
                    true,
                    span,
                );
                let data = compact.read(&client)?;
                write_cells(&mut result, columns, core, &data);
            }
            Ok(result)
        })
    }
}

struct Buffer {
    handle: Handle,
    len: usize,
}

impl Buffer {
    fn upload(client: &Client, samples: Vec<f32>) -> Self {
        let len = samples.len();
        Self {
            handle: client.create(Bytes::from_elems(samples)),
            len,
        }
    }

    fn empty(client: &Client, length: usize) -> Self {
        Self {
            handle: client.empty(length * size_of::<f32>()),
            len: length,
        }
    }

    fn arg(&self) -> TensorArg {
        TensorHandle::new_contiguous([self.len], self.handle.clone(), f32::cube_type()).into_arg()
    }

    fn read(self, client: &Client) -> Result<Bytes, String> {
        let bytes = client
            .read_one(self.handle)
            .map_err(|error| error.to_string())?;
        if bytes.len() != self.len * size_of::<f32>() {
            return Err("accelerator did not return the expected damage buffer size".to_owned());
        }
        if !bytes.as_ptr().cast::<f32>().is_aligned() {
            return Err("unaligned accelerator damage buffer".to_owned());
        }
        Ok(bytes)
    }
}

struct Job<'a> {
    client: &'a Client,
    height: usize,
    width: usize,
    area: usize,
    masked: bool,
}

impl Job<'_> {
    fn dispatch(&self, count: usize) -> (CubeCount, CubeDim, usize) {
        // A CPU unit owns a contiguous span to amortize dispatch and avoid
        // workers writing different lanes of the same cache line.
        let span = if self.client.properties().hardware.num_cpu_cores.is_some() {
            64
        } else {
            1
        };
        let units = count.div_ceil(span);
        let dim = CubeDim::new(self.client, units.min(128));
        (
            cubecl::calculate_cube_count_elemwise(self.client, units, dim),
            dim,
            span,
        )
    }

    fn blur(
        &self,
        source: &Buffer,
        weights: &Buffer,
        scratch: [&Buffer; 2],
        first: usize,
        planes: usize,
        radius: usize,
    ) {
        let [vertical, horizontal] = scratch;
        // The stencil owns row-local spans so threads can share tap addresses
        // across columns without crossing a reflected row boundary.
        let span = if self.client.properties().hardware.num_cpu_cores.is_some() {
            16
        } else {
            1
        };
        let units = self.width.div_ceil(span) * self.height * planes;
        let dim = CubeDim::new(self.client, units.min(128));
        for (input, output, first_plane, pass_y) in [
            (source, vertical, first, true),
            (vertical, horizontal, 0, false),
        ] {
            let count = cubecl::calculate_cube_count_elemwise(self.client, units, dim);
            kernels::gaussian::launch(
                self.client,
                count,
                dim,
                input.arg(),
                weights.arg(),
                output.arg(),
                self.width,
                self.height,
                planes,
                first_plane,
                radius,
                pass_y,
                span,
            );
        }
        if self.masked {
            let (count, dim, span) = self.dispatch(self.area * (planes - 1));
            kernels::normalize::launch(
                self.client,
                count,
                dim,
                horizontal.arg(),
                self.area,
                planes - 1,
                span,
            );
        }
    }

    fn contrast(&self, samples: &Buffer, original: &Buffer, output: &Buffer, initial: bool) {
        let (count, dim, span) = self.dispatch(self.area);
        kernels::directional::launch(
            self.client,
            count,
            dim,
            samples.arg(),
            original.arg(),
            output.arg(),
            self.width,
            self.height,
            self.masked,
            initial,
            span,
        );
    }

    fn copy_residual(&self, source: &Buffer, residuals: &Buffer, plane: usize, destination: usize) {
        let (count, dim, span) = self.dispatch(self.area);
        kernels::copy_plane::launch(
            self.client,
            count,
            dim,
            source.arg(),
            residuals.arg(),
            self.area,
            plane,
            destination,
            span,
        );
    }
}

struct TileInput<'a> {
    window: Window,
    image_width: usize,
    mask: Option<&'a [u8]>,
    samples: Vec<f32>,
}

fn pack_lightness(image: Image<'_>, tile: Window) -> TileInput<'_> {
    let area = tile.width() * tile.height();
    let mut samples = Vec::with_capacity(area * (3 + usize::from(image.mask.is_some())));
    for plane in image.planes {
        for row in tile.rows(image.width) {
            samples.extend_from_slice(&plane[row]);
        }
    }
    TileInput {
        window: tile,
        image_width: image.width,
        mask: image.mask,
        samples,
    }
}

fn execute_tile(
    client: &Client,
    input: TileInput<'_>,
    weights: &[Buffer; 4],
    cells: Option<CellReadout<'_>>,
) -> Result<Buffer, String> {
    let TileInput {
        window: tile,
        image_width,
        mask,
        mut samples,
    } = input;
    let (height, width) = (tile.height(), tile.width());
    let area = height * width;
    if samples.len() != 3 * area {
        return Err("Packed lightness planes do not match the halo grid".to_owned());
    }
    // The normalized filters of an all-one support window are exactly one,
    // including the reflected halo and the intermediate float32 pass.
    let mask = mask.filter(|mask| tile.rows(image_width).any(|row| mask[row].contains(&0)));
    let masked = mask.is_some();
    let plane_count = if masked { 4 } else { 3 };
    let job = Job {
        client,
        height,
        width,
        area,
        masked,
    };
    if let Some(mask) = mask {
        for row in tile.rows(image_width) {
            samples.extend(
                mask[row]
                    .iter()
                    .map(|&value| f32::from(u8::from(value != 0))),
            );
        }
    }
    let original = Buffer::upload(client, samples);
    let masked_samples;
    let filter_source = if masked {
        masked_samples = Buffer::empty(client, area * plane_count);
        let (count, dim, span) = job.dispatch(area * plane_count);
        kernels::mask_samples::launch(
            client,
            count,
            dim,
            original.arg(),
            masked_samples.arg(),
            area,
            span,
        );
        &masked_samples
    } else {
        &original
    };

    let output = Buffer::empty(client, 4 * area);
    let residuals = Buffer::empty(client, 3 * area);
    let vertical = Buffer::empty(client, plane_count * area);
    let horizontal = Buffer::empty(client, plane_count * area);
    let scratch = [&vertical, &horizontal];
    job.contrast(&original, &original, &output, true);

    // The local scale contributes to tone, boundary, and signed detail.
    job.blur(
        filter_source,
        &weights[1],
        scratch,
        0,
        if masked { 4 } else { 2 },
        RADII[1],
    );
    job.contrast(&horizontal, &original, &output, false);
    job.blur(
        filter_source,
        &weights[2],
        scratch,
        0,
        plane_count,
        RADII[2],
    );
    job.contrast(&horizontal, &original, &output, false);
    job.copy_residual(&horizontal, &residuals, 2, 1);
    for (scale, plane) in [(0, 0), (3, 2)] {
        job.blur(
            filter_source,
            &weights[scale],
            scratch,
            2,
            if masked { 2 } else { 1 },
            RADII[scale],
        );
        job.copy_residual(&horizontal, &residuals, 0, plane);
    }

    let edges = Buffer::empty(client, 4 * area);
    let (count, dim, span) = job.dispatch(area);
    kernels::extrema::launch(
        client,
        count,
        dim,
        original.arg(),
        edges.arg(),
        width,
        height,
        span,
    );
    let (count, dim, span) = job.dispatch(area);
    kernels::nearby::launch(
        client,
        count,
        dim,
        edges.arg(),
        vertical.arg(),
        width,
        height,
        false,
        span,
    );
    let (count, dim, span) = job.dispatch(area);
    kernels::nearby::launch(
        client,
        count,
        dim,
        vertical.arg(),
        edges.arg(),
        width,
        height,
        true,
        span,
    );
    if let Some(readout) = cells {
        let core = readout.core;
        let heat = readout.weights.map(|weights| {
            let mut tile = Vec::with_capacity(core.height() * core.width());
            for row in core.rows(image_width) {
                tile.extend_from_slice(&weights[row]);
            }
            Buffer::upload(client, tile)
        });
        let cells = core.height().div_ceil(4) * core.width().div_ceil(4);
        let compact = Buffer::empty(client, 4 * 3 * cells);
        let (count, dim, span) = job.dispatch(cells);
        kernels::finish_cells::launch(
            client,
            count,
            dim,
            original.arg(),
            residuals.arg(),
            edges.arg(),
            output.arg(),
            heat.as_ref().unwrap_or(&original).arg(),
            compact.arg(),
            width,
            area,
            core.top - tile.top,
            core.left - tile.left,
            core.height(),
            core.width(),
            readout.threshold,
            masked,
            heat.is_some(),
            false,
            span,
        );
        return Ok(compact);
    }
    let (count, dim, span) = job.dispatch(area);
    kernels::finish::launch(
        client,
        count,
        dim,
        original.arg(),
        residuals.arg(),
        edges.arg(),
        output.arg(),
        area,
        span,
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{lightness, lightness_cells, lightness_deferred, lightness_plan};

    #[test]
    fn deferred_cells_keep_masked_edges_and_inspection() {
        let (height, width) = (17, 23);
        let pixels = height * width;
        let candidate: Vec<_> = (0..pixels).map(|i| ((i * 31) % 97) as f32).collect();
        let target: Vec<_> = (0..pixels).map(|i| ((i * 47) % 89) as f32).collect();
        let delta: Vec<_> = candidate.iter().zip(&target).map(|(c, t)| c - t).collect();
        let mask: Vec<_> = (0..pixels)
            .map(|i| if i % 7 == 3 { 0 } else { 255 })
            .collect();
        let heat: Vec<_> = (0..pixels).map(|i| 0.5 + (i % 9) as f32 / 8.0).collect();
        let expected = lightness_cells(
            "cpu",
            height,
            width,
            [&candidate, &target, &delta],
            Some(&mask),
            Some(&heat),
            0.25,
        )
        .unwrap();
        let staged = lightness_deferred(
            "cpu",
            height,
            width,
            &lightness_plan(height, width),
            vec![[candidate, target, delta].concat()],
            Some(&mask),
        )
        .unwrap();
        let actual = staged.finish(Some(&heat), 0.25).unwrap();
        for (actual, expected) in actual.iter().flatten().zip(expected.iter().flatten()) {
            assert_eq!(actual[2], expected[2]);
            let total = |words: &[f32; 3]| f64::from(words[0]) + f64::from(words[1]);
            assert!((total(actual) - total(expected)).abs() <= 1e-6);
        }
    }

    #[test]
    fn compact_cells_keep_partial_edges_support_and_inspection() {
        let (height, width) = (11, 13);
        let area = height * width;
        let candidate: Vec<_> = (0..area).map(|i| ((i * 31) % 97) as f32).collect();
        let target: Vec<_> = (0..area).map(|i| ((i * 47) % 89) as f32).collect();
        let delta: Vec<_> = candidate.iter().zip(&target).map(|(c, t)| c - t).collect();
        let support: Vec<_> = (0..area)
            .map(|i| if i % 7 != 3 { 255 } else { 0 })
            .collect();
        let heat: Vec<_> = (0..area).map(|i| 0.5 + (i % 9) as f32 / 8.0).collect();
        let threshold = 0.25;
        let pixels = lightness(
            "cpu",
            height,
            width,
            &candidate,
            &target,
            &delta,
            Some(&support),
        )
        .unwrap();
        let cells = lightness_cells(
            "cpu",
            height,
            width,
            [&candidate, &target, &delta],
            Some(&support),
            Some(&heat),
            threshold,
        )
        .unwrap();
        for (samples, reduced) in pixels.iter().zip(cells) {
            let expected = (0..height)
                .step_by(4)
                .flat_map(|y| (0..width).step_by(4).map(move |x| (y, x)));
            for ((y, x), [high, low, count]) in expected.zip(reduced) {
                let mut sum = 0.;
                let mut affected = 0;
                for row in y..(y + 4).min(height) {
                    for col in x..(x + 4).min(width) {
                        let i = row * width + col;
                        if support[i] != 0 {
                            let weighted = samples[i] * heat[i];
                            sum += f64::from(weighted);
                            affected += usize::from(weighted >= threshold);
                        }
                    }
                }
                assert!((sum - (f64::from(high) + f64::from(low))).abs() <= 1e-7);
                assert_eq!(count as usize, affected);
            }
        }
    }
}
