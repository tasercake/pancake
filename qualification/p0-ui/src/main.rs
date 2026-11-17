//! Safe Slint interpreter, shared GPU resources and a finite native worker proof.

#[expect(
    clippy::redundant_pub_crate,
    reason = "Worker helpers stay private to this binary; the suggested pub visibility violates Rust unreachable_pub"
)]
mod worker;

use pancake_ui_qualification::SlotCache;
use slint::{ComponentHandle, Timer, TimerMode};
use slint_interpreter::{Compiler, ComponentInstance, Value};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, atomic::Ordering, mpsc},
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt;
use worker::{Frame, Message, Shared};

/// Preview width after fixed spatial downsampling.
const PREVIEW_WIDTH: u32 = 512;
/// Preview height.
const PREVIEW_HEIGHT: u32 = 384;

/// One physical device/queue pair imported by compute and presentation.
#[derive(Debug)]
struct SharedGpu {
    /// Registered compute selector for the same handles.
    compute: cubecl::Device,
    /// Shared resource allocator.
    device: wgpu::Device,
    /// Shared ordered command stream.
    queue: wgpu::Queue,
    /// Actual adapter/backend identity for native observation.
    description: String,
}

/// Set a dynamic property, recording an actual schema failure if it is rejected.
fn set(component: &ComponentInstance, name: &str, value: impl AsRef<str>, shared: &Shared) {
    if component
        .set_property(name, Value::String(value.as_ref().into()))
        .is_err()
    {
        shared.state.store(worker::FAILED, Ordering::SeqCst);
    }
}

/// Import a bounded preview texture using exactly the same device/queue as Slint.
///
/// # Errors
///
/// Returns completion, texture-import or dynamic-property errors.
fn present(
    component: &ComponentInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut SlotCache<wgpu::Texture>,
    frame: &Frame,
) -> Result<(), Box<dyn std::error::Error>> {
    let _status = device.poll(wgpu::PollType::wait_indefinitely())?;
    cache.make_room();
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("P0 bounded shared preview"),
            size: wgpu::Extent3d {
                width: PREVIEW_WIDTH,
                height: PREVIEW_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &frame.rgba,
    );
    let _submission = queue.submit(std::iter::empty());
    component.set_property("picture", slint::Image::try_from(texture.clone())?.into())?;
    cache.insert(texture);
    Ok(())
}

/// Initialize one physical adapter and import its handles into both libraries.
///
/// # Errors
///
/// Returns physical-adapter, device, `CubeCL` registration or Slint backend errors.
fn shared_gpu() -> Result<SharedGpu, Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        cubecl::future::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..wgpu::RequestAdapterOptions::default()
        }))?;
    let info = adapter.get_info();
    if info.device_type == wgpu::DeviceType::Cpu {
        return Err("physical GPU required for UI interop qualification".into());
    }
    let (device, queue) =
        cubecl::future::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let setup = cubecl::wgpu::WgpuSetup {
        instance: instance.clone(),
        adapter: adapter.clone(),
        device: device.clone(),
        queue: queue.clone(),
        backend: adapter.get_info().backend,
    };
    let selected =
        cubecl::wgpu::try_init_device(setup, cubecl::wgpu::RuntimeOptions::try_default()?)?;
    slint::BackendSelector::new()
        .require_wgpu_30(slint::wgpu_30::WGPUConfiguration::Manual {
            instance,
            adapter,
            device: device.clone(),
            queue: queue.clone(),
        })
        .select()?;
    Ok(SharedGpu {
        compute: cubecl::Device::Wgpu(selected),
        device,
        queue,
        description: format!(
            "{} / {:?} / {:?}",
            info.name, info.backend, info.device_type
        ),
    })
}

/// Attach only genuine native control callbacks to the finite worker.
///
/// # Errors
///
/// Returns an error if the pinned component callback schema differs.
fn install_callbacks(
    component: &ComponentInstance,
    shared: &Arc<Shared>,
    sender: &mpsc::SyncSender<Message>,
) -> Result<(), slint_interpreter::SetCallbackError> {
    let pattern = Rc::new(Cell::new(false));
    let clicks = Rc::new(Cell::new(0_u32));
    let weak = component.as_weak();
    let toggle_shared = Arc::clone(shared);
    let toggle_sender = sender.clone();
    let toggle_pattern = Rc::clone(&pattern);
    component.set_callback("toggle-pattern", move |_| {
        let inverted = !toggle_pattern.get();
        if worker::request(&toggle_sender, &toggle_shared, inverted, 1) {
            toggle_pattern.set(inverted);
            if let Some(component) = weak.upgrade() {
                set(
                    &component,
                    "pattern-state",
                    if inverted { "Pattern B" } else { "Pattern A" },
                    &toggle_shared,
                );
                set(&component, "worker-state", "Queued", &toggle_shared);
            }
        }
        Value::Void
    })?;
    let weak = component.as_weak();
    let start_shared = Arc::clone(shared);
    let start_sender = sender.clone();
    component.set_callback("start-job", move |_| {
        if worker::request(&start_sender, &start_shared, pattern.get(), 128)
            && let Some(component) = weak.upgrade()
        {
            set(&component, "worker-state", "Queued", &start_shared);
        }
        Value::Void
    })?;
    let cancel_shared = Arc::clone(shared);
    component.set_callback("cancel-job", move |_| {
        cancel_shared.cancel.store(true, Ordering::SeqCst);
        Value::Void
    })?;
    let weak = component.as_weak();
    let click_shared = Arc::clone(shared);
    component.set_callback("ping-ui", move |_| {
        clicks.set(clicks.get().saturating_add(1));
        if let Some(component) = weak.upgrade() {
            set(
                &component,
                "ui-count",
                clicks.get().to_string(),
                &click_shared,
            );
        }
        Value::Void
    })?;

    Ok(())
}

