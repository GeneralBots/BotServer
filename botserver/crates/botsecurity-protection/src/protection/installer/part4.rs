use super::*;

impl InstallResult {
    pub fn print(&self) {
        println!("\n=== Security Protection Installation Result ===");
        println!(
            "Status: {}",
            if self.success {
                "✓ SUCCESS"
            } else {
                "✗ FAILED"
            }
        );

        if !self.packages_installed.is_empty() {
            println!("\nInstalled Packages:");
            for pkg in &self.packages_installed {
                println!("  - {pkg}");
            }
        }

        #[cfg(not(windows))]
        {
            println!("\nSudoers Configuration:");
            println!(
                "  - File created: {}",
                if self.sudoers_created { "✓" } else { "✗" }
            );
        }

        println!(
            "\nDatabases Updated: {}",
            if self.databases_updated { "✓" } else { "✗" }
        );

        if !self.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &self.warnings {
                println!("  ! {warning}");
            }
        }

        if !self.errors.is_empty() {
            println!("\nErrors:");
            for error in &self.errors {
                println!("  ✗ {error}");
            }
        }

        println!("\n");
    }
}
