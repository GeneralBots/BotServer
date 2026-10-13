//! Part of the bottest CLI — see `cli/mod.rs` for the entry point.
//!
//! Moved out of the binary crate by #1505 so the fixture module tree is
//! compiled once; the CLI is part of the `bottest` library now.

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::harness::{TestConfig, TestHarness};

use super::args::RunnerConfig;
use super::cargo::{discover_test_files, run_cargo_test};
use super::driver::{
    check_webdriver_available, detect_browser_version, detect_chromedriver_for_version,
    detect_existing_browser, setup_test_dependencies, start_chromedriver,
};
use super::report::TestResults;

pub fn run_unit_tests(config: &RunnerConfig) -> Result<TestResults> {
    info!("Running unit tests...");

    let mut results = TestResults::new("unit");
    let start = std::time::Instant::now();

    let test_files = discover_test_files("tests/unit");
    if test_files.is_empty() {
        info!("No unit test files found in tests/unit/");
        results.skipped = 1;
        return Ok(results);
    }

    info!("Discovered unit test modules: {:?}", test_files);

    let filter = config.filter.as_deref();
    let env_vars: Vec<(&str, &str)> = vec![];

    match run_cargo_test("unit", filter, config.parallel, env_vars, None) {
        Ok((passed, failed, skipped)) => {
            results.passed = passed;
            results.failed = failed;
            results.skipped = skipped;
        }
        Err(e) => {
            results
                .errors
                .push(format!("Failed to run unit tests: {e}"));
            results.failed = 1;
        }
    }

    results.duration_ms = start.elapsed().as_millis() as u64;

    info!(
        "Unit tests completed: {} passed, {} failed, {} skipped ({} ms)",
        results.passed, results.failed, results.skipped, results.duration_ms
    );

    Ok(results)
}

pub async fn run_integration_tests(config: &RunnerConfig) -> Result<TestResults> {
    info!("Running integration tests...");

    let mut results = TestResults::new("integration");
    let start = std::time::Instant::now();

    if env::var("SKIP_INTEGRATION_TESTS").is_ok() {
        info!("Integration tests skipped (SKIP_INTEGRATION_TESTS is set)");
        results.skipped = 1;
        return Ok(results);
    }

    let test_config = TestConfig::full();
    let ctx = match TestHarness::setup(test_config).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to set up test harness: {}", e);
            results.failed = 1;
            results.errors.push(format!("Harness setup failed: {e}"));
            return Ok(results);
        }
    };

    info!("Test harness ready:");
    info!("  PostgreSQL: {}", ctx.database_url());
    info!("  MinIO: {}", ctx.minio_endpoint());
    info!("  Redis: {}", ctx.redis_url());
    info!("  Mock Zitadel: {}", ctx.zitadel_url());
    info!("  Mock LLM: {}", ctx.llm_url());

    let test_files = discover_test_files("tests/integration");
    if test_files.is_empty() {
        info!("No integration test files found in tests/integration/");
        results.skipped = 1;
        return Ok(results);
    }

    info!("Discovered integration test modules: {:?}", test_files);

    let filter = config.filter.as_deref();
    let db_url = ctx.database_url();
    let directory_url = ctx.zitadel_url();

    let env_vars: Vec<(&str, &str)> = vec![
        ("DATABASE_URL", &db_url),
        ("DIRECTORY_URL", &directory_url),
        ("ZITADEL_CLIENT_ID", "test-client-id"),
        ("ZITADEL_CLIENT_SECRET", "test-client-secret"),
        ("DRIVE_ACCESSKEY", "minioadmin"),
        ("DRIVE_SECRET", "minioadmin"),
    ];

    match run_cargo_test(
        "integration",
        filter,
        config.parallel,
        env_vars,
        Some("integration"),
    ) {
        Ok((passed, failed, skipped)) => {
            results.passed = passed;
            results.failed = failed;
            results.skipped = skipped;
        }
        Err(e) => {
            results
                .errors
                .push(format!("Failed to run integration tests: {e}"));
            results.failed = 1;
        }
    }

    if config.keep_env {
        info!("Keeping test environment for inspection (KEEP_ENV=1)");
        info!("  Data dir: {:?}", ctx.data_dir);
    } else {
        info!("Cleaning up test environment...");
    }

    results.duration_ms = start.elapsed().as_millis() as u64;

    info!(
        "Integration tests completed: {} passed, {} failed, {} skipped ({} ms)",
        results.passed, results.failed, results.skipped, results.duration_ms
    );

    Ok(results)
}

