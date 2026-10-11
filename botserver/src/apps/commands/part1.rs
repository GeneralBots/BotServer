use super::*;

/// A single executable action exposed by an application.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct AppCommand {
    /// Owning app id (must match a registry `AppDefinition::id`).
    pub app: &'static str,
    /// Fully-qualified command name, e.g. `crm.people.search`.
    pub name: &'static str,
    /// Short, user-facing label shown in the palette / start menu.
    pub label: &'static str,
    /// One-line description injected into the LLM system prompt.
    pub summary: &'static str,
    /// Named parameters `(name, description)` — shown on demand via api.find.
    pub params: &'static [(&'static str, &'static str)],
    /// Optional deep-link template the command resolves to, e.g.
    /// `app://crm?person_id={person_id}`. `{<key>}` placeholders are replaced by
    /// the caller from resolved result values.
    pub deep_link: Option<&'static str>,
    /// When true only admin-role users may execute this command.
    pub admin_only: bool,
}
/// Deep-link parameter metadata for an application.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct DeepLinkParam {
    /// Query key, e.g. `person_id`.
    pub key: &'static str,
    /// What the record reference is.
    pub description: &'static str,
    /// Example value shown to the LLM.
    pub example: &'static str,
}
pub(crate) const fn cmd(
    app: &'static str,
    name: &'static str,
    label: &'static str,
    summary: &'static str,
    params: &'static [(&'static str, &'static str)],
    deep_link: Option<&'static str>,
    admin_only: bool,
) -> AppCommand {
    AppCommand {
        app,
        name,
        label,
        summary,
        params,
        deep_link,
        admin_only,
    }
}
