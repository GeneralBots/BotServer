//! Project eviction policy — a branch keeps at most
//! `VIBE_MAX_PROJECTS_PER_KIND` (default 2) projects of each kind
//! (bot / website / custom). Creating a project beyond the cap evicts the
//! OLDEST project of the same kind on that branch, reclaiming every asset it
//! owns: Incus VMs, published proxy site (payload + route + Caddy block) and
//! the on-disk workspace directory. Without eviction the workspace tree and
//! stopped VM containers grow without bound and exhaust the host disk.
//!
//! Eviction mirrors `delete_project` semantics: asset cleanup is best-effort
//! (logged, never fatal), the project row delete is the source of truth.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::projects::{Project, ProjectRegistry};
use crate::vm_lifecycle::VmLifecycle;

/// Maximum projects of one kind per branch. Overridable for tests/dev via
/// `VIBE_MAX_PROJECTS_PER_KIND`; the cap never goes below 1.
pub fn max_projects_per_kind() -> usize {
    std::env::var("VIBE_MAX_PROJECTS_PER_KIND")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(2)
}

/// Decide which project names to evict. `existing` is (created_at, name)
/// pairs, any order; the OLDEST are returned first until the survivor count
/// fits within `max`. Pure — unit-tested.
pub fn pick_evictions(existing: &[(DateTime<Utc>, String)], max: usize) -> Vec<String> {
    if existing.len() <= max {
        return Vec::new();
    }
    let mut sorted: Vec<&(DateTime<Utc>, String)> = existing.iter().collect();
    sorted.sort_by_key(|a| a.0);
    let overflow = existing.len() - max;
    sorted.iter().take(overflow).map(|(_, n)| n.clone()).collect()
}

/// Delete every asset owned by a project (best-effort, errors returned as
/// strings for the caller to log). Shared by explicit DELETE and eviction.
/// Steps: Incus VMs (rows + containers) → published proxy site (purge) →
/// bound project_domains (Caddy routes + rows) → on-disk workspace directory.
pub async fn delete_project_assets(p: &Project, lifecycle: &VmLifecycle) -> Vec<String> {
    let mut errors = Vec::new();

    // 1. Cascade-delete the project's VMs (rows + Incus containers).
    if let Err(e) = lifecycle.delete_all_for_project(p.id) {
        errors.push(format!("vm cleanup: {e}"));
    }

    // 2. Unpublish + purge the published proxy site, if any (removes payload,
    //    systemd unit for python sites, Caddy route and release history).
    let slug = VmLifecycle::alm_repo(&p.name);
    if !slug.is_empty() {
        if let Err(e) = crate::proxy_sites::unpublish_site(&slug, true).await {
            errors.push(format!("site unpublish: {e}"));
        }
    }

    // 2b. Unbind every domain binding for the project (Caddy route removal
    //     via the admin API + project_domains row delete). VM-deployed sites
    //     have no proxy payload dir, so the site unpublish above cannot
    //     remove their routes — without this step the deleted project's
    //     domain keeps answering (502) and the row lingers forever.
    let domains = crate::domains::ProjectDomains::new(lifecycle.pool().clone());
    match domains.list(p.id) {
        Ok(binds) => {
            for bind in binds {
                if let Err(e) = domains.unbind(bind.id).await {
                    errors.push(format!("domain unbind {}: {e}", bind.domain));
                }
            }
        }
        Err(e) => errors.push(format!("domain list: {e}")),
    }

    // 2c. #1386 — drop the project's per-environment databases (public +
    //     `_dev` twin). Best-effort: failures are reported, not fatal.
    for e in crate::project_db::drop_project_databases(lifecycle.pool(), p.branch_id, &p.name) {
        errors.push(format!("database drop: {e}"));
    }

    // 2d. Remove the project's persisted runs (and their telemetry) so the
    //     Knowledge Graph — which aggregates `vibe_runs` when no project is
    //     selected — never renders ghost nodes for a deleted project.
    if let Err(e) = crate::run_store::delete_runs_for_project(lifecycle.pool(), p.id) {
        errors.push(format!("run history cleanup: {e}"));
    }

    // 3. Remove the workspace directory (the disk leak — it can hold node_modules,
    //    venvs and build output for the whole project lifetime).
    let key = VmLifecycle::alm_repo(&p.name);
    if !key.is_empty() && !key.contains("..") && !key.contains('/') {
        let dir = crate::harness::workspace_root().join(&key);
        match tokio::fs::remove_dir_all(&dir).await {
            Ok(_) => log::info!("Vibe: removed workspace dir {}", dir.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("workspace removal {}: {e}", dir.display())),
        }
    }

    errors
}

