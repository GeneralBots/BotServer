use super::*;

pub async fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    match command.as_str() {
        "start" => {
            let mode = if args.contains(&"--container".to_string()) {
                InstallMode::Container
            } else {
                InstallMode::Local
            };
            let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
                args.get(idx + 1).cloned()
            } else {
                None
            };
            let pm = PackageManager::new(mode, tenant)?;
            println!("Starting all installed components...");
            let components = get_all_components();
            for component in components {
                if pm.is_installed(component.name) {
                    match pm.start(component.name) {
                        Ok(_) => println!("* Started {}", component.name),
                        Err(e) => eprintln!("x Failed to start {}: {}", component.name, e),
                    }
                }
            }
            println!("* BotServer components started");
        }
        "stop" => {
            println!("Stopping all components...");
            let components = get_all_components();
            for component in components {
                safe_pkill(&["-f", component.termination_command]);
            }
            println!("* BotServer components stopped");
        }
        "restart" => {
            println!("Restarting BotServer...");
            let components = get_all_components();
            for component in components {
                safe_pkill(&["-f", component.termination_command]);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let mode = if args.contains(&"--container".to_string()) {
                InstallMode::Container
            } else {
                InstallMode::Local
            };
            let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
                args.get(idx + 1).cloned()
            } else {
                None
            };
            let pm = PackageManager::new(mode, tenant)?;
            let components = get_all_components();
            for component in components {
                if pm.is_installed(component.name) {
                    let _ = pm.start(component.name);
                }
            }
            println!("* BotServer restarted");
        }
        "install" => {
            if args.len() < 3 {
                eprintln!("Usage: botserver install <component> [--container] [--container-only] [--tenant <name>]");
                return Ok(());
            }
            let component = &args[2];

            if component == "protection" {
                install_protection()?;
                return Ok(());
            }

            let mode = if args.contains(&"--container".to_string()) {
                InstallMode::Container
            } else {
                InstallMode::Local
            };
            let container_only = args.contains(&"--container-only".to_string());
            let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
                args.get(idx + 1).cloned()
            } else {
                None
            };
            let pm = PackageManager::new(mode.clone(), tenant)?;

            let result = if container_only && mode == InstallMode::Container {
                Some(pm.install_container_only(component)?)
            } else {
                pm.install(component).await?
            };

            if let Some(install_result) = result {
                install_result.print();
                if container_only {
                    println!("\n* Container created successfully (--container-only mode)");
                    println!("* Run without --container-only to complete installation");
                }
            }
        }
        "setup-env" => {
            let vault_container = args.get(2).map(|s| s.as_str()).unwrap_or("vault");
            let container_name = vault_container.to_string();

            println!("* Generating .env from vault container: {}", container_name);

            match PackageManager::generate_env_from_vault(&container_name) {
                Ok(env_vars) => {
                    println!("* Successfully generated .env from vault");
                    println!("\nGenerated configuration:");
                    println!("{}", env_vars);
                    println!("\nYou can now start botserver with: botserver start");
                }
                Err(e) => {
                    eprintln!("x Failed to generate .env: {}", e);
                    return Err(anyhow::anyhow!("Setup env failed: {}", e));
                }
            }
        }
        "remove" => {
            if args.len() < 3 {
                eprintln!("Usage: botserver remove <component> [--container] [--tenant <name>]");
                return Ok(());
            }
            let component = &args[2];

            if component == "protection" {
                remove_protection()?;
                return Ok(());
            }

            let mode = if args.contains(&"--container".to_string()) {
                InstallMode::Container
            } else {
                InstallMode::Local
            };
            let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
                args.get(idx + 1).cloned()
            } else {
                None
            };
            let pm = PackageManager::new(mode, tenant)?;
            pm.remove(component)?;
            println!("* Component '{}' removed successfully", component);
        }
        "list" => {
            let mode = if args.contains(&"--container".to_string()) {
                InstallMode::Container
            } else {
                InstallMode::Local
            };
            let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
                args.get(idx + 1).cloned()
            } else {
                None
            };
            let pm = PackageManager::new(mode, tenant)?;
            println!("Available components:");
            for component in pm.list() {
                let status = if pm.is_installed(&component) {
                    "* installed"
                } else {
                    "  available"
                };
                println!(" {} {}", status, component);
            }
        }
        "status" => {
            if args.len() < 3 {
                eprintln!("Usage: botserver status <component> [--container] [--tenant <name>]");
                return Ok(());
            }
            let component = &args[2];

            if component == "protection" {
                let _ = verify_protection();
                return Ok(());
            }

            let mode = if args.contains(&"--container".to_string()) {
                InstallMode::Container
            } else {
                InstallMode::Local
            };
            let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") {
                args.get(idx + 1).cloned()
            } else {
                None
            };
            let pm = PackageManager::new(mode, tenant)?;
            if pm.is_installed(component) {
                println!("* Component '{}' is installed", component);
            } else {
                println!("x Component '{}' is not installed", component);
            }
        }
        "version" => {
            let show_all = args.contains(&"--all".to_string());
            print_version(show_all).await?;
        }
        "rotate-secret" => {
            if args.len() < 3 {
                eprintln!("Usage: botserver rotate-secret <component>");
                eprintln!("Components: tables, drive, cache, email, directory, encryption, jwt");
                return Ok(());
            }
            let component = &args[2];
            rotate_secret(component).await?;
        }
        "rotate-secrets" => {
            let rotate_all = args.contains(&"--all".to_string());
            if rotate_all {
                rotate_all_secrets().await?;
            } else {
                eprintln!("Usage: botserver rotate-secrets --all");
                eprintln!("This will rotate ALL secrets. Use with caution!");
            }
        }
        "certs" => {
            if args.len() < 3 {
                print_certs_usage();
                return Ok(());
            }
            let subcommand = &args[2];
            match subcommand.as_str() {
                "generate" => {
                    let mode = if args.contains(&"--container".to_string()) {
                        InstallMode::Container
                    } else {
                        InstallMode::Local
                    };
                    let tenant = args.iter().position(|a| a == "--tenant")
                        .and_then(|i| args.get(i + 1).cloned());
                    let pm = PackageManager::new(mode, tenant)?;
                    println!("Generating all service certificates...");
                    certs_utils::generate_all_certs(&pm.base_path)?;
                    println!("* All certificates generated successfully");
                }
                "push" => {
                    let ssh_host = if let Some(idx) = args.iter().position(|a| a == "--ssh-host") {
                        args.get(idx + 1).map(|s| s.as_str())
                    } else {
                        Some("10.0.0.1")
                    };
                    if let Some(host) = ssh_host {
                        let mode = if args.contains(&"--container".to_string()) {
                            InstallMode::Container
                        } else {
                            InstallMode::Local
                        };
                        let tenant = args.iter().position(|a| a == "--tenant")
                            .and_then(|i| args.get(i + 1).cloned());
                        let pm = PackageManager::new(mode, tenant)?;
                        println!("Pushing certificates to containers via SSH to {}...", host);
                        certs_utils::push_all_certs_ssh(&pm.base_path, host)?;
                        println!("* All certificates pushed successfully");
                    } else {
                        eprintln!("Usage: botserver certs push --ssh-host <host-ip>");
                    }
                }
                "provision" => {
                    let ssh_host = if let Some(idx) = args.iter().position(|a| a == "--ssh-host") {
                        args.get(idx + 1).map(|s| s.as_str())
                    } else {
                        Some("10.0.0.1")
                    };
                    if let Some(host) = ssh_host {
                        let mode = if args.contains(&"--container".to_string()) {
                            InstallMode::Container
                        } else {
                            InstallMode::Local
                        };
                        let tenant = args.iter().position(|a| a == "--tenant")
                            .and_then(|i| args.get(i + 1).cloned());
                        let pm = PackageManager::new(mode, tenant)?;
                        println!("=== mTLS Certificate Provisioning ===");
                        println!("Phase 1: Generating certificates...");
                        certs_utils::generate_all_certs(&pm.base_path)?;
                        println!("Phase 2: Pushing to containers via SSH to {}...", host);
                        certs_utils::push_all_certs_ssh(&pm.base_path, host)?;
                        println!("* mTLS provisioning complete!");
                        println!();
                        println!("Next steps:");
                        println!("  1. Restart each service to enable mTLS on dual ports");
                        println!("  2. Update connection strings to use mTLS ports");
                        println!("  3. Verify services still work on original ports");
                    } else {
                        eprintln!("Usage: botserver certs provision --ssh-host <host-ip>");
                    }
                }
                _ => {
                    eprintln!("Unknown certs command: {}", subcommand);
                    print_certs_usage();
                }
            }
        }
        "vault" => {
            if args.len() < 3 {
                print_vault_usage();
                return Ok(());
            }
            let subcommand = &args[2];
            match subcommand.as_str() {
                "migrate" => {
                    let env_file = args.get(3).map(|s| s.as_str()).unwrap_or(".env");
                    vault_migrate(env_file).await?;
                }
                "put" => {
                    if args.len() < 4 {
                        eprintln!("Usage: botserver vault put <path> [key=value] [key=value...]");
                        eprintln!("       botserver vault put <path> (interactive mode - prompts for keys)");
                        return Ok(());
                    }
                    let path = &args[3];
                    let kvs: Vec<&str> = args[4..].iter().map(|s| s.as_str()).collect();
                    vault_put(path, &kvs).await?;
                }
                "get" => {
                    if args.len() < 4 {
                        eprintln!("Usage: botserver vault get <path> [key]");
                        return Ok(());
                    }
                    let path = &args[3];
                    let key = args.get(4).map(|s| s.as_str());
                    vault_get(path, key).await?;
                }
                "list" => {
                    vault_list()?;
                }
                "health" => {
                    vault_health().await?;
                }
                _ => {
                    eprintln!("Unknown vault command: {}", subcommand);
                    print_vault_usage();
                }
            }
        }
        "--version" | "-v" => {
            print_version(false).await?;
        }
        "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }
    Ok(())
}
