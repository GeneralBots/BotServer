use super::*;

pub async fn download_file(url: &str, output_path: &str) -> Result<(), anyhow::Error> {
    let url = url.to_string();
    let output_path = output_path.to_string();
    let download_handle = tokio::spawn(async move {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; BotServer/1.0)")
            .connect_timeout(std::time::Duration::from_secs(30))
            .read_timeout(std::time::Duration::from_secs(300))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()?;
        let response = client.get(&url).send().await?;
        if response.status().is_success() {
            let bytes = response.bytes().await?;
            let mut file = tokio::fs::File::create(&output_path).await?;
            use tokio::io::AsyncWriteExt;
            file.write_all(&bytes).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("HTTP {}: {}", response.status(), url))
        }
    });
    download_handle.await?
}

pub fn exec_in_container(container: &str, command: &str) -> Result<()> {
    info!("Executing in container {}: {}", container, command);
    let output = std::process::Command::new("lxc")
        .args(["exec", container, "--", "bash", "-c", command])
        .output()
        .ok()
        .ok_or_else(|| anyhow::anyhow!("Failed to execute lxc command"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!(
            "Container command failed.\nCommand: {}\nStderr: {}\nStdout: {}",
            command, stderr, stdout
        );
        return Err(anyhow::anyhow!(
            "Container command failed: {}",
            if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() }
        ));
    }
    Ok(())
}

pub fn download_in_container(container: &str, url: &str, _component: &str, binary_name: Option<&str>) -> Result<()> {
    let download_cmd = format!("wget -O /tmp/download.tmp {}", url);
    exec_in_container(container, &download_cmd)?;
    let path = std::path::Path::new(url);
    let is_tar_gz = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tgz"))
        || (path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gz"))
            && path
                .file_stem()
                .and_then(|s| std::path::Path::new(s).extension())
                .is_some_and(|e| e.eq_ignore_ascii_case("tar")));
    let is_zip = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
    if is_tar_gz {
        exec_in_container(container, "tar -xzf /tmp/download.tmp -C /opt/gbo/bin")?;
    } else if is_zip {
        exec_in_container(container, "unzip -o /tmp/download.tmp -d /opt/gbo/bin")?;
    } else if let Some(name) = binary_name {
        let mv_cmd = format!(
            "mv /tmp/download.tmp /opt/gbo/bin/{} && chmod +x /opt/gbo/bin/{}",
            name, name
        );
        exec_in_container(container, &mv_cmd)?;
    }
    exec_in_container(container, "rm -f /tmp/download.tmp")?;
    Ok(())
}
