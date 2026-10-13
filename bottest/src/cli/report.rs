//! Part of the bottest CLI — see `cli/mod.rs` for the entry point.
//!
//! Moved out of the binary crate by #1505 so the fixture module tree is
//! compiled once; the CLI is part of the `bottest` library now.

#[derive(Debug, Clone)]
pub struct TestResults {
    pub suite: String,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

impl TestResults {
    #[must_use]
    pub fn new(suite: &str) -> Self {
        Self {
            suite: suite.to_string(),
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_ms: 0,
            errors: Vec::new(),
        }
    }

    #[must_use]
    pub const fn success(&self) -> bool {
        self.failed == 0 && self.errors.is_empty()
    }
}

pub fn print_summary(results: &[TestResults]) {
    println!("\n{}", "=".repeat(60));
    println!("TEST SUMMARY");
    println!("{}", "=".repeat(60));

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_skipped = 0;
    let mut total_duration = 0;

    for result in results {
        println!(
            "\n{} tests: {} passed, {} failed, {} skipped ({} ms)",
            result.suite, result.passed, result.failed, result.skipped, result.duration_ms
        );

        for error in &result.errors {
            println!("  ERROR: {error}");
        }

        total_passed += result.passed;
        total_failed += result.failed;
        total_skipped += result.skipped;
        total_duration += result.duration_ms;
    }

    println!("\n{}", "-".repeat(60));
    println!(
        "TOTAL: {total_passed} passed, {total_failed} failed, {total_skipped} skipped ({total_duration} ms)"
    );
    println!("{}", "=".repeat(60));

    if total_failed > 0 {
        println!("\n❌ TESTS FAILED");
    } else {
        println!("\n✅ ALL TESTS PASSED");
    }
}