/// Run one real window and drain/cancel its finite worker before normal shutdown.
///
/// # Errors
///
/// Returns compilation, platform, callback, thread or native event-loop errors.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let SharedGpu {
        compute,
        device,
        queue,
        description,
    } = shared_gpu()?;
    let started = Instant::now();
    let mut compiler = Compiler::new();
    compiler.set_edition((1, 19));
    // Only repository-owned, embedded UI is accepted. There is no user source,
    // path argument, hot reload or custom import callback at this boundary.
    let result = cubecl::future::block_on(compiler.build_from_source(
        include_str!("qualification.slint").to_owned(),
        "qualification.slint".into(),
    ));
    let definition = result.component("QualificationWindow").ok_or_else(|| {
        format!(
            "UI compilation failed: {:?}",
            result.diagnostics().collect::<Vec<_>>()
        )
    })?;
    let component = definition.create()?;
    component.set_property("device-state", Value::String(description.into()))?;
    let compilation_ms = started.elapsed().as_millis();
    let shared = Arc::new(Shared::default());
    let close_shared = Arc::clone(&shared);
    component.window().on_close_requested(move || {
        close_shared.cancel.store(true, Ordering::SeqCst);
        if slint::quit_event_loop().is_err() {
            close_shared.state.store(worker::FAILED, Ordering::SeqCst);
        }
        slint::CloseRequestResponse::HideWindow
    });
    let (sender, commands) = mpsc::sync_channel::<Message>(1);
    let (previews, receiver) = mpsc::sync_channel::<Frame>(1);
    let thread_shared = Arc::clone(&shared);
    let worker = std::thread::Builder::new()
        .name("p0-image-worker".to_owned())
        .spawn(move || worker::run(&compute, &thread_shared, &commands, &previews))?;
    let cache = Rc::new(RefCell::new(SlotCache::new()));
    component.set_property("budget-state", Value::String(format!(
        "compile={compilation_ms}ms; cmd=1; preview=1; u16 planes=2 (12582912B); RGBA/frame=786432B; textures<=2"
    ).into()))?;

    let render_shared = Arc::clone(&shared);
    let render_device = device.clone();
    let render_queue = queue.clone();
    component
        .window()
        .set_rendering_notifier(move |state, api| {
            if matches!(state, slint::RenderingState::RenderingSetup)
                && let slint::GraphicsAPI::WGPU30 { device, queue, .. } = api
            {
                render_shared.interop.store(
                    device == &render_device && queue == &render_queue,
                    Ordering::SeqCst,
                );
            }
        })?;

    install_callbacks(&component, &shared, &sender)?;

    let timer = Timer::default();
    let weak = component.as_weak();
    let poll_shared = Arc::clone(&shared);
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        let Some(component) = weak.upgrade() else { return; };
        if let Ok(frame) = receiver.try_recv()
            && frame.job == poll_shared.job.load(Ordering::SeqCst)
            && present(&component, &device, &queue, &mut cache.borrow_mut(), &frame).is_err() {
            poll_shared.state.store(worker::FAILED, Ordering::SeqCst);
        }
        let state = match poll_shared.state.load(Ordering::SeqCst) {
            worker::QUEUED => "Queued", worker::RUNNING => "Running", worker::READY => "Ready",
            worker::CANCELLED => "Cancelled", worker::COMPLETE => "Complete", worker::FAILED => "Failed", _ => "Idle",
        };
        set(&component, "worker-state", state, &poll_shared);
        set(&component, "completed-batches", poll_shared.completed.load(Ordering::SeqCst).to_string(), &poll_shared);
        set(&component, "submitted-batches", poll_shared.submitted.load(Ordering::SeqCst).to_string(), &poll_shared);
        set(&component, "cache-peak", cache.borrow().peak().to_string(), &poll_shared);
        set(&component, "interop-state", if poll_shared.interop.load(Ordering::SeqCst) { "Shared device + queue confirmed" } else { "Pending shared identity" }, &poll_shared);
        set(&component, "budget-state", format!(
            "compile_ms={compilation_ms};cmd_slots=1;preview_slots=1;u16_live={};u16_peak={};u16_bytes={};rgba_live={};rgba_peak={};rgba_bytes={};texture_peak={};texture_bytes={};root_u32_staging_bound=25165824",
            poll_shared.planes_live.load(Ordering::SeqCst),
            poll_shared.planes_peak.load(Ordering::SeqCst),
            6_291_456_u64.saturating_mul(u64::from(poll_shared.planes_peak.load(Ordering::SeqCst))),
            poll_shared.rgba_live.load(Ordering::SeqCst),
            poll_shared.rgba_peak.load(Ordering::SeqCst),
            786_432_u64.saturating_mul(u64::from(poll_shared.rgba_peak.load(Ordering::SeqCst))),
            cache.borrow().peak(), 786_432_usize.saturating_mul(cache.borrow().peak()),
        ), &poll_shared);
    });
    let _accepted = worker::request(&sender, &shared, false, 1);
    let outcome = component.run();
    shared.cancel.store(true, Ordering::SeqCst);
    let shutdown = sender.send(Message::Shutdown);
    drop(timer);
    drop(component);
    drop(sender);
    worker
        .join()
        .map_err(|_| "image worker terminated unexpectedly")?;
    shutdown?;
    outcome?;
    Ok(())
}
