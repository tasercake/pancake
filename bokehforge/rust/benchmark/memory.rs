use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use pyo3::prelude::*;

const GIB: u64 = 1024 * 1024 * 1024;
pub(super) const MIN_PIXELS: usize = 1_000_000;
pub(super) const MAX_PIXELS: usize = 20_000_000;

fn mem_available(proc_root: &Path) -> Option<u64> {
    let meminfo = fs::read_to_string(proc_root.join("meminfo")).ok()?;
    let available = meminfo
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))?;
    let mut fields = available.split_whitespace();
    let kib: u64 = fields.next()?.parse().ok()?;
    (fields.next()? == "kB" && fields.next().is_none()).then(|| kib.checked_mul(1024))?
}

fn host_headroom(
    proc_root: &Path,
    cgroup_root: &Path,
    fallback: impl FnOnce() -> Option<u64>,
) -> Option<u64> {
    let mut free = mem_available(proc_root).or_else(fallback)?;
    let cgroup = fs::read_to_string(proc_root.join("self/cgroup")).ok()?;
    let mut lines = cgroup.lines();
    let relative = lines.next()?.strip_prefix("0::/")?;
    if lines.next().is_some() {
        return None;
    }
    let root = cgroup_root.canonicalize().ok()?;
    let group = root.join(relative).canonicalize().ok()?;
    if !group.starts_with(&root) || !group.is_dir() {
        return None;
    }
    for ancestor in group.ancestors().take_while(|path| path.starts_with(&root)) {
        let limit = match fs::read_to_string(ancestor.join("memory.max")) {
            Ok(limit) => limit,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(_) => return None,
        };
        if limit.trim() != "max" {
            let limit: u64 = limit.trim().parse().ok()?;
            let usage: u64 = fs::read_to_string(ancestor.join("memory.current"))
                .ok()?
                .trim()
                .parse()
                .ok()?;
            free = free.min(limit.saturating_sub(usage));
        }
    }
    Some(free)
}

#[cfg(target_os = "linux")]
fn available_pages() -> Option<u64> {
    // Both sysconf selectors return a count or a negative error value.
    let pages = u64::try_from(unsafe { libc::sysconf(libc::_SC_AVPHYS_PAGES) }).ok()?;
    let page_size = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).ok()?;
    pages.checked_mul(page_size)
}

#[cfg(target_os = "linux")]
pub(super) fn replay_fits(device: &str, target_pixels: usize) -> bool {
    if !matches!(device, "auto" | "gpu" | "cuda")
        || host_headroom(
            Path::new("/proc"),
            Path::new("/sys/fs/cgroup"),
            available_pages,
        )
        .is_none_or(|free| free < 8 * GIB)
        || crate::engine::select_backend(device).ok() != Some("cuda")
    {
        return false;
    }
    let free = cudarc::driver::CudaContext::new(0)
        .ok()
        .and_then(|context| context.mem_get_info().ok())
        .map(|(free, _)| free as u64);
    // Keep space for a cold Inspection blend and full-resolution CUDA maps.
    free.is_some_and(|free| free >= 6 * GIB + 128 * target_pixels as u64)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn replay_fits(_device: &str, _target_pixels: usize) -> bool {
    false
}

#[pyfunction]
pub(super) fn benchmark_host_replay_fits(
    py: Python<'_>,
    proc_root: std::path::PathBuf,
    cgroup_root: std::path::PathBuf,
    fallback_free: u64,
) -> bool {
    py.detach(|| {
        host_headroom(&proc_root, &cgroup_root, || Some(fallback_free))
            .is_some_and(|free| free >= 8 * GIB)
    })
}
