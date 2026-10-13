//! Part of the bottest CLI — see `cli/mod.rs` for the entry point.
//!
//! Moved out of the binary crate by #1505 so the fixture module tree is
//! compiled once; the CLI is part of the `bottest` library now.

use anyhow::Result;
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSuite {
    Unit,
    Integration,
    E2E,
    All,
}

impl std::str::FromStr for TestSuite {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unit" => Ok(Self::Unit),
            "integration" | "int" => Ok(Self::Integration),
            "e2e" | "end-to-end" => Ok(Self::E2E),
            "all" => Ok(Self::All),
            _ => Err(format!("Unknown test suite: {s}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub suite: TestSuite,
    pub filter: Option<String>,
    pub parallel: bool,
    pub verbose: bool,
    pub keep_env: bool,
    pub headed: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            suite: TestSuite::All,
            filter: None,
            parallel: true,
            verbose: false,
            keep_env: env::var("KEEP_ENV").is_ok(),
            headed: env::var("HEADED").is_ok(),
        }
    }
}

pub fn print_usage() {
    eprintln!(
        r#"
BotTest - Test Runner for General Bots

USAGE:
    bottest [OPTIONS] [SUITE]

SUITES:
    unit            Run unit tests only (fast, no external services)
    integration     Run integration tests (starts real services)
    e2e             Run end-to-end browser tests
    all             Run all test suites (default)

OPTIONS:
    -f, --filter <PATTERN>    Filter tests by name pattern
    -p, --parallel            Run tests in parallel (default)
    -s, --sequential          Run tests sequentially
    -v, --verbose             Enable verbose output
    -k, --keep-env            Keep test environment after completion
    -h, --headed              Run browser tests with visible browser
    --setup                   Download and install test dependencies
    --demo                    Run a quick browser demo (no database needed)
    --help                    Show this help message

ENVIRONMENT VARIABLES:
    KEEP_ENV=1                Keep test environment for inspection
    HEADED=1                  Run browser tests with visible browser
    DATABASE_URL              Override test database URL
    TEST_THREADS              Number of parallel test threads
    SKIP_E2E_TESTS            Skip E2E tests
    SKIP_INTEGRATION_TESTS    Skip integration tests

EXAMPLES:
    bottest unit                      Run all unit tests
    bottest integration -f queue      Run integration tests matching "queue"
    bottest e2e --headed              Run E2E tests with visible browser
    bottest all -v                    Run all tests with verbose output
    bottest --setup                   Install ChromeDriver and dependencies
    bottest --demo                    Open browser and navigate to example.com
"#
    );
}

pub fn parse_args() -> Result<(RunnerConfig, bool, bool)> {
    let args: Vec<String> = env::args().collect();
    let mut config = RunnerConfig::default();
    let mut setup_only = false;
    let mut demo_mode = false;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "--setup" => {
                setup_only = true;
            }
            "--demo" => {
                demo_mode = true;
                config.headed = true;
            }
            "-f" | "--filter" => {
                i += 1;
                if i < args.len() {
                    config.filter = Some(args[i].clone());
                } else {
                    anyhow::bail!("--filter requires a pattern argument");
                }
            }
            "-p" | "--parallel" => {
                config.parallel = true;
            }
            "-s" | "--sequential" => {
                config.parallel = false;
            }
            "-v" | "--verbose" => {
                config.verbose = true;
            }
            "-k" | "--keep-env" => {
                config.keep_env = true;
            }
            "-h" | "--headed" => {
                config.headed = true;
            }
            arg if !arg.starts_with('-') => {
                config.suite = arg.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            other => {
                anyhow::bail!("Unknown argument: {other}");
            }
        }
        i += 1;
    }

    Ok((config, setup_only, demo_mode))
}
