use super::*;

/// Run `program args` in `cwd` with a timeout. No `-c` shell strings are
/// composed here: every argument is passed verbatim to `std::process`.
pub fn run(
    program: &str,
    args: &[String],
    cwd: &std::path::Path,
    timeout_secs: u64,
) -> Result<RunOutput, GuardError> {
    let mut command = prepare_command(program, args, cwd)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| GuardError::Spawn(e.to_string()))?;

    // #917 — drain stdout/stderr concurrently with a bounded buffer, then poll
    // the child with the timeout. The previous code joined the reader threads
    // *before* polling the child, so a child that never exited (and thus never
    // closed its pipes) deadlocked the runner and the timeout never fired.
    let stdout_join = child
        .stdout
        .take()
        .map(|o| std::thread::spawn(move || drain_reader(o)));
    let stderr_join = child
        .stderr
        .take()
        .map(|o| std::thread::spawn(move || drain_reader(o)));

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut exit_code: Option<i32> = None;
    let mut timed_out = false;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| GuardError::Io(e.to_string()))?
        {
            exit_code = status.code();
            break;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let stdout = stdout_join
        .map(|j| j.join().map_err(|_| GuardError::Io("stdout thread".into())))
        .transpose()?
        .unwrap_or_default();
    let stderr = stderr_join
        .map(|j| j.join().map_err(|_| GuardError::Io("stderr thread".into())))
        .transpose()?
        .unwrap_or_default();

    if timed_out {
        return Err(GuardError::Timeout);
    }
    Ok(RunOutput {
        exit_code,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Spawn a validated long-lived child without a command shell. The caller
/// owns the child handle and is responsible for monitoring its lifetime.
pub fn spawn_persistent(
    program: &str,
    args: &[String],
    cwd: &std::path::Path,
) -> Result<std::process::Child, GuardError> {
    prepare_command(program, args, cwd)?
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| GuardError::Spawn(e.to_string()))
}
