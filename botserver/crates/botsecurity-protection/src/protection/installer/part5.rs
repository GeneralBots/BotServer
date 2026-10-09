use super::*;

impl UninstallResult {
    pub fn print(&self) {
        println!("\n=== Security Protection Uninstallation Result ===");
        println!(
            "Status: {}",
            if self.success {
                "✓ SUCCESS"
            } else {
                "✗ FAILED"
            }
        );

        #[cfg(not(windows))]
        {
            println!(
                "Sudoers removed: {}",
                if self.sudoers_removed { "✓" } else { "✗" }
            );
        }

        println!("\nMessage:");
        println!("  {}", self.message);

        if !self.errors.is_empty() {
            println!("\nErrors:");
            for error in &self.errors {
                println!("  ✗ {error}");
            }
        }

        println!("\n");
    }
}
