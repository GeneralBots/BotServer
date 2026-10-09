use super::*;

pub const LLAMA_CPP_VERSION: &str = "b7345";

pub fn get_llama_cpp_url() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "x86_64")]
        {
        let _ = std::path::Path::new("/usr/local/cuda").exists()
            || std::path::Path::new("/opt/cuda").exists()
            || std::env::var("CUDA_HOME").is_ok();

            if std::path::Path::new("/usr/share/vulkan").exists()
                || std::env::var("VULKAN_SDK").is_ok()
            {
                info!("Detected Vulkan - using Vulkan build");
                return get_component_url("llm_linux_vulkan");
            }

            info!("Using standard Ubuntu x64 build (CPU)");
            get_component_url("llm")
        }

        #[cfg(target_arch = "s390x")]
        {
            info!("Detected s390x architecture");
            return get_component_url("llm_linux_s390x");
        }

        #[cfg(target_arch = "aarch64")]
        {
            info!("Detected ARM64 architecture on Linux");
            warn!("No pre-built llama.cpp for Linux ARM64 - LLM will not be available");
            return None;
        }
    }

    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        {
            info!("Detected macOS ARM64 (Apple Silicon)");
            return get_component_url("llm_macos_arm64");
        }

        #[cfg(target_arch = "x86_64")]
        {
            info!("Detected macOS x64 (Intel)");
            return get_component_url("llm_macos_x64");
        }
    }

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            if std::env::var("CUDA_PATH").is_ok() {
                if let Some(output) = safe_nvcc_version() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    if version_str.contains("13.") {
                        info!("Detected CUDA 13.x on Windows");
                        return get_component_url("llm_win_cuda13");
                    } else if version_str.contains("12.") {
                        info!("Detected CUDA 12.x on Windows");
                        return get_component_url("llm_win_cuda12");
                    }
                }
            }

            if std::env::var("VULKAN_SDK").is_ok() {
                info!("Detected Vulkan SDK on Windows");
                return get_component_url("llm_win_vulkan");
            }

            info!("Using standard Windows x64 CPU build");
            return get_component_url("llm_win_cpu_x64");
        }

        #[cfg(target_arch = "aarch64")]
        {
            info!("Detected Windows ARM64");
            return get_component_url("llm_win_cpu_arm64");
        }
    }
}
