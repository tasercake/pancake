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

pub(crate) fn available(device: &str) -> bool {
    availability(device).is_ok()
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
    let planes = [candidate, target, delta];
    let area = height
        .checked_mul(width)
        .filter(|&size| size != 0 && planes.iter().all(|plane| plane.len() == size))
        .ok_or("accelerator lightness planes have invalid dimensions")?;
    if mask.is_some_and(|values| values.len() != area) {
        return Err("accelerator lightness mask has invalid dimensions".to_owned());
    }
    let image = Image {
        height,
        width,
        planes,
        mask,
    };

    catch_unwind(AssertUnwindSafe(|| {
        execute(&device::client(device)?, image)
    }))
    .map_err(|failure| {
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

#[derive(Clone, Copy)]
struct Window {
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,
}

impl Window {
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

fn execute(client: &Client, image: Image<'_>) -> Result<[Vec<f32>; 4], String> {
    let Image { height, width, .. } = image;
    let area = width * height;
    let weights = weights()
        .each_ref()
        .map(|values| Buffer::upload(client, values.clone()));

    // Bound every tile including its halo. A narrow, short image can use its
    // full height to leave more space for columns, and vice versa.
    let (tile_height, tile_width) = if area <= TILE_PIXELS {
        (height, width)
    } else {
        let min_rows_with_halo = height.min(2 * HALO + 1);
        let tile_width = width.min((TILE_PIXELS / min_rows_with_halo).saturating_sub(2 * HALO));
        let padded_width = (tile_width + 2 * HALO).min(width);
        let max_rows = TILE_PIXELS / padded_width;
        let tile_height = if max_rows >= height {
            height
        } else {
            max_rows.saturating_sub(2 * HALO).max(1)
        };
        (tile_height, tile_width)
    };

    let mut output: [Vec<f32>; 4] = std::array::from_fn(|_| vec![0.; area]);
    for top in (0..height).step_by(tile_height) {
        for left in (0..width).step_by(tile_width) {
            let core = Window {
                top,
                left,
                bottom: top.saturating_add(tile_height).min(height),
                right: left.saturating_add(tile_width).min(width),
            };
            let tile = core.halo(height, width);
            let maps = execute_tile(client, image, tile, &weights)?;
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
    }
    Ok(output)
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
    fn dispatch(&self, count: usize) -> (CubeCount, CubeDim) {
        let dim = CubeDim::new(self.client, 128);
        (
            cubecl::calculate_cube_count_elemwise(self.client, count, dim),
            dim,
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
        for (input, output, first_plane, pass_y) in [
            (source, vertical, first, true),
            (vertical, horizontal, 0, false),
        ] {
            let (count, dim) = self.dispatch(self.area * planes);
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
            );
        }
        if self.masked {
            let (count, dim) = self.dispatch(self.area * (planes - 1));
            kernels::normalize::launch(
                self.client,
                count,
                dim,
                horizontal.arg(),
                self.area,
                planes - 1,
            );
        }
    }

    fn contrast(&self, samples: &Buffer, original: &Buffer, output: &Buffer, initial: bool) {
        let (count, dim) = self.dispatch(self.area);
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
        );
    }

    fn copy_residual(&self, source: &Buffer, residuals: &Buffer, plane: usize, destination: usize) {
        let (count, dim) = self.dispatch(self.area);
        kernels::copy_plane::launch(
            self.client,
            count,
            dim,
            source.arg(),
            residuals.arg(),
            self.area,
            plane,
            destination,
        );
    }
}

fn execute_tile(
    client: &Client,
    image: Image<'_>,
    tile: Window,
    weights: &[Buffer; 4],
) -> Result<Bytes, String> {
    let Image {
        width: image_width,
        planes,
        mask,
        ..
    } = image;
    let (height, width) = (tile.height(), tile.width());
    let area = height * width;
    let masked = mask.is_some();
    let plane_count = if masked { 4 } else { 3 };
    let job = Job {
        client,
        height,
        width,
        area,
        masked,
    };
    let original = {
        let mut samples = Vec::with_capacity(area * plane_count);
        for plane in planes {
            for row in tile.rows(image_width) {
                samples.extend_from_slice(&plane[row]);
            }
        }
        if let Some(mask) = mask {
            for row in tile.rows(image_width) {
                samples.extend(
                    mask[row]
                        .iter()
                        .map(|&value| f32::from(u8::from(value != 0))),
                );
            }
        }
        Buffer::upload(client, samples)
    };
    let masked_samples;
    let filter_source = if masked {
        masked_samples = Buffer::empty(client, area * plane_count);
        let (count, dim) = job.dispatch(area * plane_count);
        kernels::mask_samples::launch(
            client,
            count,
            dim,
            original.arg(),
            masked_samples.arg(),
            area,
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
    let (count, dim) = job.dispatch(area);
    kernels::extrema::launch(
        client,
        count,
        dim,
        original.arg(),
        edges.arg(),
        width,
        height,
    );
    let (count, dim) = job.dispatch(area);
    kernels::nearby::launch(
        client,
        count,
        dim,
        edges.arg(),
        vertical.arg(),
        width,
        height,
        false,
    );
    let (count, dim) = job.dispatch(area);
    kernels::nearby::launch(
        client,
        count,
        dim,
        vertical.arg(),
        edges.arg(),
        width,
        height,
        true,
    );
    let (count, dim) = job.dispatch(area);
    kernels::finish::launch(
        client,
        count,
        dim,
        original.arg(),
        residuals.arg(),
        edges.arg(),
        output.arg(),
        area,
    );
    output.read(client)
}
