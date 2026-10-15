//! #747 — Command runner behind the Vibe tools.
//!
//! Mirrors the `SafeCommand` discipline: an explicit binary allowlist, no
//! shell metacharacters in arguments, no environment overrides, a hard
//! timeout, and bounded output capture. This is the only place in the
//! harness that spawns processes.

pub(crate) use std::collections::HashSet;
pub(crate) use std::path::Path;
pub(crate) use std::process::Stdio;
pub(crate) use std::sync::LazyLock;

static ALLOWED_COMMANDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "ls",
        "cat",
        "head",
        "tail",
        "grep",
        "find",
        "wc",
        "diff",
        "stat",
        "git",
        "mkdir",
        "touch",
        "cp",
        "mv",
        "rm",
        // #917 — `sh` is deliberately absent: `sh -c "..."` executes arbitrary
        // code with no blocked metacharacter, and no harness tool needs it.
        "node",
        "npm",
        "npx",
        "cargo",
        "python3",
        "python",
        "botserver",
        "botc",
        "caddy",
        "incus",
        "wsl",
        "dig",
        "nslookup",
        // #1486 — CodeGraph: pre-indexed code knowledge graph (symbols, call
        // edges, blast radius). Installed into `apps`-kind project VMs; the
        // `code/*` wired tools shell out through here.
        "codegraph",
    ])
});

const FORBIDDEN_SHELL_CHARS: [char; 9] = [';', '|', '&', '$', '`', '<', '>', '\n', '\0'];
const MAX_OUTPUT_BYTES: usize = 512 * 1024;
const MAX_ARGS: usize = 64;

/// #1489 — `PATH` handed to the child process. `prepare_command` clears the
/// environment, so without this the child inherits nothing and `execvp` falls
/// back to `confstr(_CS_PATH)`, which on this host is only `/bin:/usr/bin`.
/// A binary installed by an npm global prefix (`/usr/local/bin`) was therefore
/// unreachable from `shell/run` while being perfectly reachable from a normal
/// shell — surfacing as `Spawn("No such file or directory")`, which reads like
/// a missing binary rather than a missing `PATH` entry.
///
/// Constant, never inherited from the botserver process: the guard decides what
/// the child can see.
const CHILD_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardError {
    CommandNotAllowed(String),
    InvalidArgument(String),
    ShellInjection(String),
    Spawn(String),
    Timeout,
    Io(String),
}

impl std::fmt::Display for GuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandNotAllowed(c) => write!(f, "command not in allowlist: {c}"),
            Self::InvalidArgument(a) => write!(f, "invalid argument: {a}"),
            Self::ShellInjection(s) => write!(f, "shell injection attempt: {s}"),
            Self::Spawn(m) => write!(f, "spawn failed: {m}"),
            Self::Timeout => write!(f, "command exceeded time limit"),
            Self::Io(m) => write!(f, "io: {m}"),
        }
    }
}

#[derive(Debug)]
pub struct RunOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

mod prepare;
mod validate;
mod runner;
mod drain;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod harness_cmd_tests;

use prepare::*;
use validate::*;
pub use runner::*;
use drain::*;
