//! The debug overlay's numbers: how much CPU, GPU and memory Lumen itself is
//! using (mpv included, since it runs inside the process). A thread samples
//! them once a second, and only while the overlay is switched on.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use eframe::egui;
use parking_lot::Mutex;

#[derive(Clone, Copy, Debug, Default)]
pub struct Usage {
    /// Percent of one core, like Activity Monitor and top (so it can pass 100).
    pub cpu: f32,
    /// Percent of the GPU's time; None where the OS won't say.
    pub gpu: Option<f32>,
    pub mem_bytes: u64,
}

#[derive(Default)]
pub struct Monitor {
    pub usage: Arc<Mutex<Option<Usage>>>,
    running: Option<Arc<AtomicBool>>,
}

impl Monitor {
    /// Start or stop the sampling thread to match the setting.
    pub fn set_enabled(&mut self, on: bool, ctx: &egui::Context) {
        match (&self.running, on) {
            (None, true) => {
                let flag = Arc::new(AtomicBool::new(true));
                let (f, usage, ctx) = (flag.clone(), self.usage.clone(), ctx.clone());
                std::thread::Builder::new()
                    .name("perf".into())
                    .spawn(move || sample_loop(&f, &usage, &ctx))
                    .ok();
                self.running = Some(flag);
            }
            (Some(flag), false) => {
                flag.store(false, Ordering::Relaxed);
                self.running = None;
                *self.usage.lock() = None;
            }
            _ => {}
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        if let Some(f) = &self.running {
            f.store(false, Ordering::Relaxed);
        }
    }
}

fn sample_loop(running: &AtomicBool, usage: &Mutex<Option<Usage>>, ctx: &egui::Context) {
    let mut gpu = gpu::Gpu::new();
    let mut last = (Instant::now(), cpu_time());
    while running.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_secs(1));
        let now = (Instant::now(), cpu_time());
        let cpu = percent(now.1.saturating_sub(last.1), now.0 - last.0);
        *usage.lock() = Some(Usage { cpu, gpu: gpu.percent(), mem_bytes: memory() });
        last = now;
        ctx.request_repaint();
    }
}

fn percent(busy: Duration, wall: Duration) -> f32 {
    (busy.as_secs_f64() / wall.as_secs_f64().max(1e-3) * 100.0) as f32
}

/// CPU time used by the whole process, user and system.
#[cfg(unix)]
fn cpu_time() -> Duration {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    // Safety: getrusage only writes into the struct it's given.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) } != 0 {
        return Duration::ZERO;
    }
    let tv = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    tv(ru.ru_utime) + tv(ru.ru_stime)
}

#[cfg(windows)]
fn cpu_time() -> Duration {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let z = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut a, mut b, mut kernel, mut user) = (z, z, z, z);
    // Safety: the pseudo-handle is always valid; the call fills the four structs.
    if unsafe { GetProcessTimes(GetCurrentProcess(), &mut a, &mut b, &mut kernel, &mut user) } == 0 {
        return Duration::ZERO;
    }
    let ticks = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    Duration::from_nanos((ticks(kernel) + ticks(user)) * 100)
}

/// The memory figure Activity Monitor shows (the process's footprint).
#[cfg(target_os = "macos")]
fn memory() -> u64 {
    let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    // Safety: the buffer is the v4 struct the flavour asks for.
    let r = unsafe { libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V4, (&raw mut info).cast()) };
    if r == 0 { info.ri_phys_footprint } else { 0 }
}

/// Resident memory, from /proc.
#[cfg(all(unix, not(target_os = "macos")))]
fn memory() -> u64 {
    let pages = std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
        .unwrap_or(0);
    // Safety: sysconf has no preconditions.
    pages * unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(0) as u64
}

/// Private bytes, the figure Task Manager shows as Memory.
#[cfg(windows)]
fn memory() -> u64 {
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut c: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    // Safety: cb tells the call how big the struct is.
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), (&raw mut c).cast::<PROCESS_MEMORY_COUNTERS>(), c.cb) };
    if ok != 0 { c.PrivateUsage as u64 } else { 0 }
}

