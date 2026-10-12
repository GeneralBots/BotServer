//! Persistence for `user_memories`: CRUD with deduplication by supersession,
//! filtered listing, pinning and export. Import lives in [`crate::import`].
//!
//! Dedupe contract within one owner+branch scope: content equal after
//! normalization (whitespace collapsed, lowercased) is skipped outright; a
//! case-insensitive match with differing spacing is resolved by confidence —
//! the incoming row supersedes the incumbent (recorded via `superseded_by`)
//! only when its confidence is greater than or equal to the incumbent's.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{
    ensure_content, ensure_kind, ensure_scope, ensure_source, SCOPE_BRANCH, UserMemory,
};
use crate::schema::user_memories;

pub const LIST_LIMIT: i64 = 100;

/// Resolution of an insert against existing rows.
#[derive(Debug, Clone, PartialEq)]
pub enum DedupeDecision {
    Fresh,
    SkipDuplicate { candidate: Uuid },
    SkipLowerConfidence { candidate: Uuid },
    Supersede { candidate: Uuid },
}

/// Result of [`insert`].
#[derive(Debug)]
pub enum DedupeOutcome {
    Created(UserMemory),
    Superseded { old_id: Uuid, memory: UserMemory },
    Skipped(UserMemory),
}

/// Compact comparison view of one active memory row.
#[derive(Debug, Clone)]
pub struct DedupeCandidate {
    pub id: Uuid,
    pub normalized: String,
    pub raw_lowercase: String,
    pub confidence: f32,
    pub updated_at: DateTime<Utc>,
}

impl DedupeCandidate {
    pub fn build(id: Uuid, content: &str, confidence: f32, updated_at: DateTime<Utc>) -> Self {
        Self {
            id,
            normalized: normalize_content(content),
            raw_lowercase: content.trim().to_lowercase(),
            confidence,
            updated_at,
        }
    }
}

pub(crate) fn candidate_of_row(row: &UserMemory) -> DedupeCandidate {
    DedupeCandidate::build(row.id, &row.content, row.confidence, row.updated_at)
}

/// Collapses whitespace runs, drops sentence punctuation and lowercases, so
/// that "Prefers   dark mode." and "prefers dark mode" are recognised as the
/// same memory.
pub fn normalize_content(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Resolves how new content relates to the active candidate set.
///
/// Normalized equality wins first and always skips. Otherwise the strongest
/// lowercase-identical incumbent (confidence, then recency) decides whether
/// the incoming row supersedes it.
pub fn decide_dedupe(
    candidates: &[DedupeCandidate],
    normalized: &str,
    raw_lowercase: &str,
    confidence: f32,
) -> DedupeDecision {
    // Strongest candidate whose normalised content matches the incoming memory.
    let mut duplicate: Option<&DedupeCandidate> = None;
    for candidate in candidates {
        if candidate.normalized != normalized {
            continue;
        }
        let replaces = match duplicate {
            None => true,
            Some(best) => {
                candidate.confidence > best.confidence
                    || (candidate.confidence == best.confidence
                        && candidate.updated_at > best.updated_at)
            }
        };
        if replaces {
            duplicate = Some(candidate);
        }
    }

    if let Some(best) = duplicate {
        // The same memory restated: skip it. When the wording is also identical
        // the incoming row is a revision, so a stronger confidence supersedes
        // the stored one instead of being discarded.
        if best.raw_lowercase.eq_ignore_ascii_case(raw_lowercase) {
            return if confidence > best.confidence {
                DedupeDecision::Supersede { candidate: best.id }
            } else {
                DedupeDecision::SkipLowerConfidence { candidate: best.id }
            };
        }
        return DedupeDecision::SkipDuplicate { candidate: best.id };
    }

    let mut incumbent: Option<&DedupeCandidate> = None;
    for candidate in candidates {
        if candidate.raw_lowercase != raw_lowercase {
            continue;
        }
        let replaces = match incumbent {
            None => true,
            Some(best) => {
                candidate.confidence > best.confidence
                    || (candidate.confidence == best.confidence
                        && candidate.updated_at > best.updated_at)
            }
        };
        if replaces {
            incumbent = Some(candidate);
        }
    }

    match incumbent {
        Some(best) if confidence >= best.confidence => DedupeDecision::Supersede { candidate: best.id },
        Some(best) => DedupeDecision::SkipLowerConfidence { candidate: best.id },
        None => DedupeDecision::Fresh,
    }
}

/// Validated input for a new memory row.
pub struct NewMemory<'a> {
    pub org_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub owner_user_id: Uuid,
    pub scope: &'a str,
    pub kind: &'a str,
    pub content: &'a str,
    pub source: &'a str,
    pub confidence: f32,
    pub pinned: bool,
}

