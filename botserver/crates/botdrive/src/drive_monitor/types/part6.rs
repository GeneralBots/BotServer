// The only item here is feature-gated, so the glob must be as well.
#[cfg(any(feature = "research", feature = "llm"))]
use super::*;

#[cfg(any(feature = "research", feature = "llm"))]
pub(super) fn parse_kb_path(s3_key: &str) -> Option<KbPathParts> {
    let segments: Vec<&str> = s3_key.split('/').collect();
    let gbkb_idx = segments.iter().position(|s| s.ends_with(".gbkb"))?;
    if gbkb_idx + 1 >= segments.len() {
        return None;
    }
    let kb_name = segments[gbkb_idx + 1].to_string();
    let file_name = segments[gbkb_idx + 2..].join("/");
    if file_name.is_empty() {
        return None;
    }
    let relative_path = format!("{}/{}", kb_name, file_name);
    Some(KbPathParts {
        kb_name,
        file_name,
        relative_path,
    })
}
