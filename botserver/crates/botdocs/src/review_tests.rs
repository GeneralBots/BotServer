//! Unit tests for `super` (review.rs).
//!
//! Split out of `review.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn finding(severity: Severity, category: &str, summary: &str) -> Finding {
    Finding {
        severity,
        category: category.to_string(),
        summary: summary.to_string(),
        excerpt: String::new(),
    }
}

#[test]
fn a_clean_review_yields_no_findings() {
    let review = parse_review(
        ReviewerRole::Editor,
        r#"{"verdict":"looks good","findings":[]}"#,
    );
    assert!(review.findings.is_empty());
    assert_eq!(review.verdict, "looks good");
}

#[test]
fn a_fenced_reply_is_still_parsed() {
    let raw = "```json\n{\"verdict\":\"ok\",\"findings\":[{\"severity\":\"major\",\"category\":\"clarity\",\"summary\":\"The second paragraph contradicts the first.\"}]}\n```";
    let review = parse_review(ReviewerRole::Editor, raw);
    assert_eq!(review.findings.len(), 1);
    assert_eq!(review.findings[0].severity, Severity::Major);
    assert_eq!(review.findings[0].category, "clarity");
}

#[test]
fn an_unparseable_reply_produces_an_empty_review_not_a_panic() {
    let review = parse_review(ReviewerRole::Critic, "I think it is fine overall.");
    assert!(review.findings.is_empty());
    assert!(review.verdict.is_empty());
}

#[test]
fn a_severity_word_the_model_invents_does_not_become_critical() {
    assert_eq!(Severity::parse("catastrophic"), Severity::Minor);
    assert_eq!(Severity::parse("blocker"), Severity::Critical);
    assert_eq!(Severity::parse("HIGH"), Severity::Critical);
    assert_eq!(Severity::parse(""), Severity::Minor);
}

#[test]
fn a_finding_with_no_summary_is_dropped() {
    let raw = r#"{"findings":[{"severity":"minor","summary":"   "},{"severity":"minor","summary":"Real issue."}]}"#;
    let review = parse_review(ReviewerRole::Editor, raw);
    assert_eq!(review.findings.len(), 1);
}

#[test]
fn two_reviewers_finding_the_same_problem_produce_agreement_two() {
    let reviews = vec![
        Review {
            role: ReviewerRole::Editor,
            findings: vec![finding(Severity::Minor, "clarity", "The conclusion is unclear.")],
            verdict: String::new(),
        },
        Review {
            role: ReviewerRole::Critic,
            findings: vec![finding(Severity::Critical, "clarity", "The conclusion is unclear.")],
            verdict: String::new(),
        },
    ];
    let merged = merge_reviews(&reviews);
    assert_eq!(merged.findings.len(), 1, "the same complaint must merge");
    assert_eq!(merged.findings[0].agreement, 2);
    assert_eq!(merged.agreed, 1);
    assert_eq!(
        merged.findings[0].severity,
        Severity::Critical,
        "the more severe report wins"
    );
    assert_eq!(merged.findings[0].reviewers, vec![ReviewerRole::Editor, ReviewerRole::Critic]);
}

#[test]
fn different_problems_stay_separate() {
    let reviews = vec![
        Review {
            role: ReviewerRole::Editor,
            findings: vec![finding(Severity::Minor, "clarity", "The conclusion is unclear.")],
            verdict: String::new(),
        },
        Review {
            role: ReviewerRole::Critic,
            findings: vec![finding(Severity::Minor, "evidence", "No source is cited for the figure.")],
            verdict: String::new(),
        },
    ];
    assert_eq!(merge_reviews(&reviews).findings.len(), 2);
}

#[test]
fn wording_differences_still_merge() {
    let reviews = vec![
        Review {
            role: ReviewerRole::Editor,
            findings: vec![finding(Severity::Minor, "clarity", "The conclusion  is unclear.")],
            verdict: String::new(),
        },
        Review {
            role: ReviewerRole::Critic,
            findings: vec![finding(Severity::Minor, "clarity", "the conclusion is unclear")],
            verdict: String::new(),
        },
    ];
    assert_eq!(merge_reviews(&reviews).findings[0].agreement, 2);
}