pub(crate) fn fetch_candidates(
    conn: &mut PgConnection,
    owner: Uuid,
    branch: Option<Uuid>,
) -> Result<Vec<DedupeCandidate>, String> {
    use crate::schema::user_memories::dsl::*;
    user_memories
        .filter(owner_user_id.eq(owner))
        .filter(branch_id.eq(branch))
        .filter(superseded_by.is_null())
        .select((id, content, confidence, updated_at))
        .load::<(Uuid, String, f32, DateTime<Utc>)>(conn)
        .map(|rows| {
            rows.into_iter()
                .map(|(row_id, text, conf, seen)| DedupeCandidate::build(row_id, &text, conf, seen))
                .collect()
        })
        .map_err(|e| format!("memory candidates query failed: {e}"))
}

pub(crate) fn do_insert(conn: &mut PgConnection, spec: &NewMemory<'_>) -> Result<UserMemory, String> {
    let now = Utc::now();
    let row = UserMemory {
        id: Uuid::new_v4(),
        org_id: spec.org_id,
        branch_id: spec.branch_id,
        owner_user_id: spec.owner_user_id,
        scope: spec.scope.to_string(),
        kind: spec.kind.to_string(),
        content: spec.content.trim().to_string(),
        source: spec.source.to_string(),
        confidence: spec.confidence.clamp(0.0, 1.0),
        pinned: spec.pinned,
        superseded_by: None,
        embedding_ref: None,
        created_at: now,
        updated_at: now,
    };
    diesel::insert_into(user_memories::table)
        .values(&row)
        
        .get_result(conn)
        .map_err(|e| format!("memory insert failed: {e}"))
}

pub(crate) fn link_supersede(conn: &mut PgConnection, old_id: Uuid, new_id: Uuid) -> Result<(), String> {
    use crate::schema::user_memories::columns::*;
    let touched = diesel::update(user_memories::table.filter(id.eq(old_id)))
        .set((superseded_by.eq(Some(new_id)), updated_at.eq(Utc::now())))
        .execute(conn)
        .map_err(|e| format!("memory supersede failed: {e}"))?;
    if touched == 0 {
        return Err(format!("supersede target vanished: {old_id}"));
    }
    Ok(())
}

fn load_row(conn: &mut PgConnection, memory_id: Uuid) -> Result<UserMemory, String> {
    crate::schema::user_memories::dsl::user_memories
        .find(memory_id)
        .first::<UserMemory>(conn)
        .map_err(|e| format!("memory load failed: {e}"))
}

/// Inserts a memory applying the dedupe contract described at module level.
pub fn insert(conn: &mut PgConnection, spec: NewMemory<'_>) -> Result<DedupeOutcome, String> {
    let scope_clean = ensure_scope(spec.scope)?;
    let kind_clean = ensure_kind(spec.kind)?;
    let content_clean = ensure_content(spec.content)?;
    let source_clean = ensure_source(spec.source)?;
    let spec = NewMemory {
        org_id: spec.org_id,
        branch_id: spec.branch_id,
        owner_user_id: spec.owner_user_id,
        scope: &scope_clean,
        kind: &kind_clean,
        content: &content_clean,
        source: &source_clean,
        confidence: spec.confidence,
        pinned: spec.pinned,
    };

    let raw_lowercase = spec.content.trim().to_lowercase();
    let normalized = normalize_content(spec.content);
    let candidates = fetch_candidates(conn, spec.owner_user_id, spec.branch_id)?;

    match decide_dedupe(&candidates, &normalized, &raw_lowercase, spec.confidence) {
        DedupeDecision::Fresh => Ok(DedupeOutcome::Created(do_insert(conn, &spec)?)),
        DedupeDecision::SkipDuplicate { candidate }
        | DedupeDecision::SkipLowerConfidence { candidate } => {
            Ok(DedupeOutcome::Skipped(load_row(conn, candidate)?))
        }
        DedupeDecision::Supersede { candidate } => {
            let memory = do_insert(conn, &spec)?;
            link_supersede(conn, candidate, memory.id)?;
            Ok(DedupeOutcome::Superseded { old_id: candidate, memory })
        }
    }
}

/// Partial update payload; only present fields are applied.
#[derive(Debug, Default)]
pub struct MemoryPatch {
    pub kind: Option<String>,
    pub scope: Option<String>,
    pub content: Option<String>,
    pub pinned: Option<bool>,
}

/// Updates one owned memory. Returns `None` when no owned row matches.
pub fn update(
    conn: &mut PgConnection,
    memory_id: Uuid,
    owner: Uuid,
    patch: &MemoryPatch,
) -> Result<Option<UserMemory>, String> {
    use crate::schema::user_memories::dsl::*;

    if let Some(ref value) = patch.scope { ensure_scope(value)?; }
    if let Some(ref value) = patch.kind { ensure_kind(value)?; }
    if let Some(ref value) = patch.content { ensure_content(value)?; }

    let current: Option<UserMemory> = user_memories
        .filter(id.eq(memory_id))
        .filter(owner_user_id.eq(owner))
        .first::<UserMemory>(conn)
        .optional()
        .map_err(|e| format!("memory lookup failed: {e}"))?;
    let mut next = match current {
        Some(row) => row,
        None => return Ok(None),
    };

    if let Some(ref value) = patch.kind { next.kind = value.clone(); }
    if let Some(ref value) = patch.scope { next.scope = value.clone(); }
    if let Some(ref value) = patch.content { next.content = value.trim().to_string(); }
    if let Some(value) = patch.pinned { next.pinned = value; }
    next.updated_at = Utc::now();

    let updated: Option<UserMemory> = diesel::update(
        user_memories.filter(id.eq(memory_id)).filter(owner_user_id.eq(owner)),
    )
    .set(&next)
    
    .get_result(conn)
    .optional()
    .map_err(|e| format!("memory update failed: {e}"))?;
    Ok(updated)
}

