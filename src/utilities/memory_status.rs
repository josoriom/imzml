#[derive(Clone, Copy)]
pub(crate) struct MemoryStatus {
    pub(crate) current_bytes: u64,
    pub(crate) peak_bytes: u64,
}

pub(crate) fn get_memory_status() -> Option<MemoryStatus> {
    let (current_bytes, peak_bytes) = read_resident_memory()?;
    Some(MemoryStatus {
        current_bytes,
        peak_bytes,
    })
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn read_resident_memory() -> Option<(u64, u64)> {
    unsafe {
        let mut data: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
        let result = libc::task_info(
            libc::mach_task_self_,
            libc::MACH_TASK_BASIC_INFO,
            &mut data as *mut _ as libc::task_info_t,
            &mut count,
        );
        if result != libc::KERN_SUCCESS {
            return None;
        }
        Some((data.resident_size as u64, data.resident_size_max as u64))
    }
}

#[cfg(target_os = "linux")]
fn read_resident_memory() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    let current_bytes = read_status_kb(&text, "VmRSS:")? * 1024;
    let peak_bytes = read_status_kb(&text, "VmHWM:").unwrap_or(current_bytes / 1024) * 1024;
    Some((current_bytes, peak_bytes))
}

#[cfg(target_os = "linux")]
fn read_status_kb(text: &str, key: &str) -> Option<u64> {
    text.lines()
        .find(|line| line.starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
}

#[cfg(target_os = "windows")]
fn read_resident_memory() -> Option<(u64, u64)> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) == 0 {
            return None;
        }
        Some((
            counters.WorkingSetSize as u64,
            counters.PeakWorkingSetSize as u64,
        ))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn read_resident_memory() -> Option<(u64, u64)> {
    None
}