/// Enforce the per-kind cap on `branch_id` before creating a new project of
/// `project_type`. Evicts oldest same-kind projects (with full asset cleanup)
/// and returns their names; hard failures (DB) surface as Err.
pub async fn evict_oldest_if_needed(
    registry: &ProjectRegistry,
    lifecycle: &Arc<VmLifecycle>,
    branch_id: Uuid,
    project_type: &str,
) -> Result<Vec<String>, String> {
    let cap = max_projects_per_kind();
    // This runs BEFORE the new project row is inserted: the newcomer takes
    // one slot, so at most `cap - 1` same-kind projects may survive on the
    // branch. (Keeping `cap` here would allow cap+1 rows to exist between
    // eviction and insert.)
    let keep = cap.saturating_sub(1);
    let query = crate::projects::ListProjectsQuery {
        branch_id: Some(branch_id),
        project_type: Some(project_type.to_string()),
        status: None,
        limit: Some(500),
        offset: None,
        all_branches: false,
    };
    let existing: Vec<Project> = registry.list(&query)?;
    let pairs: Vec<(DateTime<Utc>, String)> =
        existing.iter().map(|p| (p.created_at, p.name.clone())).collect();
    let victims = pick_evictions(&pairs, keep);
    if victims.is_empty() {
        return Ok(Vec::new());
    }

    let mut evicted = Vec::new();
    for name in victims {
        let Some(p) = existing.iter().find(|p| p.name == name) else {
            continue;
        };
        let id = p.id;
        for e in delete_project_assets(p, lifecycle).await {
            log::warn!("Vibe eviction {name}: asset cleanup: {e}");
        }
        match registry.delete(id) {
            Ok(true) => {
                log::info!(
                    "Vibe: evicted project '{name}' (id={id}, kind={project_type}) — per-branch cap {cap} reached"
                );
                evicted.push(name);
            }
            Ok(false) => log::warn!("Vibe eviction: project {id} already gone"),
            Err(e) => return Err(format!("evict project '{name}': {e}")),
        }
    }
    Ok(evicted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(days_ago: i64) -> DateTime<Utc> {
        Utc::now() - chrono::Duration::days(days_ago)
    }

    #[test]
    fn no_eviction_under_cap() {
        let existing = vec![(ts(3), "a".into()), (ts(1), "b".into())];
        assert!(pick_evictions(&existing, 2).is_empty());
    }

    #[test]
    fn evicts_oldest_first() {
        let existing = vec![
            (ts(1), "new".into()),
            (ts(9), "oldest".into()),
            (ts(5), "mid".into()),
        ];
        assert_eq!(pick_evictions(&existing, 2), vec!["oldest".to_string()]);
    }

    #[test]
    fn evicts_multiple_when_far_over_cap() {
        let existing: Vec<_> = (0..5)
            .map(|i| (ts(i), format!("p{i}")))
            .collect();
        // ts(i) = now - i days → p4 is the oldest, p0 the newest. Cap 2 →
        // evict p4, p3, p2 in that order (oldest first).
        assert_eq!(
            pick_evictions(&existing, 2),
            vec!["p4".to_string(), "p3".to_string(), "p2".to_string()]
        );
    }

    #[test]
    fn empty_branch_needs_no_eviction() {
        assert!(pick_evictions(&[], 2).is_empty());
    }

    #[test]
    fn cap_env_override() {
        // Safe restore around the process-global env.
        let prev = std::env::var("VIBE_MAX_PROJECTS_PER_KIND").ok();
        std::env::set_var("VIBE_MAX_PROJECTS_PER_KIND", "1");
        assert_eq!(max_projects_per_kind(), 1);
        std::env::set_var("VIBE_MAX_PROJECTS_PER_KIND", "0");
        assert_eq!(max_projects_per_kind(), 2, "cap never below 1");
        match prev {
            Some(v) => std::env::set_var("VIBE_MAX_PROJECTS_PER_KIND", v),
            None => std::env::remove_var("VIBE_MAX_PROJECTS_PER_KIND"),
        }
    }
}
