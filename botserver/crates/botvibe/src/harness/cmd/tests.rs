#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    pub(super) fn rejects_non_allowlisted_commands() {
        let cwd = std::env::temp_dir();
        let err = run("docker", &["ps".to_string()], &cwd, 5);
        assert!(matches!(err, Err(GuardError::CommandNotAllowed(name)) if name == "docker"));
    }

    #[test]
    pub(super) fn rejects_shell_metacharacters_in_args() {
        let cwd = std::env::temp_dir();
        let args = vec!["--help".to_string(), "echo hi; rm -rf /".to_string()];
        assert!(matches!(
            run("git", &args, &cwd, 5),
            Err(GuardError::ShellInjection(_))
        ));
    }

    #[test]
    pub(super) fn sh_is_not_allowlisted() {
        assert!(
            !ALLOWED_COMMANDS.contains("sh"),
            "sh -c is arbitrary code execution"
        );
        // #1489 — admitting absolute paths must not become a way in.
        assert!(!program_is_allowed("/bin/sh"));
    }

    #[test]
    pub(super) fn runs_allowlisted_command() {
        let cwd = std::env::temp_dir();
        let out = run("git", &["--version".to_string()], &cwd, 30);
        let out = out.expect("git should run");
        assert_eq!(out.exit_code, Some(0));
        assert!(out.stdout.contains("git version"));
    }

    #[test]
    pub(super) fn allowlist_contains_harness_commands() {
        for cmd in ["git", "cat", "ls", "tail", "npm", "cargo", "python3"] {
            assert!(ALLOWED_COMMANDS.contains(cmd), "{cmd} must be allowlisted");
        }
    }

    // #1489 — the allowlist is a list of binary names, so an explicit absolute
    // path to an allowed binary must resolve while a path to a refused binary
    // must not.
    #[test]
    pub(super) fn absolute_path_to_allowed_binary_is_accepted() {
        assert!(program_is_allowed("/usr/local/bin/codegraph"));
        assert!(program_is_allowed("/usr/bin/git"));
        assert!(program_is_allowed("git"));
    }

    #[test]
    pub(super) fn absolute_path_to_refused_binary_is_rejected() {
        assert!(!program_is_allowed("/usr/local/bin/docker"));
        assert!(!program_is_allowed("/bin/sh"));
        assert!(!program_is_allowed("docker"));
    }

    #[test]
    pub(super) fn traversal_in_program_path_is_still_rejected() {
        let cwd = std::env::temp_dir();
        let err = run("/usr/local/../bin/git", &[], &cwd, 5);
        assert!(matches!(err, Err(GuardError::InvalidArgument(_))));
    }

    // #1489 — the child must be able to resolve a binary that lives only under
    // `/usr/local/bin` (the npm global prefix), which `confstr(_CS_PATH)` does
    // not cover. `node` is used as the stand-in because it is allowlisted and
    // present on every supported host; the assertion is that PATH reaches the
    // child at all, which is what was broken.
    #[cfg(not(target_os = "windows"))]
    #[test]
    pub(super) fn child_receives_a_path() {
        let cwd = std::env::temp_dir();
        let out = run("node", &["-p".to_string(), "process.env.PATH".to_string()], &cwd, 30);
        let out = out.expect("node should run");
        assert_eq!(out.exit_code, Some(0));
        let reported = out.stdout.trim();
        assert!(!reported.is_empty(), "child PATH must not be empty");
        assert_eq!(
            reported, CHILD_PATH,
            "child must get the constant guard PATH, not an inherited one"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    pub(super) fn child_path_covers_the_npm_global_prefix() {
        assert!(
            CHILD_PATH.split(':').any(|dir| dir == "/usr/local/bin"),
            "npm global installs land in /usr/local/bin"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    pub(super) fn runs_npm_without_a_command_shell() {
        let cwd = std::env::temp_dir();
        let out = run("npm", &["--version".to_string()], &cwd, 30)
            .expect("npm should run through node.exe");
        assert_eq!(out.exit_code, Some(0));
        assert!(!out.stdout.trim().is_empty());
    }
}
