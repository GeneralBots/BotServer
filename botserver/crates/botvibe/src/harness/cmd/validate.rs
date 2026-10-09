use super::*;

/// #1489 — the allowlist is a list of binary names, so an explicit absolute
/// path is admitted when its final component is an allowed name. The `..`
/// rejection in `validate_program` still applies to the whole path, and a path
/// such as `/usr/local/bin/docker` is refused exactly like a bare `docker`.
pub(super) fn program_is_allowed(program: &str) -> bool {
    if ALLOWED_COMMANDS.contains(program) {
        return true;
    }
    match Path::new(program).file_name().and_then(|n| n.to_str()) {
        Some(basename) => ALLOWED_COMMANDS.contains(basename),
        None => false,
    }
}

/// Validate a single argument: no shell metacharacters, bounded length this
/// shell-char check mirrors `command_guard` semantics without needing the
/// botcore crate.
pub(super) fn validate_arg(arg: &str) -> Result<(), GuardError> {
    if arg.is_empty() {
        return Err(GuardError::InvalidArgument("empty argument".into()));
    }
    if arg.chars().count() > 4096 {
        return Err(GuardError::InvalidArgument("argument too long".into()));
    }
    for ch in arg.chars() {
        if FORBIDDEN_SHELL_CHARS.contains(&ch) {
            return Err(GuardError::ShellInjection(format!(
                "forbidden character '{ch}' in argument"
            )));
        }
    }
    // Control characters can alter argument parsing semantics even without
    // shell metacharacters — reject anything non-printable (#1297).
    if arg
        .chars()
        .any(|ch| ch.is_control() || char::is_whitespace(ch) && (ch != ' ' && ch != '\t'))
    {
        return Err(GuardError::InvalidArgument(
            "argument contains control characters".into(),
        ));
    }
    Ok(())
}

/// The program name itself must be a plain executable name or absolute path
/// with no traversal or metacharacters (#1297).
pub(super) fn validate_program(program: &str) -> Result<(), GuardError> {
    if program.is_empty() {
        return Err(GuardError::InvalidArgument("empty program".into()));
    }
    if program.chars().any(|ch| FORBIDDEN_SHELL_CHARS.contains(&ch)) {
        return Err(GuardError::ShellInjection(format!(
            "forbidden character in program name '{program}'"
        )));
    }
    if program.contains("..") {
        return Err(GuardError::InvalidArgument(
            "program path must not contain traversal sequences".into(),
        ));
    }
    Ok(())
}
