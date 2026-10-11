

/// Reduce a model reference to its bare filename. Template config.csv files
/// may store a legacy relative path (e.g. "../../../../data/llm/model.gguf")
/// that only resolves in the production layout; stripping the directories and
/// rejoining under the stack data dir makes it work everywhere.
pub(super) fn model_file_name(model: &str) -> &str {
    let model = model.trim();
    if model.is_empty() {
        return model;
    }
    model.rsplit(['/', '\\']).next().unwrap_or(model)
}
