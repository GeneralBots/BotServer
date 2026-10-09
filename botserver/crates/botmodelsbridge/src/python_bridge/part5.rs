use super::*;

impl Drop for PythonFaceBridge {
    fn drop(&mut self) {
        if let Ok(mut process_guard) = self.process.try_lock() {
            if let Some(mut process) = process_guard.take() {
                let _ = process.child.kill();
            }
        }
    }
}
