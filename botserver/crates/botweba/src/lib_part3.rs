use super::*;

pub(super) fn render_html(app: &WebApp, content: &str) -> String {
    let meta_tags: String = app
        .config
        .meta_tags
        .iter()
        .map(|(k, v)| format!("<meta name=\"{k}\" content=\"{v}\">"))
        .collect::<Vec<_>>()
        .join("\n    ");

    let scripts: String = app
        .config
        .scripts
        .iter()
        .map(|s| format!("<script src=\"{s}\"></script>"))
        .collect::<Vec<_>>()
        .join("\n    ");

    let styles: String = app
        .config
        .styles
        .iter()
        .map(|s| format!("<link rel=\"stylesheet\" href=\"{s}\">"))
        .collect::<Vec<_>>()
        .join("\n    ");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    {}
    {}
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }}
    </style>
</head>
<body>
    {}
    {}
</body>
</html>"#,
        app.name, meta_tags, styles, content, scripts
    )
}
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
pub fn init() {
    log::info!("WEBA module initialized");
}
