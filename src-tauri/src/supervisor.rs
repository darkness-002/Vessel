use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerStatus {
    Running,
    Idle,
    Hibernated,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerProcessMetrics {
    pub id: String,
    pub name: String,
    pub url: String,
    pub status: ContainerStatus,
    pub pid: Option<u32>,
    pub memory_mb: f64,
    pub cpu_usage: f32,
    pub last_active_at: String,
    pub idle_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ManagedContainer {
    pub id: String,
    pub name: String,
    pub current_url: String,
    pub status: ContainerStatus,
    pub last_focused_at: Instant,
    pub pid: Option<u32>,
    pub is_focused: bool,
}

#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub idle_timeout: Duration,
    pub check_interval: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            // Default: 15 minutes of inactivity triggers hibernation
            idle_timeout: Duration::from_secs(15 * 60),
            check_interval: Duration::from_secs(5),
        }
    }
}

pub struct ProcessSupervisor {
    containers: HashMap<String, ManagedContainer>,
    system: sysinfo::System,
    config: SupervisorConfig,
}

pub type SupervisorState = Arc<Mutex<ProcessSupervisor>>;

#[cfg(target_os = "windows")]
pub fn extract_window_pid(win: &WebviewWindow) -> Option<u32> {
    if let Ok(hwnd) = win.hwnd() {
        extern "system" {
            fn GetWindowThreadProcessId(
                hwnd: *mut std::ffi::c_void,
                lpdw_process_id: *mut u32,
            ) -> u32;
        }
        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd.0, &mut pid);
        }
        if pid != 0 {
            return Some(pid);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn extract_window_pid(_win: &WebviewWindow) -> Option<u32> {
    None
}

impl ProcessSupervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        let mut system = sysinfo::System::new();
        system.refresh_all();
        Self {
            containers: HashMap::new(),
            system,
            config,
        }
    }

    pub fn set_idle_timeout(&mut self, timeout: Duration) {
        self.config.idle_timeout = timeout;
    }

    pub fn idle_timeout(&self) -> Duration {
        self.config.idle_timeout
    }

    pub fn check_interval(&self) -> Duration {
        self.config.check_interval
    }

    pub fn register_container(&mut self, id: &str, name: &str, url: &str) {
        if !self.containers.contains_key(id) {
            self.containers.insert(
                id.to_string(),
                ManagedContainer {
                    id: id.to_string(),
                    name: name.to_string(),
                    current_url: url.to_string(),
                    status: ContainerStatus::Closed,
                    last_focused_at: Instant::now(),
                    pid: None,
                    is_focused: false,
                },
            );
        }
    }

    pub fn remove_container(&mut self, id: &str) {
        self.containers.remove(id);
    }

    pub fn register_or_activate(&mut self, id: &str, name: &str, url: &str, pid: Option<u32>) {
        if let Some(entry) = self.containers.get_mut(id) {
            entry.status = ContainerStatus::Running;
            entry.last_focused_at = Instant::now();
            entry.is_focused = true;
            if !url.is_empty() {
                entry.current_url = url.to_string();
            }
            if let Some(p) = pid {
                entry.pid = Some(p);
            }
        } else {
            self.containers.insert(
                id.to_string(),
                ManagedContainer {
                    id: id.to_string(),
                    name: name.to_string(),
                    current_url: url.to_string(),
                    status: ContainerStatus::Running,
                    last_focused_at: Instant::now(),
                    pid,
                    is_focused: true,
                },
            );
        }
    }

    pub fn update_focus(&mut self, id: &str, focused: bool) {
        if let Some(entry) = self.containers.get_mut(id) {
            entry.is_focused = focused;
            entry.last_focused_at = Instant::now();
            if focused {
                entry.status = ContainerStatus::Running;
            } else if entry.status == ContainerStatus::Running {
                entry.status = ContainerStatus::Idle;
            }
        }
    }

    pub fn update_url(&mut self, id: &str, url: &str) {
        if let Some(entry) = self.containers.get_mut(id) {
            entry.current_url = url.to_string();
            entry.last_focused_at = Instant::now();
        }
    }

    pub fn mark_closed(&mut self, id: &str) {
        if let Some(entry) = self.containers.get_mut(id) {
            entry.status = ContainerStatus::Closed;
            entry.is_focused = false;
            entry.pid = None;
        }
    }

    pub fn mark_hibernated(&mut self, id: &str) {
        if let Some(entry) = self.containers.get_mut(id) {
            entry.status = ContainerStatus::Hibernated;
            entry.is_focused = false;
            entry.pid = None;
        }
    }

    pub fn get_container(&self, id: &str) -> Option<ManagedContainer> {
        self.containers.get(id).cloned()
    }

    pub fn get_all_managed(&self) -> Vec<ManagedContainer> {
        self.containers.values().cloned().collect()
    }

    pub fn should_hibernate(
        &self,
        id: &str,
        now: Instant,
        never_hibernate: bool,
        per_container_timeout: Option<Duration>,
    ) -> bool {
        if let Some(c) = self.containers.get(id) {
            if c.status != ContainerStatus::Running && c.status != ContainerStatus::Idle {
                return false;
            }
            if c.is_focused {
                return false;
            }
            if never_hibernate {
                return false;
            }
            let timeout = per_container_timeout.unwrap_or(self.config.idle_timeout);
            now.duration_since(c.last_focused_at) >= timeout
        } else {
            false
        }
    }

    pub fn get_metrics(&mut self) -> Vec<ContainerProcessMetrics> {
        self.system
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.system.refresh_cpu_all();

        let my_pid = sysinfo::Pid::from_u32(std::process::id());
        let child_processes: Vec<&sysinfo::Process> = self
            .system
            .processes()
            .values()
            .filter(|p| p.parent() == Some(my_pid))
            .collect();

        let active_count = self
            .containers
            .values()
            .filter(|c| c.status == ContainerStatus::Running || c.status == ContainerStatus::Idle)
            .count();

        let total_child_mem: u64 = child_processes.iter().map(|p| p.memory()).sum();
        let total_child_cpu: f32 = child_processes.iter().map(|p| p.cpu_usage()).sum();

        let mut metrics = Vec::new();
        let now = Instant::now();

        for entry in self.containers.values_mut() {
            let idle_seconds = now.duration_since(entry.last_focused_at).as_secs();

            if entry.status == ContainerStatus::Running && !entry.is_focused && idle_seconds > 5 {
                entry.status = ContainerStatus::Idle;
            }

            let mut mem_mb = 0.0;
            let mut cpu = 0.0;

            if entry.status == ContainerStatus::Running || entry.status == ContainerStatus::Idle {
                // If specific PID is known, use it directly
                if let Some(target_pid) = entry.pid {
                    let spid = sysinfo::Pid::from_u32(target_pid);
                    if let Some(proc) = self.system.process(spid) {
                        mem_mb = (proc.memory() as f64) / (1024.0 * 1024.0);
                        cpu = proc.cpu_usage();
                    }
                }

                // Fallback to webview subprocess allocation
                if mem_mb == 0.0 && active_count > 0 {
                    mem_mb = (total_child_mem as f64 / active_count as f64) / (1024.0 * 1024.0);
                    cpu = total_child_cpu / active_count as f32;
                }
            }

            let last_active_str = chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::seconds(idle_seconds as i64))
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339();

            metrics.push(ContainerProcessMetrics {
                id: entry.id.clone(),
                name: entry.name.clone(),
                url: entry.current_url.clone(),
                status: entry.status,
                pid: entry.pid,
                memory_mb: (mem_mb * 10.0).round() / 10.0,
                cpu_usage: (cpu * 10.0).round() / 10.0,
                last_active_at: last_active_str,
                idle_seconds,
            });
        }

        metrics.sort_by(|a, b| a.name.cmp(&b.name));
        metrics
    }
}

