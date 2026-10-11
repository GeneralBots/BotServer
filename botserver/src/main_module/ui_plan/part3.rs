
/// Builds the system-prompt fragment teaching the LLM the `__ui_plan__`
/// contract. Includes the app catalog so the model only references apps
/// that actually exist in this build.
pub fn ui_automation_instructions() -> String {
    let apps: Vec<String> = crate::apps::registry::all_apps()
        .iter()
        .map(|a| format!("- {} (id: {}) — {}", a.title, a.id, a.description))
        .collect();
    let deep_links: Vec<String> = crate::apps::commands::APP_DEEP_LINKS
        .iter()
        .map(|(app_id, params)| {
            let keys = params
                .iter()
                .map(|p| format!("{} ({})", p.key, p.description))
                .collect::<Vec<_>>()
                .join(", ");
            format!("- {app_id}: {keys}")
        })
        .collect();
    let seq_hints: Vec<String> = crate::apps::commands::UI_SEQUENCE_HINTS
        .iter()
        .map(|(app_id, hint)| format!("- {app_id}: {hint}"))
        .collect();
    format!(
        r#"---
## Agentic UI Automation

You can drive the suite applications on behalf of the user. When the user
asks you to create, update, fill, or act on data inside an application,
respond with a single JSON object as the FIRST line of your reply, followed
by a short user-facing confirmation message.

Available applications:
{apps}

Contract — emit exactly:
{{"__ui_plan__": {{"app": "<app-id>", "steps": [
  {{"op": "open", "app": "<app-id>"}},
  {{"op": "click", "label": "<visible button/link text>"}},
  {{"op": "fill", "field": "<field label or name>", "value": "<value>"}},
  {{"op": "select", "field": "<field label>", "value": "<option>"}},
  {{"op": "cell", "cell": "<A1-style reference>", "value": "<value>"}},
  {{"op": "submit"}},
  {{"op": "wait", "ms": 500}}
]}}}}

Rules:
- Only use app ids from the list above; default to the app matching the user's request.
- `fill` values must be plain text without HTML.
- When unsure of a field name, use the label text shown in the form.
- Keep plans minimal: open, fill the requested fields, submit.
- For spreadsheet apps (sheet), write data with `cell` steps using A1-style references (e.g. "A1", "B2"). One `cell` step per cell.
- Plans may contain up to 200 steps; emit one step per cell/field, never truncate data.
- If the user asks to FIND something, use the search endpoint or apps.find instead: do not emit a plan, just answer normally.

### Contextual deep links to applications
When you reference a specific record that lives in an application (a person in
CRM, an invoice in billing, a file in drive, a product in products), include a
clickable deep link in your web reply so the user can open the app already
contextualized to that record. Use the exact markdown form:

  [<short action label>](app://<app-id>?<key>=<value>&<key2>=<value2>)

Examples:
- A person: `[Abrir ficha de Maria Silva](app://crm?person_id=123e4567-e89b-12d3-a456-426614174000)`
- An invoice: `[Ver fatura](app://billing?invoice_id=<billing-user-id>)`
- A product: `[Ver produto](app://products?product_id=<id>)`
- A file: `[Abrir no Drive](app://drive?path=faturas%2F2026-08)`

Per-app deep-link keys (use the key that matches the record you reference):

{deep_links}

Rules for deep links:
- Only emit links to apps from the list above and only for ids you actually
  returned in your answer (never invent an id).
- Put ONE deep link per referenced record, directly after it in the reply.
- This deep-link syntax is only rendered by the web desktop shell; on other
  channels the syntax is invisible to the user, so it is safe to include.

### UI navigation hints (web desktop only)
When planning `__ui_plan__` steps, follow the natural flow of the target app:

{seq_hints}

These hints describe how each app loads and where its actions live, so your
plan should navigate the app realistically instead of guessing buttons.
"#,
        apps = apps.join("\n"),
        deep_links = deep_links.join("\n"),
        seq_hints = seq_hints.join("\n"),
    )
}
