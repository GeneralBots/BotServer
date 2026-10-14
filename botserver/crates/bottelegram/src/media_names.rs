//! Naming rules for ingested Telegram media.
//!
//! The generated name ends up inside a Drive object key, so it must never carry
//! a separator, a traversal sequence or an arbitrary extension.

use crate::media::InboundMedia;
use uuid::Uuid;

const MAX_FILE_NAME_LEN: usize = 80;

/// Builds the object name used inside the bot's `inbox/`: an opaque prefix keeps
/// every item unique, and documents keep their original stem so the filing tool
/// and the model can still classify by name.
pub(crate) fn stored_file_name(media: &InboundMedia) -> String {
    let provided = media.file_name.as_deref().and_then(sanitize_file_name);

    let extension = provided
        .as_deref()
        .and_then(extension_of)
        .or_else(|| media.mime_type.as_deref().and_then(extension_from_mime))
        .unwrap_or_else(|| default_extension(media.kind).to_string());

    let id = Uuid::new_v4();

    match (media.kind, provided.as_deref()) {
        ("document", Some(name)) => {
            let suffix = format!(".{extension}");
            let stem = name
                .strip_suffix(suffix.as_str())
                .unwrap_or(name)
                .trim_matches(|c: char| c == '.' || c == '-' || c == '_');

            if stem.is_empty() {
                format!("{id}.{extension}")
            } else {
                format!("{id}-{stem}.{extension}")
            }
        }
        _ => format!("{id}.{extension}"),
    }
}

/// Keeps only the base name and a conservative character set. Names that
/// sanitize to nothing are dropped so the caller falls back to the generated id.
fn sanitize_file_name(raw: &str) -> Option<String> {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .trim();

    let filtered: String = base
        .chars()
        .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .take(MAX_FILE_NAME_LEN)
        .collect();

    let trimmed = filtered.trim_matches(|c: char| c == '.' || c == '-' || c == '_');

    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn extension_of(name: &str) -> Option<String> {
    let (_, extension) = name.rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();

    if extension.is_empty()
        || extension.len() > 8
        || !extension.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }

    Some(extension)
}

fn extension_from_mime(mime: &str) -> Option<String> {
    let subtype = mime.split('/').nth(1)?;
    let subtype = subtype.split(';').next().unwrap_or(subtype);

    let canonical = match subtype {
        "jpeg" => "jpg",
        "plain" => "txt",
        "mpeg" => "mp3",
        "x-msvideo" => "avi",
        "quicktime" => "mov",
        "vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "vnd.ms-excel" => "xls",
        "vnd.ms-powerpoint" => "ppt",
        "msword" => "doc",
        other => other,
    };

    let candidate = canonical.to_ascii_lowercase();

    if candidate.is_empty()
        || candidate.len() > 8
        || !candidate.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }

    Some(candidate)
}

fn default_extension(kind: &str) -> &'static str {
    match kind {
        "image" => "jpg",
        "voice" => "ogg",
        "audio" => "mp3",
        "video" => "mp4",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(kind: &'static str, file_name: Option<&str>, mime_type: Option<&str>) -> InboundMedia {
        InboundMedia {
            kind,
            file_id: "file-id".to_string(),
            file_name: file_name.map(|name| name.to_string()),
            mime_type: mime_type.map(|mime| mime.to_string()),
            file_size: Some(1024),
        }
    }

    #[test]
    fn sanitize_keeps_only_the_base_name() {
        assert_eq!(
            sanitize_file_name("C:\\Users\\ana\\contrato.pdf"),
            Some("contrato.pdf".to_string())
        );
        assert_eq!(
            sanitize_file_name("../../etc/passwd"),
            Some("passwd".to_string())
        );
    }

    #[test]
    fn sanitize_drops_characters_outside_the_allowed_set() {
        assert_eq!(
            sanitize_file_name("nota fiscal 01.pdf"),
            Some("notafiscal01.pdf".to_string())
        );
    }

    #[test]
    fn sanitize_rejects_names_without_usable_characters() {
        assert_eq!(sanitize_file_name(".."), None);
        assert_eq!(sanitize_file_name("/"), None);
        assert_eq!(sanitize_file_name(""), None);
    }

    #[test]
    fn sanitize_caps_the_length() {
        let long = "a".repeat(MAX_FILE_NAME_LEN * 2);
        assert_eq!(
            sanitize_file_name(&long).map(|name| name.len()),
            Some(MAX_FILE_NAME_LEN)
        );
    }

    #[test]
    fn extension_of_reads_a_lowercase_suffix() {
        assert_eq!(extension_of("contrato.PDF"), Some("pdf".to_string()));
        assert_eq!(extension_of("arquivo"), None);
        assert_eq!(extension_of("arquivo."), None);
    }

    #[test]
    fn extension_from_mime_normalizes_common_types() {
        assert_eq!(extension_from_mime("image/jpeg"), Some("jpg".to_string()));
        assert_eq!(
            extension_from_mime(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            ),
            Some("docx".to_string())
        );
        assert_eq!(extension_from_mime("application/octet-stream"), None);
        assert_eq!(
            extension_from_mime("text/plain; charset=utf-8"),
            Some("txt".to_string())
        );
    }

    #[test]
    fn default_extension_covers_every_media_kind() {
        assert_eq!(default_extension("image"), "jpg");
        assert_eq!(default_extension("voice"), "ogg");
        assert_eq!(default_extension("audio"), "mp3");
        assert_eq!(default_extension("video"), "mp4");
        assert_eq!(default_extension("document"), "bin");
    }

    #[test]
    fn stored_name_keeps_the_original_stem_for_documents() {
        let name = stored_file_name(&media("document", Some("contrato.pdf"), None));
        assert!(name.ends_with("-contrato.pdf"), "unexpected name: {name}");
    }

    #[test]
    fn stored_name_for_images_is_opaque() {
        let name = stored_file_name(&media("image", None, Some("image/jpeg")));

        assert!(name.ends_with(".jpg"), "unexpected name: {name}");
        // UUID (36) plus the extension: no part of the Telegram file is reused.
        assert_eq!(name.len(), 40, "unexpected name: {name}");
    }

    #[test]
    fn stored_name_falls_back_to_the_mime_subtype() {
        let name = stored_file_name(&media("document", None, Some("application/pdf")));
        assert!(name.ends_with(".pdf"), "unexpected name: {name}");
    }

    #[test]
    fn stored_name_rejects_a_traversing_document_name() {
        let name = stored_file_name(&media("document", Some("../../evil.docx"), None));
        assert!(!name.contains('/'), "path separator survived: {name}");
        assert!(name.ends_with("-evil.docx"), "unexpected name: {name}");
    }
}
