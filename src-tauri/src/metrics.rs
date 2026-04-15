use sysinfo::{Pid, ProcessesToUpdate, System};

pub fn get_resource_usage_cross_platform(root_pid: u32) -> (f32, u64) {
    let mut sys = System::new_all();
    let _ = sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut included = std::collections::HashSet::new();
    included.insert(Pid::from_u32(root_pid));

    let mut changed = true;
    while changed {
        changed = false;
        for (pid, process) in sys.processes() {
            if let Some(parent) = process.parent() {
                if included.contains(&parent) && !included.contains(pid) {
                    included.insert(*pid);
                    changed = true;
                }
            }
        }
    }

    let mut total_cpu = 0.0_f32;
    let mut total_mem_bytes = 0_u64;

    for pid in included {
        if let Some(process) = sys.process(pid) {
            total_cpu += process.cpu_usage();
            total_mem_bytes = total_mem_bytes.saturating_add(process.memory());
        }
    }

    (total_cpu, total_mem_bytes / (1024 * 1024))
}
