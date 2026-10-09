use super::*;

/// Create the branch's default bot project (#1500). Idempotent: an existing
/// `(branch_id, name)` project is returned untouched (bot pair re-checked).
pub(crate) fn ensure_branch_default_project_conn(
    conn: &mut PgConnection,
    org_id: Uuid,
    branch_id: Uuid,
    name: &str,
) -> Result<Uuid, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || branch_id.is_nil() {
        return Err("vibe bootstrap: branch name/id required".to_string());
    }
    if let Err(e) = conn.batch_execute(VIBE_PROJECTS_FALLBACK_SCHEMA) {
        log::warn!("vibe bootstrap: schema fallback failed (may already exist): {e}");
    }

    // The signup hook only carries a branch_id and callers have passed it as
    // the org too — inserting bot rows with org = branch then dies on
    // `bots_org_id_fkey`. Resolve the owning org from the branch row itself;
    // nil is the documented fallback for branch rows without an org.
    let passed_org_id = org_id;
    let org_id =
        crate::bootstrap_backfill::canonical_org_for_branch(conn, branch_id);
    if org_id != passed_org_id {
        log::warn!(
            "vibe bootstrap: caller org {passed_org_id} != branch org {org_id} — using the branch's org"
        );
    }
    let org_id = if org_id.is_nil() {
        passed_org_id
    } else {
        org_id
    };

    let existing = diesel::sql_query(
        "SELECT id FROM vibe_projects WHERE branch_id = $1 AND name = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .bind::<diesel::sql_types::Text, _>(trimmed)
    .get_result::<ProjectIdRow>(conn)
    .optional()
    .map_err(|e| format!("vibe bootstrap lookup: {e}"))?;

    let project_id = match existing {
        Some(row) => {
            // Self-heal rows created by the buggy caller with org = branch:
            // the wrong org_id breaks project→org joins downstream.
            if let Err(e) = diesel::sql_query(
                "UPDATE vibe_projects SET org_id = $3 WHERE id = $1 AND org_id <> $3 \
                 AND org_id = $2",
            )
            .bind::<diesel::sql_types::Uuid, _>(row.id)
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .execute(conn)
            {
                log::warn!("vibe bootstrap: heal project org {}: {e}", row.id);
            }
            row.id
        }
        None => {
            let id = Uuid::new_v4();
            let payload = serde_json::json!({ "bootstrap": true, "source": "workspace" });
            diesel::sql_query(
                "INSERT INTO vibe_projects \
                 (id, org_id, branch_id, name, project_type, repository, framework, custom_domain, \
                  source_control, status, environment, payload, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, 'bot', $4, '', '', 'git', 'ready', 'development', $5, NOW(), NOW()) \
                 ON CONFLICT (branch_id, name) DO NOTHING",
            )
            .bind::<diesel::sql_types::Uuid, _>(id)
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Text, _>(trimmed)
            .bind::<diesel::sql_types::Jsonb, _>(&payload)
            .execute(conn)
            .map_err(|e| format!("vibe bootstrap insert: {e}"))?;
            let row = diesel::sql_query(
                "SELECT id FROM vibe_projects WHERE branch_id = $1 AND name = $2",
            )
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Text, _>(trimmed)
            .get_result::<ProjectIdRow>(conn)
            .map_err(|e| format!("vibe bootstrap reselect: {e}"))?;
            log::info!(
                "vibe bootstrap: default bot project '{}' created for branch {}",
                trimmed,
                branch_id
            );
            row.id
        }
    };

    ensure_bot_rows_conn(
        conn,
        &BotRowRef {
            id: project_id,
            org_id,
            branch_id,
            name: trimmed.to_string(),
        },
        Some(&format!("Default bot of branch {trimmed}")),
    )?;
    // #1440 — an auto-created project with no project_members row resolves
    // every human to Viewer, so the Properties Delete button 403s for the
    // workspace owner. Grant Owner to the org's owner identity up front.
    grant_default_project_owner(conn, project_id, org_id);
    Ok(project_id)
}

/// #1440 — grants Owner on the default project to the workspace's human
/// owner. Two identities may exist for the signup email and both are
/// granted (idempotently): the `users` row id (canonical UUIDv5 of
/// `zitadel:{directory_user_id}`, provisioned at signup) and the derived
/// `UUIDv5(zitadel:{email})` fallback used by email-keyed flows. Best-effort
/// and silent: a workspace without an owner contact (Drive-discovered) or a
/// missing table simply gets no grant.
pub(crate) fn grant_default_project_owner(conn: &mut PgConnection, project_id: Uuid, org_id: Uuid) {
    #[derive(diesel::QueryableByName)]
    struct OwnerEmail {
        #[diesel(sql_type = diesel::sql_types::Text)]
        email: String,
    }
    #[derive(diesel::QueryableByName)]
    struct OwnerId {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    let emails: Vec<OwnerEmail> = diesel::sql_query(
        "SELECT email FROM crm_contacts \
         WHERE org_id = $1 AND email IS NOT NULL AND email <> '' \
         ORDER BY created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(org_id)
    .load(conn)
    .unwrap_or_default();
    let mut owner_ids: Vec<Uuid> = diesel::sql_query(
        "SELECT id FROM users WHERE email = ANY($1) ORDER BY created_at ASC",
    )
    .bind::<diesel::sql_types::Array<diesel::sql_types::Text>, _>(
        emails.iter().map(|e| e.email.clone()).collect::<Vec<_>>(),
    )
    .load(conn)
    .unwrap_or_default()
    .into_iter()
    .map(|r: OwnerId| r.id)
    .collect();
    for email in &emails {
        owner_ids.push(Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("zitadel:{}", email.email).as_bytes(),
        ));
    }
    for owner_id in owner_ids {
        if owner_id.is_nil() {
            continue;
        }
        let inserted = diesel::sql_query(
            "INSERT INTO project_members (project_id, user_id, role) \
             SELECT $1, $2, 'owner' \
             WHERE NOT EXISTS ( \
                 SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 \
             )",
        )
        .bind::<diesel::sql_types::Uuid, _>(project_id)
        .bind::<diesel::sql_types::Uuid, _>(owner_id)
        .execute(conn);
        match inserted {
            Ok(n) if n > 0 => log::info!(
                "vibe bootstrap: granted Owner on default project {project_id} to {owner_id}"
            ),
            Ok(_) => {}
            Err(e) => log::warn!("vibe bootstrap: owner grant on {project_id} failed: {e}"),
        }
    }
}
