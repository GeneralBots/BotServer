//! The bottest CLI: argument parsing, the unit/integration/E2E runners and
//! the ChromeDriver provisioning they need.
//!
//! Part of the library (not the binary) since #1505 — `bottest/src/main.rs` is a
//! wrapper, so the fixture modules are compiled once per build instead of twice.

mod args;
mod cargo;
mod driver;
mod report;
mod suites;

pub use args::{parse_args, print_usage, RunnerConfig, TestSuite};
pub use report::{print_summary, TestResults};

use std::process::ExitCode;
use tracing::{error, info};

use driver::{run_browser_demo, setup_logging, setup_test_dependencies};
use suites::{run_e2e_tests, run_integration_tests, run_unit_tests};

/// Run the CLI: parse the arguments, execute the requested suites and
/// translate the aggregate result into a process exit code.
#[tokio::main]
pub async fn run() -> ExitCode {
    let (config, setup_only, demo_mode) = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            print_usage();
            return ExitCode::from(1);
        }
    };

    setup_logging(config.verbose);

    info!(
        "BotTest - General Bots Test Suite v{}",
        env!("CARGO_PKG_VERSION")
    );

    if setup_only {
        info!("Setting up test dependencies...");
        match setup_test_dependencies().await {
            Ok((chromedriver, chrome)) => {
                println!("\n✅ Dependencies installed successfully!");
                println!("  ChromeDriver: {}", chromedriver.display());
                println!("  Browser: {}", chrome.display());
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("\n❌ Setup failed: {e}");
                return ExitCode::from(1);
            }
        }
    }

    if demo_mode {
        info!("Running browser demo...");
        match run_browser_demo().await {
            Ok(()) => {
                println!("\n✅ Browser demo completed successfully!");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("\n❌ Browser demo failed: {e}");
                return ExitCode::from(1);
            }
        }
    }

    info!("Running {:?} tests", config.suite);

    let start = std::time::Instant::now();
    let mut all_results = Vec::new();

    let result = match config.suite {
        TestSuite::Unit => run_unit_tests(&config),
        TestSuite::Integration => run_integration_tests(&config).await,
        TestSuite::E2E => run_e2e_tests(&config).await,
        TestSuite::All => {
            let unit = run_unit_tests(&config);
            let integration = run_integration_tests(&config).await;
            let e2e = run_e2e_tests(&config).await;

            match (unit, integration, e2e) {
                (Ok(u), Ok(i), Ok(e)) => {
                    all_results.push(u);
                    all_results.push(i);
                    all_results.push(e);
                    Ok(TestResults::new("all"))
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
            }
        }
    };

    match result {
        Ok(results) => {
            if all_results.is_empty() {
                all_results.push(results);
            }
        }
        Err(e) => {
            error!("Test execution failed: {}", e);
            return ExitCode::from(1);
        }
    }

    let total_duration = start.elapsed();
    for result in &mut all_results {
        if result.duration_ms == 0 {
            result.duration_ms = total_duration.as_millis() as u64;
        }
    }

    print_summary(&all_results);

    let all_passed = all_results.iter().all(TestResults::success);
    if all_passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
