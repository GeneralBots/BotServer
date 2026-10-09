use super::*;

impl DriveMonitor {
    pub fn start_kb_processor(&self) {
        let kb_manager = self.kb_manager.clone();
        let bot_id = self.bot_id;
        let bot_name = self.bot_name.clone();
        let branch_slug = self.branch_slug.clone();
        let tenant_slug = self.tenant_slug.clone();
        let org_slug = self.org_slug.clone();
        let work_root = self.work_root.clone();
        let pending_kb_index = self.pending_kb_index.clone();
        let files_being_indexed = self.files_being_indexed.clone();
        let kb_indexed_folders = self.kb_indexed_folders.clone();
        let file_repo = self.file_repo.clone();
        let is_processing = self.is_processing.clone();

        tokio::spawn(async move {

            while is_processing.load(Ordering::SeqCst) {
                let kb_key = {
                    let pending = pending_kb_index.write().await;
                    pending.iter().next().cloned()
                };

                let Some(kb_key) = kb_key else {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                };

                let parts: Vec<&str> = kb_key.splitn(2, '_').collect();
                if parts.len() < 2 {
                    let mut pending = pending_kb_index.write().await;
                    pending.remove(&kb_key);
                    continue;
                }

                let kb_folder_name = parts[1];
                let kb_folder_path =
                    work_root.join(&tenant_slug).join(&org_slug)
                        .join(format!("{org_id}.gborg/{branch_slug}.gbai/{bot_name}.gbkb", org_id = current_org_id()))
                        .join(kb_folder_name);

                {
                    let indexing = files_being_indexed.read().await;
                    if indexing.contains(&kb_key) {
                        let mut pending = pending_kb_index.write().await;
                        pending.remove(&kb_key);
                        continue;
                    }
                }

                {
                    let mut indexing = files_being_indexed.write().await;
                    indexing.insert(kb_key.clone());
                }

                log::trace!("Indexing KB: {} for bot: {}", kb_key, bot_name);

                let result =
                    tokio::time::timeout(std::time::Duration::from_secs(120), kb_manager.handle_gbkb_change(bot_id, &bot_name, kb_folder_path.as_path())).await;

                {
                    let mut indexing = files_being_indexed.write().await;
                    indexing.remove(&kb_key);
                }

                {
                    let mut pending = pending_kb_index.write().await;
                    pending.remove(&kb_key);
                }

                match result {
                    Ok(Ok(_)) => {
                        log::info!("Successfully indexed KB: {}", kb_key);
                        {
                            let mut indexed = kb_indexed_folders.write().await;
                            indexed.insert(kb_key.clone());
                        }
                        let pattern = format!("{}/", kb_folder_name);
                        if let Err(e) = file_repo.mark_indexed_by_pattern(&pattern) {
                            log::warn!("Failed to mark files indexed for {}: {}", kb_key, e);
                        }
                    }
                    Ok(Err(e)) => {
                        log::warn!("Failed to index KB {}: {}", kb_key, e);
                        let pattern = format!("{}/", kb_folder_name);
                        if let Err(e) = file_repo.mark_failed_by_pattern(&pattern) {
                            log::warn!("Failed to mark files failed for {}: {}", kb_key, e);
                        }
                    }
                    Err(_) => {
                        log::error!("KB indexing timed out after 120s for {}", kb_key);
                        let pattern = format!("{}/", kb_folder_name);
                        if let Err(e) = file_repo.mark_failed_by_pattern(&pattern) {
                            log::warn!("Failed to mark files failed for {}: {}", kb_key, e);
                        }
                    }
                }
            }

            log::trace!("KB processor stopping for bot {}", bot_name);
        });
    }
}
