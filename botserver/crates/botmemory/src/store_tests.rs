//! Tests for the user-memory store.
//!
//! Split out of `store.rs` to keep that file inside the project size
//! limit; the queries and dedupe contract stay beside their callers.


use super::*;

fn candidate(content: &str, confidence: f32) -> DedupeCandidate {
    DedupeCandidate::build(Uuid::new_v4(), content, confidence, Utc::now())
}

#[test]
fn normalized_duplicate_is_skipped() {
    let existing = vec![candidate("Prefers   dark mode.", 0.9)];
    let decision =
        decide_dedupe(&existing, &normalize_content("prefers dark mode"), "prefers dark mode", 0.5);
    assert_eq!(decision, DedupeDecision::SkipDuplicate { candidate: existing[0].id });
}

#[test]
fn identical_lowercase_supersedes_on_higher_confidence() {
    let existing = vec![candidate("Likes tea", 0.5)];
    let supersede = decide_dedupe(&existing, "likes tea", "Likes Tea", 0.9);
    assert_eq!(supersede, DedupeDecision::Supersede { candidate: existing[0].id });

    let skipped = decide_dedupe(&existing, "likes tea", "LIKES TEA", 0.3);
    assert_eq!(skipped, DedupeDecision::SkipLowerConfidence { candidate: existing[0].id });
}

#[test]
fn distinct_content_is_fresh() {
    let decision =
        decide_dedupe(&[candidate("Drives a truck", 0.8)], "flies planes", "Flies planes", 0.6);
    assert_eq!(decision, DedupeDecision::Fresh);
}
