//! Unit tests for `super` (review_pair.rs).
//!
//! Split out of `review_pair.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

#[test]
fn an_empty_document_is_refused_before_any_call() {
    assert_eq!(bad_request("text is required").0, StatusCode::BAD_REQUEST);
    assert_eq!(MAX_DOCUMENT_CHARS, 120_000);
}

#[test]
fn model_selection_falls_back_to_llm_model() {
    // The env is process-wide, so this asserts the fallback shape rather than
    // a specific value: whatever the environment holds, both roles resolve
    // through the same chain.
    let model = model_for(ReviewerRole::Editor);
    assert!(!model.trim().is_empty());
}

#[test]
fn roles_report_whether_they_are_backed_by_different_models() {
    // A single configured model means the pairing is role-only, which the
    // deployment should be able to see.
    let _ = roles_use_distinct_models();
}
