pub struct StarterSkill {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
    pub icon_glyph: &'static str,
    pub permissions: &'static [&'static str],
    pub prompts: &'static [&'static str],
    pub entry: &'static str,
    pub scripts: &'static [(&'static str, &'static str)],
}
pub fn starter_skills() -> Vec<StarterSkill> {
    vec![
        StarterSkill {
            slug: "expense-parser",
            name: "Expense Parser",
            description: "Captures expenses from chat and builds a monthly report.",
            tags: &["finance", "productivity"],
            icon_glyph: "\u{1F4B8}",
            permissions: &["table.read", "table.write"],
            prompts: &["expense-extract"],
            entry: "parse_expense.bas",
            scripts: &[
                (
                    "parse_expense.bas",
                    "HEAR \"Describe the expense\" AS expense\n\
                     LLM \"Extract amount and category from: {expense}\"\n\
                     SET amount = LLM_RESULT.amount\n\
                     SAVE {amount: amount, note: expense} TO expenses\n\
                     TALK \"Expense recorded: {amount}\"",
                ),
                (
                    "report_expenses.bas",
                    "items = GET FROM expenses WHERE created_at >= TODAY - 30\n\
                     IF COUNT(items) = 0 THEN\n\
                         TALK \"No expenses in the last 30 days.\"\n\
                     ELSE\n\
                         total = 0\n\
                         FOR EACH item IN items\n\
                             total = total + item.amount\n\
                         NEXT\n\
                         TALK \"{COUNT(items)} expenses totaling {total}.\"\n\
                     END IF",
                ),
            ],
        },
        StarterSkill {
            slug: "meeting-minutes",
            name: "Meeting Minutes",
            description: "Summarizes meetings and files the minutes as documents.",
            tags: &["office", "documents"],
            icon_glyph: "\u{1F4DD}",
            permissions: &["file.write", "email.send"],
            prompts: &["minutes-summary"],
            entry: "capture_minutes.bas",
            scripts: &[
                (
                    "capture_minutes.bas",
                    "HEAR \"Paste the meeting notes\" AS notes\n\
                     summary = LLM \"Summarize decisions and action items\"\n\
                     REMEMBER \"last_minutes\" = summary\n\
                     TALK \"Minutes ready:\n{summary}\"",
                ),
                (
                    "save_minutes.bas",
                    "summary = RECALL \"last_minutes\"\n\
                     CREATE FILE \"meetings/{TODAY}.md\" WITH summary\n\
                     SEND MAIL TO USER.email WITH subject \"Meeting minutes {TODAY}\", body summary\n\
                     TALK \"Minutes saved and emailed.\"",
                ),
            ],
        },
        StarterSkill {
            slug: "lead-qualifier",
            name: "Lead Qualifier",
            description: "Scores inbound leads by revenue and saves them to CRM tables.",
            tags: &["sales", "crm"],
            icon_glyph: "\u{1F3AF}",
            permissions: &["table.read", "table.write"],
            prompts: &["lead-score"],
            entry: "qualify_lead.bas",
            scripts: &[
                (
                    "qualify_lead.bas",
                    "HEAR \"Company name\" AS company\n\
                     HEAR \"Estimated annual revenue\" AS revenue\n\
                     score = LLM \"Score this lead 0 to 100 given revenue {revenue}\"\n\
                     SAVE {company: company, revenue: revenue, score: score} TO leads\n\
                     TALK \"Lead {company} scored {score}.\"",
                ),
                (
                    "list_hot_leads.bas",
                    "hot = GET FROM leads WHERE score >= 70\n\
                     IF COUNT(hot) = 0 THEN\n\
                         TALK \"No hot leads yet.\"\n\
                     ELSE\n\
                         FOR EACH lead IN hot\n\
                             TALK \"- {lead.company}: {lead.score}\"\n\
                         NEXT\n\
                     END IF",
                ),
            ],
        },
        StarterSkill {
            slug: "invoice-qa",
            name: "Invoice QA",
            description: "Detects anomalies in issued invoices and exports a review file.",
            tags: &["finance", "quality"],
            icon_glyph: "\u{1F4C4}",
            permissions: &["table.read", "file.write"],
            prompts: &["invoice-anomaly"],
            entry: "invoice_check.bas",
            scripts: &[
                (
                    "invoice_check.bas",
                    "suspects = GET FROM invoices WHERE total < 0 OR customer IS NULL\n\
                     IF COUNT(suspects) = 0 THEN\n\
                         TALK \"All invoices look healthy.\"\n\
                     ELSE\n\
                         FOR EACH inv IN suspects\n\
                             TALK \"Invoice {inv.number}: {inv.total} needs review\"\n\
                         NEXT\n\
                     END IF",
                ),
                (
                    "export_invoice_qa.bas",
                    "suspects = GET FROM invoices WHERE total < 0 OR customer IS NULL\n\
                     CREATE FILE \"exports/invoice_qa_{TODAY}.csv\" WITH suspects\n\
                     TALK \"Exported {COUNT(suspects)} invoices for review.\"",
                ),
            ],
        },
        StarterSkill {
            slug: "site-monitor",
            name: "Site Monitor",
            description: "Checks a website health endpoint and alerts on failure.",
            tags: &["ops", "monitoring"],
            icon_glyph: "\u{1F6A8}",
            permissions: &["http.request", "email.send"],
            prompts: &[],
            entry: "check_site.bas",
            scripts: &[
                (
                    "check_site.bas",
                    "HEAR \"URL to monitor\" AS url\n\
                     response = GET HTTP \"{url}/health\"\n\
                     REMEMBER \"last_status\" = response.status\n\
                     IF response.status == 200 THEN\n\
                         TALK \"Site is UP.\"\n\
                     ELSE\n\
                         TALK \"Site DOWN with status {response.status}.\"\n\
                     END IF",
                ),
                (
                    "notify_outage.bas",
                    "status = RECALL \"last_status\"\n\
                     IF status != 200 THEN\n\
                         SEND MAIL TO USER.email WITH subject \"Outage detected\", body \"Status: {status}\"\n\
                         TALK \"Alert email sent.\"\n\
                     END IF",
                ),
            ],
        },
        StarterSkill {
            slug: "email-digest",
            name: "Email Digest",
            description: "Compiles unread messages into a single daily digest.",
            tags: &["email", "productivity"],
            icon_glyph: "\u{2709}\u{FE0F}",
            permissions: &["table.read", "email.send"],
            prompts: &["digest-tone"],
            entry: "collect_digest.bas",
            scripts: &[
                (
                    "collect_digest.bas",
                    "unread = GET FROM emails WHERE unread = true\n\
                     digest = LLM \"Turn these messages into a short digest\"\n\
                     REMEMBER \"digest_body\" = digest\n\
                     TALK \"Digest covers {COUNT(unread)} messages.\"",
                ),
                (
                    "send_digest.bas",
                    "digest = RECALL \"digest_body\"\n\
                     SEND MAIL TO USER.email WITH subject \"Daily digest\", body digest\n\
                     TALK \"Digest sent.\"",
                ),
            ],
        },
        StarterSkill {
            slug: "kb-quizmaster",
            name: "KB Quizmaster",
            description: "Quizzes users on knowledge base topics and tracks scores.",
            tags: &["training", "kb"],
            icon_glyph: "\u{1F9E0}",
            permissions: &["kb.read", "table.write"],
            prompts: &["quiz-question", "quiz-grade"],
            entry: "quiz_ask.bas",
            scripts: &[
                (
                    "quiz_ask.bas",
                    "USE KB \"manual\"\n\
                     question = LLM \"Ask one quiz question from the manual\"\n\
                     REMEMBER \"current_question\" = question\n\
                     TALK \"{question}\"",
                ),
                (
                    "quiz_grade.bas",
                    "HEAR \"Your answer\" AS answer\n\
                     verdict = LLM \"Grade this answer against the manual: {answer}\"\n\
                     SAVE {question: RECALL(\"current_question\"), verdict: verdict} TO quiz_scores\n\
                     TALK \"{verdict}\"",
                ),
            ],
        },
        StarterSkill {
            slug: "social-drafter",
            name: "Social Drafter",
            description: "Drafts social media posts from a topic and publishes on approval.",
            tags: &["marketing", "social"],
            icon_glyph: "\u{1F4E2}",
            permissions: &["social.post"],
            prompts: &["post-draft"],
            entry: "draft_post.bas",
            scripts: &[
                (
                    "draft_post.bas",
                    "HEAR \"Topic for the post\" AS topic\n\
                     draft = LLM \"Write a short professional post about {topic}\"\n\
                     REMEMBER \"draft_post\" = draft\n\
                     TALK \"Draft:\\n{draft}\"",
                ),
                (
                    "post_approved.bas",
                    "draft = RECALL \"draft_post\"\n\
                     POST TO SOCIAL \"linkedin\" MESSAGE draft\n\
                     TALK \"Published to LinkedIn.\"",
                ),
            ],
        },
        StarterSkill {
            slug: "csv-cleaner",
            name: "CSV Cleaner",
            description: "Removes empty rows and duplicates from CSV files in Drive.",
            tags: &["data", "files"],
            icon_glyph: "\u{1F9F9}",
            permissions: &["file.read", "file.write"],
            prompts: &[],
            entry: "clean_csv.bas",
            scripts: &[
                (
                    "clean_csv.bas",
                    "rows = READ FILE \"data.csv\"\n\
                     clean = LLM \"Remove empty and duplicated lines, keep header\"\n\
                     WRITE FILE \"data_clean.csv\" WITH clean\n\
                     TALK \"Cleaned file saved as data_clean.csv.\"",
                ),
                (
                    "summary_clean.bas",
                    "clean = READ FILE \"data_clean.csv\"\n\
                     TALK \"Result has {COUNT(clean)} lines.\"",
                ),
            ],
        },
        StarterSkill {
            slug: "webhook-fanout",
            name: "Webhook Fanout",
            description: "Delivers an event payload to every registered webhook endpoint.",
            tags: &["integrations", "webhooks"],
            icon_glyph: "\u{1F500}",
            permissions: &["http.request", "table.write"],
            prompts: &[],
            entry: "fanout.bas",
            scripts: &[
                (
                    "fanout.bas",
                    "endpoints = GET FROM webhook_endpoints WHERE active = true\n\
                     FOR EACH ep IN endpoints\n\
                         WEBHOOK \"{ep.url}\" WITH {event: \"fanout\", sent_at: NOW}\n\
                     NEXT\n\
                     TALK \"Event delivered to {COUNT(endpoints)} endpoints.\"",
                ),
                (
                    "fanout_log.bas",
                    "SAVE {delivered_at: NOW, target_count: COUNT(endpoints)} TO webhook_logs\n\
                     TALK \"Fanout logged.\"",
                ),
            ],
        },
    ]
}
