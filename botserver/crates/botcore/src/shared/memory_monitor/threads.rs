use super::*;

pub fn register_thread(name: &str, component: &str) {
let info = ThreadInfo {
name: name.to_string(),
started_at: Instant::now(),
last_activity: Instant::now(),
activity_count: 0,
component: component.to_string(),
};
if let Ok(mut registry) = THREAD_REGISTRY.write() {
registry.insert(name.to_string(), info);
}
trace!("Registered: {} (component: {})", name, component);
}

pub fn record_thread_activity(name: &str) {
if let Ok(mut registry) = THREAD_REGISTRY.write() {
if let Some(info) = registry.get_mut(name) {
info.last_activity = Instant::now();
info.activity_count += 1;
}
}
}

pub fn unregister_thread(name: &str) {
if let Ok(mut registry) = THREAD_REGISTRY.write() {
registry.remove(name);
}
info!("Unregistered: {}", name);
}

pub fn log_thread_stats() {
if let Ok(registry) = THREAD_REGISTRY.read() {
info!("Active thread count: {}", registry.len());
for (name, info) in registry.iter() {
let uptime = info.started_at.elapsed().as_secs();
let idle = info.last_activity.elapsed().as_secs();
info!(
"[THREAD] {} | component={} | uptime={}s | idle={}s | activities={}",
name, info.component, uptime, idle, info.activity_count
);
}
}
}
