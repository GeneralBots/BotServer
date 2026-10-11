//! Recall injection: assembles a token-budgeted context block from the
//! caller's visible memories, ranked by pin state, lexical overlap with the
//! query hint, then recency. All failures degrade to an empty block so the
//! chat path is never disrupted.

use diesel::PgConnection;
use uuid::Uuid;

use crate::models::UserMemory;
use crate::store;

pub const RECALL_ROW_LIMIT: i64 = 12;
const WORD_MIN_CHARS: usize = 2;
const LINE_CONTENT_MAX_CHARS: usize = 300;
const HEADER: &str = "Relevant user context:";

/// Counts how many query words occur in the lowercased content.
pub fn lexical_score(content_lowercase: &str, words: &[String]) -> usize {
    words.iter().filter(|word| content_lowercase.contains(word.as_str())).count()
}

fn query_words(query_hint: &str) -> Vec<String> {
    query_hint
        .split_whitespace()
        .map(|raw| {
            raw.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|word| word.chars().count() >= WORD_MIN_CHARS)
        .collect()
}

/// Orders rows: pinned first, then higher lexical score, then recency.
pub fn rank_rows(rows: Vec<UserMemory>, query_hint: &str) -> Vec<UserMemory> {
    let words = query_words(query_hint);
    let mut scored: Vec<(usize, UserMemory)> = rows
        .into_iter()
        .map(|row| (lexical_score(&row.content.to_lowercase(), &words), row))
        .collect();
    scored.sort_by(|(score_a, row_a), (score_b, row_b)| {
        row_b
            .pinned
            .cmp(&row_a.pinned)
            .then(score_b.cmp(score_a))
            .then(row_b.updated_at.cmp(&row_a.updated_at))
    });
    scored.into_iter().map(|(_, row)| row).collect()
}

/// Renders the ranked block within a character budget (approximated as four
/// characters per token). Returns an empty string when nothing fits.
pub fn assemble_block(rows: &[UserMemory], max_chars: usize) -> String {
    if rows.is_empty() || max_chars <= HEADER.chars().count() {
        return String::new();
    }

    let mut block = String::from(HEADER);
    let mut used = HEADER.chars().count();
    let mut appended = false;
    for row in rows {
        let clipped: String = row.content.chars().take(LINE_CONTENT_MAX_CHARS).collect();
        let line = format!("\n- [{}] {}", row.kind, clipped);
        let line_len = line.chars().count();
        if used + line_len <= max_chars {
            block.push_str(&line);
            used += line_len;
            appended = true;
            continue;
        }
        let remaining = max_chars - used;
        if remaining > 16 {
            let piece: String = line.chars().take(remaining).collect();
            block.push_str(&piece);
            appended = true;
        }
        break;
    }

    if appended {
        block
    } else {
        String::new()
    }
}

/// Builds the recall block for prompt injection. Owned memories are always
/// considered; branch-shared rows (`scope = 'branch'`) join when the branch
/// claim matches. Returns an empty string when there are no live rows.
pub fn recall_block(
    conn: &mut PgConnection,
    user_id: Uuid,
    branch_claim: Option<Uuid>,
    query_hint: &str,
    max_tokens: usize,
) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    if max_chars == 0 {
        return String::new();
    }

    let rows = match store::fetch_recent_visible(conn, user_id, branch_claim, RECALL_ROW_LIMIT) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("memory recall fetch failed for user {user_id}: {e}");
            return String::new();
        }
    };
    if rows.is_empty() {
        return String::new();
    }

    assemble_block(&rank_rows(rows, query_hint), max_chars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SCOPE_PRIVATE;
    use chrono::{Duration, Utc};

    fn memory(content: &str, kind: &str, pinned: bool, minutes_ago: i64) -> UserMemory {
        let updated_at = Utc::now() - Duration::minutes(minutes_ago);
        UserMemory {
            id: Uuid::new_v4(),
            org_id: None,
            branch_id: None,
            owner_user_id: Uuid::new_v4(),
            scope: SCOPE_PRIVATE.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            source: "manual".to_string(),
            confidence: 0.8,
            pinned,
            superseded_by: None,
            embedding_ref: None,
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn ranking_prefers_pin_then_score_then_recency() {
        let pinned_plain = memory("Enjoys hiking", "fact", true, 60);
        let old_match = memory("Invoice deadline is Friday", "task", false, 50);
        let fresh_match = memory("Invoice paid quarterly", "fact", false, 10);
        let unrelated = memory("Owns a red bicycle", "fact", false, 1);

        let ranked = rank_rows(
            vec![old_match.clone(), pinned_plain.clone(), unrelated, fresh_match.clone()],
            "invoice payment schedule",
        );

        assert_eq!(ranked[0].id, pinned_plain.id);
        assert_eq!(ranked[1].id, fresh_match.id);
        assert_eq!(ranked[2].id, old_match.id);
    }

    #[test]
    fn assembly_respects_character_budget() {
        let rows = vec![
            memory("First memory about invoicing workflow", "fact", true, 5),
            memory("Second memory about payroll cycles", "fact", false, 4),
        ];
        let full = assemble_block(&rows, 4000);
        assert!(full.starts_with(HEADER));
        assert!(full.contains("invoicing"));

        let tight_budget = HEADER.chars().count() + 20;
        let clipped = assemble_block(&rows, tight_budget);
        assert!(clipped.chars().count() <= tight_budget);

        assert_eq!(assemble_block(&[], 4000), "");
        assert_eq!(assemble_block(&rows, 4), "");
    }
}
