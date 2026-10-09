use super::*;

impl BootstrapManager {
    pub(super) fn templates_source_dir(&self) -> PathBuf {
        let work = PathBuf::from("work/templates");
        if work.is_dir() {
            work
        } else {
            PathBuf::from("bottemplates/bots")
        }
    }

    /// Every shipped template is laid out as `bots/<name>/<name>.gbai/`. Both
    /// the database sync and the Drive upload must operate on the `.gbai`
    /// directory itself: collecting every `*.gbai` recursively is what makes the
    /// catalog discoverable — the previous single-level scan matched no entry at
    /// all (zero templates synced, #1354) and copied the wrapper directory name,
    /// producing a Drive layout no runtime resolver discovers (#1355).
    pub(super) fn template_gbai_dirs(&self) -> Vec<(String, PathBuf)> {
        let root = self.templates_source_dir();
        let mut found = Vec::new();
        collect_gbai_dirs_recursive(&root, 3, &mut found);
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }

    /// Sync the on-disk template bots into the `app_templates` table so the
    /// templates API lists real, persisted entries. Each `*.gbai` directory
    /// becomes one row; existing names are refreshed (version/description),
    /// never duplicated.
    pub fn sync_templates_to_database(&self) -> anyhow::Result<()> {
        info!("Syncing templates to database...");
        let source = self.templates_source_dir();
        if !source.is_dir() {
            warn!(
                "No templates directory found at {}; skipping sync",
                source.display()
            );
            return Ok(());
        }

        let mut conn = crate::shared::utils::establish_pg_connection()?;
        let branch_id = crate::shared::utils::current_org_id();

        let mut synced = 0usize;
        for (bot_name, source_dir) in self.template_gbai_dirs() {
            debug!("Template {bot_name} discovered at {}", source_dir.display());

            // Idempotent by (name, branch): refresh an existing row, insert a
            // new one otherwise — never duplicate templates across restarts.
            let updated = diesel::sql_query(
                "UPDATE app_templates SET version = '1.0' \
                 WHERE name = $1 AND branch_id = $2",
            )
            .bind::<diesel::sql_types::Text, _>(bot_name.clone())
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .execute(&mut conn)
            .with_context(|| format!("Failed to update template {bot_name}"))?;
            if updated == 0 {
                let id = Uuid::new_v4();
                diesel::sql_query(
                    "INSERT INTO app_templates (id, name, description, kind, version, author, branch_id) \
                     VALUES ($1, $2, '', 'bot', '1.0', '', $3)",
                )
                .bind::<diesel::sql_types::Uuid, _>(id)
                .bind::<diesel::sql_types::Text, _>(bot_name.clone())
                .bind::<diesel::sql_types::Uuid, _>(branch_id)
                .execute(&mut conn)
                .with_context(|| format!("Failed to insert template {bot_name}"))?;
            }
            synced += 1;
        }
        info!("Synced {synced} templates to database");
        Ok(())
    }

    /// Copy template bots into the org work path (`{work}/{org}.gborg/`)
    /// so the drive monitor discovers and loads them into MinIO. Existing
    /// copies are replaced recursively; the whole pass is best-effort and
    /// never fails startup when a template is malformed.
    pub async fn upload_templates_to_drive(&self, _cfg: &AppConfig) -> anyhow::Result<()> {
        info!("Uploading templates to drive...");
        let source = self.templates_source_dir();
        if !source.is_dir() {
            warn!(
                "No templates directory found at {}; skipping upload",
                source.display()
            );
            return Ok(());
        }

        let org_id = crate::shared::utils::current_org_id();
        let org_work = crate::shared::utils::get_org_work_path(org_id);
        let target_root = PathBuf::from(org_work);
        if !target_root.exists() {
            std::fs::create_dir_all(&target_root).with_context(|| {
                format!("Failed to create org work dir {}", target_root.display())
            })?;
        }

        let mut uploaded = 0usize;
        for (bot_name, source_dir) in self.template_gbai_dirs() {
            // Org layout `{org}.gborg/{bot}.gbai/…` — the shape every runtime
            // resolver (tool exec, prompt load, drive_monitor) expects (#1355).
            // Copying the wrapper directory instead produced
            // `{org}.gborg/{name}/…`, which no resolver discovers.
            let target = target_root.join(format!("{bot_name}.gbai"));
            if let Err(e) = copy_dir_recursive(&source_dir, &target) {
                warn!("Failed to copy template {bot_name}: {e}");
                continue;
            }
            uploaded += 1;
        }
        info!("Uploaded {uploaded} templates to drive work path");
        Ok(())
    }
}
