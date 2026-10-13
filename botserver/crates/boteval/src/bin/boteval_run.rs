//! boteval-run: CLI evaluation runner (Issue #800).
//!
//! Loads a dataset and either prints the benchmark artifact or evaluates it
//! against an OpenAI-compatible chat endpoint, then applies the CI gate.
//! With `--harness <workspace-root>` the 14 harness-tagged tasks run through
//! the REAL vibe agent loop (LiveHarnessTarget) instead of a plain LLM call.
//!
//! ```text
//! boteval-run --dump-dataset > datasets/vibe-200.json
//! boteval-run --dataset datasets/vibe-200.json \
//!             --endpoint https://api.example.com/v1/chat/completions \
//!             --api-key $KEY --model gpt-oss-120b [--gate ci/gate.toml]
//! boteval-run --dataset datasets/vibe-200.json \
//!             --endpoint ... --api-key $KEY --model deepseek-x \
//!             --harness /tmp/vibe-eval-workspaces
//! ```
//!
//! Exit codes: 0 = gate passed, 1 = gate failed, 2 = usage or I/O error.

use boteval::ci_gate::{CiGate, CiGateConfig};
use boteval::dataset::Dataset;
use boteval::http::OpenAiCompatibleTarget;
use boteval::live_agent::LiveHarnessTarget;
use boteval::runner::{run_evaluation, run_mixed_evaluation};
use boteval::vibe_suite::vibe_benchmark_json;
use std::path::Path;

const HELP: &str = "\
boteval-run — LLM response evaluation

Usage:
  boteval-run --dump-dataset                     print the vibe-200 benchmark JSON
  boteval-run --dataset <path> --endpoint <url> --api-key <key> --model <model> [--gate <path>] [--harness <workspace-root>]

Options:
  --dataset <path>   dataset JSON file (default: datasets/vibe-200.json)
  --endpoint <url>   OpenAI-compatible chat completions URL
  --api-key <key>    bearer API key
  --model <model>    model identifier (default: gpt-oss-120b)
  --gate <path>      CI gate TOML file (default: ci/gate.toml)
  --harness <dir>    run harness-tagged tasks through the real vibe agent loop
  --dump-dataset     print the generated benchmark instead of evaluating
  --help             show this help
";

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn load_gate(path: &str) -> Result<CiGateConfig, String> {
    let bytes = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read gate file {path}: {e}"))?;
    toml::from_str(&bytes).map_err(|e| format!("invalid gate config {path}: {e}"))
}

fn load_dataset(path: &str) -> Result<Dataset, String> {
    let bytes = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read dataset {path}: {e}"))?;
    Dataset::from_json(&bytes)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help") {
        print!("{HELP}");
        return;
    }

    if args.iter().any(|a| a == "--dump-dataset") {
        match vibe_benchmark_json() {
            Ok(json) => {
                println!("{json}");
            }
            Err(e) => {
                eprintln!("dump failed: {e}");
                std::process::exit(2);
            }
        }
        return;
    }

    let dataset_path = arg_value(&args, "--dataset")
        .unwrap_or_else(|| "datasets/vibe-200.json".to_string());
    let endpoint = match arg_value(&args, "--endpoint") {
        Some(value) => value,
        None => {
            eprintln!("missing required --endpoint (use --help for usage)");
            std::process::exit(2);
        }
    };
    let api_key = arg_value(&args, "--api-key").unwrap_or_default();
    let model = arg_value(&args, "--model")
        .unwrap_or_else(|| "gpt-oss-120b".to_string());
    let gate_path = arg_value(&args, "--gate")
        .unwrap_or_else(|| "ci/gate.toml".to_string());

    let dataset = match load_dataset(&dataset_path) {
        Ok(dataset) => dataset,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if !Path::new(&dataset_path).exists() {
        eprintln!("dataset file not found: {dataset_path}");
        std::process::exit(2);
    }
    if dataset.is_empty() {
        eprintln!("dataset {dataset_path} has no entries");
        std::process::exit(2);
    }

    let target = OpenAiCompatibleTarget::new(endpoint, api_key, model);
    let report = match arg_value(&args, "--harness") {
        Some(workspace_root) => {
            if !Path::new(&workspace_root).is_dir() {
                if let Err(e) = std::fs::create_dir_all(&workspace_root) {
                    eprintln!("cannot create harness workspace {workspace_root}: {e}");
                    std::process::exit(2);
                }
            }
            // The harness tools resolve project workspaces under
            // VIBE_WORKSPACE_ROOT; point them at the eval root so agent runs
            // never touch the server's own workspace directory.
            std::env::set_var("VIBE_WORKSPACE_ROOT", &workspace_root);
            let harness = LiveHarnessTarget::new(workspace_root);
            run_mixed_evaluation(&dataset, &target, &harness).await
        }
        None => run_evaluation(&dataset, &target).await,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("report serialization failed: {e}");
            std::process::exit(2);
        }
    }

    let gate = match load_gate(&gate_path) {
        Ok(config) => CiGate::new(config),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let verdict = gate.evaluate(&report);
    println!(
        "gate: {} ({})",
        if verdict.pass { "PASS" } else { "FAIL" },
        verdict
            .reasons
            .join("; ")
            .if_empty_or("no reasons")
    );
    if !verdict.pass {
        std::process::exit(1);
    }
}

trait IfEmptyOr {
    fn if_empty_or(self, fallback: &str) -> String;
}

impl IfEmptyOr for String {
    fn if_empty_or(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}