#[test]
fn findings_are_ordered_by_severity_then_agreement() {
    let reviews = vec![
        Review {
            role: ReviewerRole::Editor,
            findings: vec![
                finding(Severity::Minor, "style", "Use fewer adverbs."),
                finding(Severity::Critical, "evidence", "The figure is unsourced."),
            ],
            verdict: String::new(),
        },
        Review {
            role: ReviewerRole::Critic,
            findings: vec![
                finding(Severity::Minor, "style", "Use fewer adverbs."),
                finding(Severity::Minor, "tone", "The opening is abrupt."),
            ],
            verdict: String::new(),
        },
    ];
    let merged = merge_reviews(&reviews);
    assert_eq!(merged.findings[0].severity, Severity::Critical);
    assert_eq!(merged.findings[1].severity, Severity::Minor);
    assert_eq!(merged.findings[1].agreement, 2, "the agreed minor outranks the lone one");
    assert_eq!(merged.findings[2].agreement, 1);
}

#[test]
fn the_rewrite_prompt_carries_every_correction() {
    let merged = merge_reviews(&[Review {
        role: ReviewerRole::Editor,
        findings: vec![finding(Severity::Critical, "evidence", "Cite the 2024 figure.")],
        verdict: String::new(),
    }]);
    let prompt = build_rewrite_prompt("Body text.", &merged.findings);
    assert!(prompt.contains("Cite the 2024 figure."), "{prompt}");
    assert!(prompt.contains("[critical]"), "{prompt}");
    assert!(prompt.contains("[VERIFY:"), "the rewriter must not invent facts");
}

#[test]
fn the_two_reviewers_are_told_to_stay_in_their_lane() {
    let editor = ReviewerRole::Editor.system_prompt();
    let critic = ReviewerRole::Critic.system_prompt();
    assert!(editor.contains("structure"), "{editor}");
    assert!(critic.contains("claims"), "{critic}");
    assert!(editor.contains("do not evaluate whether its claims are true"));
    assert!(critic.contains("do not comment on style"));
    assert_ne!(editor, critic);
}

#[test]
fn a_prompt_carries_the_document_and_the_instruction() {
    let prompt = build_prompt(ReviewerRole::Editor, "DOCUMENT BODY", "keep it short");
    assert!(prompt.contains("DOCUMENT BODY"));
    assert!(prompt.contains("keep it short"));
    assert!(prompt.contains("JSON only"));
}

#[test]
fn an_absent_instruction_adds_nothing() {
    let prompt = build_prompt(ReviewerRole::Editor, "BODY", "   ");
    assert!(!prompt.contains("Additional requirement"), "{prompt}");
}

#[test]
fn an_empty_pair_review_is_reported_rather_than_faked() {
    let merged = merge_reviews(&[]);
    assert!(merged.findings.is_empty());
    assert_eq!(merged.agreed, 0);
    assert!(merged.revised_text.is_empty());
}

#[test]
fn punctuation_and_case_do_not_defeat_deduplication() {
    assert_eq!(dedup_key("The conclusion is unclear."), dedup_key("the conclusion is unclear"));
    assert_eq!(dedup_key("Cite, the 2024 figure!"), dedup_key("cite the 2024 figure"));
}

#[test]
fn short_words_are_dropped_so_wording_carries_the_key() {
    // "the"/"is"/"a" carry no identity; keeping them would let a different
    // finding match just because it shares a function word.
    assert_eq!(dedup_key("the of and a"), "");
}

#[test]
fn a_fence_stripper_leaves_bare_json_alone() {
    assert_eq!(strip_fence("  {\"a\":1}  "), "{\"a\":1}");
    assert_eq!(strip_fence("```\n{\"a\":1}\n```"), "{\"a\":1}");
    assert_eq!(strip_fence("```json\n{\"a\":1}"), "{\"a\":1}", "an unclosed fence still parses");
}