/// How busy Lumen keeps the GPU, per OS. `percent` is the share of the
/// time since the last call.
#[cfg(target_os = "macos")]
mod gpu {
    //! Each process's GPU clients sit under the IOAccelerator in the IOKit
    //! registry, with an `AppUsage` list of accumulated GPU time in
    //! nanoseconds. It's what Activity Monitor's GPU column is made from.

    use std::ffi::{c_char, c_void};
    use std::time::{Duration, Instant};

    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const c_char) -> *mut c_void;
        fn IOServiceGetMatchingServices(main_port: u32, matching: *mut c_void, iter: *mut u32) -> i32;
        fn IOIteratorNext(iter: u32) -> u32;
        fn IORegistryEntryGetChildIterator(entry: u32, plane: *const c_char, iter: *mut u32) -> i32;
        fn IORegistryEntryCreateCFProperty(entry: u32, key: *const c_void, alloc: *const c_void, options: u32) -> *const c_void;
        fn IOObjectRelease(object: u32) -> i32;
    }

    pub struct Gpu {
        prefix: String,
        last: Option<(Instant, u64)>,
    }

    impl Gpu {
        pub fn new() -> Gpu {
            let mut g = Gpu { prefix: format!("pid {},", std::process::id()), last: None };
            g.percent();
            g
        }

        pub fn percent(&mut self) -> Option<f32> {
            let now = (Instant::now(), self.busy_ns()?);
            let prev = self.last.replace(now)?;
            // A client that went away takes its total with it, hence saturating.
            Some(super::percent(Duration::from_nanos(now.1.saturating_sub(prev.1)), now.0 - prev.0).min(100.0))
        }

        fn busy_ns(&self) -> Option<u64> {
            let mut total = 0u64;
            let mut found = false;
            // Safety: plain IOKit registry walking; every object handed out is released.
            unsafe {
                let mut accels = 0;
                if IOServiceGetMatchingServices(0, IOServiceMatching(c"IOAccelerator".as_ptr()), &mut accels) != 0 {
                    return None;
                }
                loop {
                    let accel = IOIteratorNext(accels);
                    if accel == 0 {
                        break;
                    }
                    let mut clients = 0;
                    if IORegistryEntryGetChildIterator(accel, c"IOService".as_ptr(), &mut clients) == 0 {
                        loop {
                            let client = IOIteratorNext(clients);
                            if client == 0 {
                                break;
                            }
                            if property(client, "IOUserClientCreator")
                                .and_then(|v| v.downcast::<CFString>())
                                .is_some_and(|s| s.to_string().starts_with(&self.prefix))
                            {
                                found = true;
                                total += app_usage(client);
                            }
                            IOObjectRelease(client);
                        }
                        IOObjectRelease(clients);
                    }
                    IOObjectRelease(accel);
                }
                IOObjectRelease(accels);
            }
            found.then_some(total)
        }
    }

    unsafe fn property(entry: u32, key: &str) -> Option<CFType> {
        let key = CFString::new(key);
        // Safety: the property comes back retained (a Create call), so wrap_under_create_rule owns it.
        let v = unsafe { IORegistryEntryCreateCFProperty(entry, key.as_concrete_TypeRef().cast(), std::ptr::null(), 0) };
        (!v.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(v) })
    }

    unsafe fn app_usage(client: u32) -> u64 {
        let Some(list) = (unsafe { property(client, "AppUsage") }).and_then(|v| v.downcast::<CFArray>()) else {
            return 0;
        };
        let key = CFString::new("accumulatedGPUTime");
        list.iter()
            // Safety: the array holds CF objects and keeps them alive while we look.
            .filter_map(|e| unsafe { CFType::wrap_under_get_rule(*e) }.downcast::<CFDictionary>())
            .filter_map(|d| {
                let v = d.find(key.as_concrete_TypeRef().cast())?;
                // Safety: the dictionary holds CF objects; find doesn't transfer ownership.
                let v = unsafe { CFType::wrap_under_get_rule(*v) };
                v.downcast::<CFNumber>()?.to_i64()
            })
            .map(|n| n.max(0) as u64)
            .sum()
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod gpu {
    //! The kernel's DRM drivers (amdgpu, i915, xe, nouveau…) list per-client
    //! busy time for each engine in /proc/self/fdinfo. Lumen's figure is its
    //! busiest engine, which is how most GPU tools show it.

    use std::collections::{HashMap, HashSet};
    use std::time::{Duration, Instant};

    pub struct Gpu {
        last: Option<(Instant, HashMap<String, u64>)>,
    }

    impl Gpu {
        pub fn new() -> Gpu {
            let mut g = Gpu { last: None };
            g.percent();
            g
        }

        pub fn percent(&mut self) -> Option<f32> {
            let now = (Instant::now(), engines()?);
            let (then, prev) = self.last.replace(now.clone())?;
            let busiest = now.1.iter().map(|(k, ns)| ns.saturating_sub(*prev.get(k).unwrap_or(ns))).max()?;
            Some(super::percent(Duration::from_nanos(busiest), now.0 - then).min(100.0))
        }
    }

    /// Busy time per engine, in ns.
    fn engines() -> Option<HashMap<String, u64>> {
        // Several fds can share one client, so count each client once.
        let mut seen = HashSet::new();
        let mut engines: HashMap<String, u64> = HashMap::new();
        for e in std::fs::read_dir("/proc/self/fdinfo").ok()?.flatten() {
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            let mut client = None;
            let mut busy = Vec::new();
            for line in text.lines() {
                let Some((k, v)) = line.split_once(':') else { continue };
                let v = v.trim();
                if k == "drm-client-id" {
                    client = Some(v.to_owned());
                } else if let Some(name) = k.strip_prefix("drm-engine-")
                    && let Some(ns) = v.strip_suffix(" ns").and_then(|n| n.parse::<u64>().ok())
                {
                    busy.push((name.to_owned(), ns));
                }
            }
            if let Some(c) = client
                && seen.insert(c)
            {
                for (name, ns) in busy {
                    *engines.entry(name).or_default() += ns;
                }
            }
        }
        (!engines.is_empty()).then_some(engines)
    }
}

#[cfg(windows)]
mod gpu {
    //! Windows keeps a "GPU Engine" performance counter per process and
    //! engine. Task Manager's GPU column is the busiest engine type, summed
    //! over that type's engines, and so is this.

    use std::collections::HashMap;

    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };

    pub struct Gpu {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
        prefix: String,
    }

    impl Gpu {
        pub fn new() -> Gpu {
            let mut g = Gpu { query: std::ptr::null_mut(), counter: std::ptr::null_mut(), prefix: format!("pid_{}_", std::process::id()) };
            let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage".encode_utf16().chain([0]).collect();
            // Safety: PDH fills in the handles; the path is NUL-terminated UTF-16.
            unsafe {
                if PdhOpenQueryW(std::ptr::null(), 0, &mut g.query) != 0
                    || PdhAddEnglishCounterW(g.query, path.as_ptr(), 0, &mut g.counter) != 0
                {
                    g.counter = std::ptr::null_mut();
                }
                PdhCollectQueryData(g.query);
            }
            g
        }

        pub fn percent(&mut self) -> Option<f32> {
            if self.counter.is_null() {
                return None;
            }
            // Safety: the buffer is sized by PDH's first call and holds `count` items.
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let (mut size, mut count) = (0u32, 0u32);
                if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut()) != PDH_MORE_DATA {
                    return None;
                }
                let mut buf = vec![0u64; (size as usize).div_ceil(8)];
                let items = buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
                if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                    return None;
                }
                let mut by_type: HashMap<String, f64> = HashMap::new();
                for item in std::slice::from_raw_parts(items, count as usize) {
                    let mut len = 0;
                    while *item.szName.add(len) != 0 {
                        len += 1;
                    }
                    let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.szName, len));
                    if !name.starts_with(&self.prefix) {
                        continue;
                    }
                    let kind = name.rsplit_once("engtype_").map(|(_, t)| t.to_owned()).unwrap_or_default();
                    *by_type.entry(kind).or_default() += item.FmtValue.Anonymous.doubleValue;
                }
                Some(by_type.values().copied().fold(0.0, f64::max).min(100.0) as f32)
            }
        }
    }

    impl Drop for Gpu {
        fn drop(&mut self) {
            // Safety: the query was opened in new().
            unsafe { PdhCloseQuery(self.query) };
        }
    }
}

pub fn format_bytes(b: u64) -> String {
    let mb = b as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 { format!("{:.2} GB", mb / 1024.0) } else { format!("{mb:.0} MB") }
}