pub async fn run_e2e_tests(config: &RunnerConfig) -> Result<TestResults> {
    info!("Running E2E tests...");

    let mut results = TestResults::new("e2e");
    let start = std::time::Instant::now();

    if env::var("SKIP_E2E_TESTS").is_ok() {
        info!("E2E tests skipped (SKIP_E2E_TESTS is set)");
        results.skipped = 1;
        return Ok(results);
    }

    if config.headed {
        info!("Running with visible browser (HEADED mode)");
    } else {
        info!("Running headless");
    }

    let (chromedriver_path, chrome_path) = match setup_test_dependencies().await {
        Ok(deps) => deps,
        Err(e) => {
            warn!("Failed to setup dependencies: {}", e);
            let browser = detect_existing_browser();
            if browser.is_none() {
                info!("No WebDriver available, skipping E2E tests");
                info!("Run 'bottest --setup' to install dependencies");
                results.skipped = 1;
                return Ok(results);
            }
            let browser_path = browser.unwrap();
            let major = detect_browser_version(&browser_path).unwrap_or_else(|| "131".to_string());
            if let Some(driver) = detect_chromedriver_for_version(&major) {
                (driver, PathBuf::from(browser_path))
            } else {
                info!("No matching ChromeDriver, skipping E2E tests");
                results.skipped = 1;
                return Ok(results);
            }
        }
    };

    let webdriver_port = 4444u16;
    let mut chromedriver_process = None;

    if !check_webdriver_available(webdriver_port).await {
        match start_chromedriver(&chromedriver_path, webdriver_port).await {
            Ok(child) => {
                chromedriver_process = Some(child);
            }
            Err(e) => {
                error!("Failed to start ChromeDriver: {}", e);
                results.failed = 1;
                results
                    .errors
                    .push(format!("ChromeDriver start failed: {e}"));
                return Ok(results);
            }
        }
    }

    let test_config = TestConfig::full();
    let ctx = match TestHarness::setup(test_config).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to set up test harness: {}", e);
            if let Some(mut child) = chromedriver_process {
                let _ = child.kill();
            }
            results.failed = 1;
            results.errors.push(format!("Harness setup failed: {e}"));
            return Ok(results);
        }
    };

    info!("Test harness ready for E2E tests");

    let server = match ctx.start_botserver().await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to start botserver: {}", e);
            if let Some(mut child) = chromedriver_process {
                let _ = child.kill();
            }
            results.failed = 1;
            results.errors.push(format!("Botserver start failed: {e}"));
            return Ok(results);
        }
    };

    if server.is_running() {
        info!("Botserver started at: {}", server.url);
    } else {
        info!("Botserver not running, E2E tests may fail");
    }

    let test_files = discover_test_files("tests/e2e");
    if test_files.is_empty() {
        info!("No E2E test files found in tests/e2e/");
        if let Some(mut child) = chromedriver_process {
            let _ = child.kill();
        }
        results.skipped = 1;
        return Ok(results);
    }

    info!("Discovered E2E test modules: {:?}", test_files);

    let filter = config.filter.as_deref();
    let headed = if config.headed { "1" } else { "" };
    let db_url = ctx.database_url();
    let directory_url = ctx.zitadel_url();
    let server_url = server.url.clone();
    let chrome_binary = chrome_path.to_string_lossy().to_string();
    let webdriver_url = format!("http://localhost:{webdriver_port}");

    let env_vars: Vec<(&str, &str)> = vec![
        ("DATABASE_URL", &db_url),
        ("DIRECTORY_URL", &directory_url),
        ("ZITADEL_CLIENT_ID", "test-client-id"),
        ("ZITADEL_CLIENT_SECRET", "test-client-secret"),
        ("DRIVE_ACCESSKEY", "minioadmin"),
        ("DRIVE_SECRET", "minioadmin"),
        ("BOTSERVER_URL", &server_url),
        ("HEADED", headed),
        ("CHROME_BINARY", &chrome_binary),
        ("WEBDRIVER_URL", &webdriver_url),
    ];

    match run_cargo_test("e2e", filter, false, env_vars, Some("e2e")) {
        Ok((passed, failed, skipped)) => {
            results.passed = passed;
            results.failed = failed;
            results.skipped = skipped;
        }
        Err(e) => {
            results.errors.push(format!("Failed to run E2E tests: {e}"));
            results.failed = 1;
        }
    }

    if let Some(mut child) = chromedriver_process {
        info!("Stopping ChromeDriver...");
        let _ = child.kill();
        let _ = child.wait();
    }

    if config.keep_env {
        info!("Keeping test environment for inspection (KEEP_ENV=1)");
        info!("  Server URL: {}", server.url);
        info!("  Data dir: {:?}", ctx.data_dir);
    } else {
        info!("Cleaning up test environment...");
    }

    results.duration_ms = start.elapsed().as_millis() as u64;

    info!(
        "E2E tests completed: {} passed, {} failed, {} skipped ({} ms)",
        results.passed, results.failed, results.skipped, results.duration_ms
    );

    Ok(results)
}