pub fn hibernate_container(
    app: &AppHandle,
    supervisor: &SupervisorState,
    id: &str,
) -> Result<String, String> {
    let win_label = format!("container-{}", id);

    let mut last_url = None;
    if let Some(win) = app.get_webview_window(&win_label) {
        if let Ok(url) = win.url() {
            last_url = Some(url.to_string());
        }
        // Destroy the heavyweight webview window to reclaim working set memory
        let _ = win.destroy();
    }

    let mut lock = supervisor.lock().map_err(|e| e.to_string())?;
    if let Some(url) = last_url {
        lock.update_url(id, &url);
    }
    lock.mark_hibernated(id);

    let container_name = lock
        .get_container(id)
        .map(|c| c.name)
        .unwrap_or_else(|| id.to_string());
    println!(
        "[Supervisor] Hibernated container '{}' to reclaim working set memory",
        id
    );
    Ok(format!(
        "Container '{}' has been hibernated",
        container_name
    ))
}

pub fn start_idle_monitor(app: AppHandle, supervisor: SupervisorState) {
    tauri::async_runtime::spawn(async move {
        loop {
            let check_interval = {
                if let Ok(lock) = supervisor.lock() {
                    lock.check_interval()
                } else {
                    Duration::from_secs(5)
                }
            };

            tokio::time::sleep(check_interval).await;

            // Re-read settings and container policy overrides each tick
            let settings = crate::profile::load_settings();
            let global_timeout =
                Duration::from_secs(settings.global_idle_threshold_minutes.max(1) * 60);
            let containers = crate::profile::load_containers();
            let mut container_policy_map = HashMap::new();
            for c in containers {
                let never = c.never_hibernate.unwrap_or(false);
                let custom_timeout = c
                    .idle_threshold_minutes
                    .map(|m| Duration::from_secs(m.max(1) * 60));
                container_policy_map.insert(c.id, (never, custom_timeout));
            }

            let containers_to_hibernate: Vec<String> = {
                if let Ok(mut lock) = supervisor.lock() {
                    lock.set_idle_timeout(global_timeout);
                    let now = Instant::now();

                    lock.get_all_managed()
                        .into_iter()
                        .filter(|c| {
                            let (never, custom_timeout) = container_policy_map
                                .get(&c.id)
                                .copied()
                                .unwrap_or((false, None));
                            lock.should_hibernate(&c.id, now, never, custom_timeout)
                        })
                        .map(|c| c.id)
                        .collect()
                } else {
                    Vec::new()
                }
            };

            // NOTE: Background audio playback detection via WebView2's ICoreWebView2_8::IsDocumentPlayingAudio
            // was investigated. Calling it directly requires unsafe unmanaged COM vtable invocations without
            // stable official Tauri/wry support. To maintain stability without adding heavy, volatile COM crate
            // dependencies, automated audio detection is omitted in favor of the explicit 'Never hibernate'
            // user policy toggle.
            for id in containers_to_hibernate {
                println!(
                    "[Supervisor] Idle timeout reached for container '{}'. Triggering hibernation...",
                    id
                );
                let _ = hibernate_container(&app, &supervisor, &id);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_lifecycle() {
        let mut supervisor = ProcessSupervisor::new(SupervisorConfig {
            idle_timeout: Duration::from_millis(50),
            check_interval: Duration::from_millis(10),
        });

        supervisor.register_container("test-id", "Test App", "https://example.com");
        let c = supervisor.get_container("test-id").expect("should exist");
        assert_eq!(c.status, ContainerStatus::Closed);

        // Activate
        supervisor.register_or_activate("test-id", "Test App", "https://example.com", Some(1234));
        let active = supervisor.get_container("test-id").unwrap();
        assert_eq!(active.status, ContainerStatus::Running);
        assert_eq!(active.pid, Some(1234));
        assert!(active.is_focused);

        // Focus loss -> Idle
        supervisor.update_focus("test-id", false);
        let unfocused = supervisor.get_container("test-id").unwrap();
        assert_eq!(unfocused.status, ContainerStatus::Idle);
        assert!(!unfocused.is_focused);

        // Update navigation route
        supervisor.update_url("test-id", "https://example.com/dashboard");
        assert_eq!(
            supervisor.get_container("test-id").unwrap().current_url,
            "https://example.com/dashboard"
        );

        // Hibernate
        supervisor.mark_hibernated("test-id");
        let hibernated = supervisor.get_container("test-id").unwrap();
        assert_eq!(hibernated.status, ContainerStatus::Hibernated);
        assert_eq!(hibernated.pid, None);

        // Remove
        supervisor.remove_container("test-id");
        assert!(supervisor.get_container("test-id").is_none());
    }

    #[test]
    fn test_metrics_generation() {
        let mut supervisor = ProcessSupervisor::new(SupervisorConfig::default());
        supervisor.register_container("c1", "App 1", "https://app1.com");
        let metrics = supervisor.get_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].id, "c1");
        assert_eq!(metrics[0].status, ContainerStatus::Closed);
        assert_eq!(metrics[0].memory_mb, 0.0);
        assert_eq!(metrics[0].cpu_usage, 0.0);
    }

    #[test]
    fn test_never_hibernate_skipped() {
        let mut supervisor = ProcessSupervisor::new(SupervisorConfig {
            idle_timeout: Duration::from_secs(10),
            check_interval: Duration::from_secs(1),
        });

        supervisor.register_or_activate("app1", "App 1", "https://app1.com", None);
        supervisor.update_focus("app1", false);

        let now = Instant::now() + Duration::from_secs(20);

        // When never_hibernate is true, container MUST NOT be hibernated
        assert!(
            !supervisor.should_hibernate("app1", now, true, None),
            "Container with never_hibernate enabled must not be hibernated"
        );

        // When never_hibernate is false, container should be hibernated
        assert!(
            supervisor.should_hibernate("app1", now, false, None),
            "Container with never_hibernate disabled should be hibernated when timeout is exceeded"
        );
    }

    #[test]
    fn test_per_container_threshold_overrides_global() {
        let mut supervisor = ProcessSupervisor::new(SupervisorConfig {
            idle_timeout: Duration::from_secs(100),
            check_interval: Duration::from_secs(1),
        });

        supervisor.register_or_activate("app2", "App 2", "https://app2.com", None);
        supervisor.update_focus("app2", false);

        // Custom override shorter than global (10s vs 100s)
        let now_15s = Instant::now() + Duration::from_secs(15);
        assert!(
            supervisor.should_hibernate("app2", now_15s, false, Some(Duration::from_secs(10))),
            "Per-container 10s threshold must override global 100s threshold after 15s"
        );
        assert!(
            !supervisor.should_hibernate("app2", now_15s, false, None),
            "Global 100s threshold must not trigger after only 15s"
        );

        // Custom override longer than global (300s vs 100s)
        let now_150s = Instant::now() + Duration::from_secs(150);
        assert!(
            !supervisor.should_hibernate("app2", now_150s, false, Some(Duration::from_secs(300))),
            "Per-container 300s threshold must override global 100s threshold at 150s"
        );
        assert!(
            supervisor.should_hibernate("app2", now_150s, false, None),
            "Global 100s threshold would trigger at 150s without override"
        );
    }

    #[test]
    fn test_global_threshold_takes_effect() {
        let mut supervisor = ProcessSupervisor::new(SupervisorConfig {
            idle_timeout: Duration::from_secs(60),
            check_interval: Duration::from_secs(1),
        });

        supervisor.register_or_activate("app3", "App 3", "https://app3.com", None);
        supervisor.update_focus("app3", false);

        let now_40s = Instant::now() + Duration::from_secs(40);
        assert!(
            !supervisor.should_hibernate("app3", now_40s, false, None),
            "Global 60s threshold should not trigger at 40s"
        );

        // Dynamically update global idle timeout
        supervisor.set_idle_timeout(Duration::from_secs(30));
        assert_eq!(supervisor.idle_timeout(), Duration::from_secs(30));

        assert!(
            supervisor.should_hibernate("app3", now_40s, false, None),
            "New global 30s threshold must take effect immediately at 40s"
        );
    }
}
