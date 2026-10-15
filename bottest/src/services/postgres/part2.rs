//! Split from `postgres.rs` (#1504).

use super::*;

impl PostgresService {
    #[must_use]
    pub fn connection_string(&self) -> String {
        self.connection_string.clone()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn build_connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@127.0.0.1:{}/{}",
            self.username, self.password, self.port, self.database_name
        )
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.process {
            log::info!("Stopping PostgreSQL...");

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            let _ = child.kill();

            for _ in 0..50 {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        self.process = None;
                        return Ok(());
                    }
                    Ok(None) => sleep(Duration::from_millis(100)).await,
                    Err(_) => break,
                }
            }

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
            self.process = None;
        }

        Ok(())
    }

    pub fn cleanup(&self) -> Result<()> {
        if self.data_dir.exists() {
            std::fs::remove_dir_all(&self.data_dir)?;
        }
        Ok(())
    }
}

impl Drop for PostgresService {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            let _ = child.kill();

            std::thread::sleep(Duration::from_millis(500));

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