/// Toggles the pin flag on one owned memory.
pub fn set_pinned(
    conn: &mut PgConnection,
    memory_id: Uuid,
    owner: Uuid,
    is_pinned: bool,
) -> Result<bool, String> {
    use crate::schema::user_memories::columns::*;
    let touched = diesel::update(
        crate::schema::user_memories::dsl::user_memories
            .filter(id.eq(memory_id))
            .filter(owner_user_id.eq(owner)),
    )
    .set((pinned.eq(is_pinned), updated_at.eq(Utc::now())))
    .execute(conn)
    .map_err(|e| format!("memory pin failed: {e}"))?;
    Ok(touched > 0)
}

/// Deletes one owned memory permanently. Returns whether a row was removed.
pub fn delete(conn: &mut PgConnection, memory_id: Uuid, owner: Uuid) -> Result<bool, String> {
    use crate::schema::user_memories::dsl::*;
    let touched = diesel::delete(
        user_memories.filter(id.eq(memory_id)).filter(owner_user_id.eq(owner)),
    )
    .execute(conn)
    .map_err(|e| format!("memory delete failed: {e}"))?;
    Ok(touched > 0)
}

/// Listing filters applied after the ownership visibility clause.
pub struct ListFilter<'a> {
    pub kind: Option<&'a str>,
    pub q: Option<&'a str>,
    pub scope: Option<&'a str>,
}

/// Lists visible memories: owned rows plus branch-shared rows (`scope =
/// 'branch'`) whose branch matches the caller claim. Filters by kind, free
/// text over content (ILIKE), and scope; page size is fixed at [`LIST_LIMIT`].
pub fn list(
    conn: &mut PgConnection,
    owner: Uuid,
    branch_claim: Option<Uuid>,
    filter: ListFilter<'_>,
    offset: i64,
) -> Result<Vec<UserMemory>, String> {
    use crate::schema::user_memories::dsl::*;

    let mut query = user_memories.filter(superseded_by.is_null()).into_boxed();
    query = match branch_claim {
        Some(claim) => query.filter(
            owner_user_id
                .eq(owner)
                .or(scope.eq(SCOPE_BRANCH).and(branch_id.eq(claim))),
        ),
        None => query.filter(owner_user_id.eq(owner)),
    };
    if let Some(kind_value) = filter.kind {
        query = query.filter(kind.eq(kind_value));
    }
    if let Some(scope_value) = filter.scope {
        query = query.filter(scope.eq(scope_value));
    }
    if let Some(term) = filter.q.map(str::trim).filter(|t| !t.is_empty()) {
        let neutralized = term.replace('%', "").replace('_', "");
        if !neutralized.is_empty() {
            query = query.filter(content.ilike(format!("%{neutralized}%")));
        }
    }

    query
        .order((pinned.desc(), updated_at.desc()))
        .limit(LIST_LIMIT)
        .offset(offset.max(0))
        .load::<UserMemory>(conn)
        .map_err(|e| format!("memory list failed: {e}"))
}

/// Fetches the newest visible memories, pinned first; used by recall.
pub(crate) fn fetch_recent_visible(
    conn: &mut PgConnection,
    owner: Uuid,
    branch_claim: Option<Uuid>,
    limit: i64,
) -> Result<Vec<UserMemory>, String> {
    use crate::schema::user_memories::dsl::*;
    let query = user_memories.filter(superseded_by.is_null()).into_boxed();
    let query = match branch_claim {
        Some(claim) => query.filter(
            owner_user_id
                .eq(owner)
                .or(scope.eq(SCOPE_BRANCH).and(branch_id.eq(claim))),
        ),
        None => query.filter(owner_user_id.eq(owner)),
    };
    query
        .order((pinned.desc(), updated_at.desc()))
        .limit(limit)
        .load::<UserMemory>(conn)
        .map_err(|e| format!("memory recall fetch failed: {e}"))
}

/// Exports every live memory owned by the caller. Superseded lineage rows are
/// internal history and therefore omitted from exports.
pub fn export_all(conn: &mut PgConnection, owner: Uuid) -> Result<Vec<UserMemory>, String> {
    use crate::schema::user_memories::dsl::*;
    user_memories
        .filter(owner_user_id.eq(owner))
        .filter(superseded_by.is_null())
        .order(updated_at.desc())
        .load::<UserMemory>(conn)
        .map_err(|e| format!("memory export failed: {e}"))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
