
pub(super) fn classify_file(key: &str) -> &'static str {
    let lower = key.to_lowercase();
    if lower.ends_with(".bas") {
        "bas"
    } else if lower.contains(".gbkb/") && is_kb_extension(key) {
        "kb"
    } else if lower.contains(".gbot/") && lower.ends_with("config.csv") {
        "config"
    } else if lower.contains(".gbot/") && (lower.ends_with("prompt.md") || lower.ends_with("prompt.txt")) {
        "prompt"
    } else {
        "other"
    }
}
pub(super) fn is_kb_extension(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.ends_with(".txt")
    || lower.ends_with(".md")
    || lower.ends_with(".pdf")
    || lower.ends_with(".xlsx")
    || lower.ends_with(".xls")
    || lower.ends_with(".docx")
    || lower.ends_with(".doc")
    || lower.ends_with(".csv")
    || lower.ends_with(".pptx")
    || lower.ends_with(".ppt")
    || lower.ends_with(".html")
    || lower.ends_with(".htm")
    || lower.ends_with(".rtf")
    || lower.ends_with(".epub")
    || lower.ends_with(".xml")
    || lower.ends_with(".json")
    || lower.ends_with(".odt")
    || lower.ends_with(".ods")
    || lower.ends_with(".odp")
    || lower.ends_with(".yaml")
    || lower.ends_with(".yml")
    || lower.ends_with(".toml")
    || lower.ends_with(".jsonl")
    || lower.ends_with(".tsv")
    || lower.ends_with(".log")
    || lower.ends_with(".cfg")
    || lower.ends_with(".ini")
    || lower.ends_with(".conf")
    || lower.ends_with(".env")
    || lower.ends_with(".svg")
    || lower.ends_with(".py")
    || lower.ends_with(".rs")
    || lower.ends_with(".js")
    || lower.ends_with(".ts")
    || lower.ends_with(".sh")
    || lower.ends_with(".sql")
    || lower.ends_with(".css")
    || lower.ends_with(".ics")
    || lower.ends_with(".vcf")
    || lower.ends_with(".eml")
    || lower.ends_with(".rst")
    || lower.ends_with(".adoc")
    || lower.ends_with(".properties")
    || lower.ends_with(".graphql")
    || lower.ends_with(".proto")
}
