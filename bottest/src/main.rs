//! Thin CLI wrapper (#1505).
//!
//! The runner itself lives in `bottest::cli`, so this binary no longer
//! re-declares the fixture module tree the library already compiles.

fn main() -> std::process::ExitCode {
    bottest::cli::run()
}
