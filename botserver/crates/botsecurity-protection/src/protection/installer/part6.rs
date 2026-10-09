use super::*;

impl VerifyResult {
    pub fn print(&self) {
        println!("\n=== Security Protection Verification ===");
        println!(
            "All tools installed: {}",
            if self.all_installed { "✓" } else { "✗" }
        );
        println!(
            "All tools configured: {}",
            if self.all_configured { "✓" } else { "✗" }
        );

        #[cfg(not(windows))]
        {
            println!(
                "Sudoers file exists: {}",
                if self.sudoers_exists { "✓" } else { "✗" }
            );
        }

        println!("\nTool Status:");
        for tool in &self.tools {
            println!(
                "  {} {}: {} {}",
                if tool.installed { "✓" } else { "✗" },
                tool.name,
                if tool.installed {
                    "installed"
                } else {
                    "not installed"
                },
                if tool.sudo_configured {
                    "(configured)"
                } else {
                    "(not configured)"
                }
            );
        }

        println!("\n");
    }
}
