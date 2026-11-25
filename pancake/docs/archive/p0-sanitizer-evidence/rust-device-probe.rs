//! Diagnostic only: exact Vulkan selection and a minimal safe wgpu request.
use std::{future::Future, sync::Arc, task::{Context, Poll, Wake, Waker}, thread};
use pancake::wgpu;
struct Notify(thread::Thread);
impl Wake for Notify { fn wake(self: Arc<Self>) { self.0.unpark(); } }
fn wait<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Notify(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop { match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => thread::park(),
    } }
}
fn main() -> Result<(), String> {
    if std::env::args().nth(1).as_deref() == Some("wgpu") {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = wait(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|e| e.to_string())?;
        println!("minimal_wgpu_adapter={:?}", adapter.get_info());
        let (device, queue) = wait(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| e.to_string())?;
        drop(queue); drop(device); drop(adapter); drop(instance);
        println!("minimal_wgpu_device=ok");
    } else {
        let selector = cubecl::wgpu::WgpuDevice::default().on(cubecl::wgpu::WgpuBackend::Vulkan);
        let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|e| e.to_string())?;
        let setup = cubecl::wgpu::try_init_setup::<cubecl::wgpu::Vulkan>(&selector, options)
            .map_err(|e| e.to_string())?;
        println!("pinned_cubecl_adapter={:?}", setup.adapter.get_info());
        let result = pancake::blur_horizontal_u16(&cubecl::Device::Wgpu(selector), 3, 1, &[0,4,8])?;
        if result != [1,4,7] { return Err(format!("kernel mismatch {result:?}")); }
        println!("pinned_cubecl_kernel=ok");
    }
    Ok(())
}
