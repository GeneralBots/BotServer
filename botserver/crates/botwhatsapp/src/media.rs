//! WhatsApp inbound media ingestion.
//!
//! Images, videos, audio notes, documents and stickers are fetched through the
//! Graph API and stored in the bot's Drive `inbox/` directory. The conversation
//! then carries an `[image] inbox/...` or `[document] inbox/...` marker so a
//! BASIC script can hand the path to `CLASSIFY IMAGE`, `GET` or a filing tool.

use log::{info, warn};
use std::sync::Arc;
use uuid::Uuid;

use crate::media_names::stored_file_name;
use crate::models::WhatsAppMessage;
use crate::state::WhatsAppState;
use crate::utils::extract_message_text;

/// Files larger than 20 MB are rejected, aligned with the Telegram flow.
pub const MAX_DOWNLOAD_BYTES: usize = 20 * 1024 * 1024;

/// Directory inside `{bot}.gbdrive` where inbound media is staged before a
/// filing tool moves it into its final folder.
const INBOX_PREFIX: &str = "inbox";

/// Default Graph API base used when `whatsapp_api_url` is not configured.
const DEFAULT_GRAPH_URL: &str = "https://graph.facebook.com/v21.0";

/// Describes the media carried by one WhatsApp webhook message.
pub(crate) struct InboundMedia {
    pub(crate) kind: &'static str,
    pub(crate) media_id: String,
    pub(crate) file_name: Option<String>,
    pub(crate) mime_type: Option<String>,
    pub(crate) caption: Option<String>,
}

/// Builds the conversation content for a message, fetching any media into Drive
/// first. Falls back to the plain text extraction when the message carries no
/// attachment.
pub async fn message_content(
    state: &Arc<WhatsAppState>,
    bot_id: Uuid,
    message: &WhatsAppMessage,
) -> String {
    let Some(media) = select_media(message) else {
        return extract_message_text(message).unwrap_or_default();
    };

    let caption = media.caption.as_deref().unwrap_or("").trim();

    match store_media(state, bot_id, &media).await {
        Ok(path) => {
            info!(
                "Stored WhatsApp {} for bot {bot_id} at {path}",
                media.kind
            );
            marker(media.kind, &path, caption)
        }
        Err(reason) => {
            warn!(
                "WhatsApp {} for bot {bot_id} not stored: {reason}",
                media.kind
            );
            marker(media.kind, &format!("(not stored: {reason})"), caption)
        }
    }
}

pub(crate) fn select_media(message: &WhatsAppMessage) -> Option<InboundMedia> {
    if let Some(image) = message.image.as_ref() {
        let media_id = image.id.clone()?;
        return Some(InboundMedia {
            kind: "image",
            media_id,
            file_name: None,
            mime_type: image.mime_type.clone(),
            caption: image.caption.clone(),
        });
    }

    if let Some(video) = message.video.as_ref() {
        let media_id = video.id.clone()?;
        return Some(InboundMedia {
            kind: "video",
            media_id,
            file_name: video.filename.clone(),
            mime_type: video.mime_type.clone(),
            caption: video.caption.clone(),
        });
    }

    if let Some(audio) = message.audio.as_ref() {
        let media_id = audio.id.clone()?;
        return Some(InboundMedia {
            kind: "audio",
            media_id,
            file_name: None,
            mime_type: audio.mime_type.clone(),
            caption: None,
        });
    }

    if let Some(document) = message.document.as_ref() {
        let media_id = document.id.clone()?;
        return Some(InboundMedia {
            kind: "document",
            media_id,
            file_name: document.filename.clone(),
            mime_type: document.mime_type.clone(),
            caption: document.caption.clone(),
        });
    }

    if let Some(sticker) = message.sticker.as_ref() {
        let media_id = sticker.id.clone()?;
        return Some(InboundMedia {
            kind: "sticker",
            media_id,
            file_name: None,
            mime_type: sticker.mime_type.clone(),
            caption: None,
        });
    }

    None
}

async fn store_media(
    state: &Arc<WhatsAppState>,
    bot_id: Uuid,
    media: &InboundMedia,
) -> Result<String, String> {
    let bytes = download_media(state, &media.media_id)
        .await
        .map_err(|e| format!("download failed: {e}"))?;

    if bytes.is_empty() {
        return Err("WhatsApp returned an empty file".to_string());
    }

    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err(format!(
            "larger than the {MAX_DOWNLOAD_BYTES} byte download limit"
        ));
    }

    let relative_path = format!("{INBOX_PREFIX}/{}", stored_file_name(media));
    let content_type = media
        .mime_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());

    (state.put_media)(bot_id, relative_path, bytes, Some(content_type)).await
}

/// Resolves the media id into bytes via the Graph API: a first call returns the
/// media `url`, a second call over that url carries the file itself. Both are
/// authenticated with the `whatsapp-api-key` bearer token.
async fn download_media(state: &Arc<WhatsAppState>, media_id: &str) -> Result<Vec<u8>, String> {
    let api_url = (state.get_config)("whatsapp_api_url")
        .unwrap_or_else(|_| DEFAULT_GRAPH_URL.to_string())
        .trim_end_matches('/')
        .to_string();
    let token = (state.secrets)("whatsapp_api_key")
        .map_err(|_| "whatsapp-api-key not configured".to_string())?;

    let client = reqwest::Client::new();

    let metadata = client
        .get(format!("{api_url}/{media_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Graph API media lookup failed: {e}"))?;

    let meta_status = metadata.status();
    if !meta_status.is_success() {
        return Err(format!(
            "Graph API media lookup failed with status {meta_status}"
        ));
    }

    let meta: serde_json::Value = metadata
        .json()
        .await
        .map_err(|e| format!("Graph API media metadata is not JSON: {e}"))?;

    let url = meta
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Graph API returned no media url".to_string())?;

    let file = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("media download failed: {e}"))?;

    let status = file.status();
    if !status.is_success() {
        return Err(format!("media download failed with status {status}"));
    }

    let bytes = file
        .bytes()
        .await
        .map_err(|e| format!("media body read failed: {e}"))?
        .to_vec();

    info!(
        "Downloaded WhatsApp media {media_id}: {} bytes",
        bytes.len()
    );

    Ok(bytes)
}

fn marker(kind: &str, target: &str, caption: &str) -> String {
    if caption.is_empty() {
        format!("[{kind}] {target}")
    } else {
        format!("[{kind}] {target}\n{caption}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AudioContent, DocumentContent, ImageContent};

    fn message_with_image(image: Option<ImageContent>) -> WhatsAppMessage {
        WhatsAppMessage {
            from: None,
            id: None,
            timestamp: None,
            message_type: Some("image".to_string()),
            text: None,
            image,
            video: None,
            audio: None,
            document: None,
            sticker: None,
            interactive: None,
            button: None,
        }
    }

    fn image(id: &str, mime: &str) -> ImageContent {
        ImageContent {
            id: Some(id.to_string()),
            mime_type: Some(mime.to_string()),
            caption: None,
        }
    }

    fn media(kind: &'static str, id: &str, caption: Option<&str>) -> InboundMedia {
        InboundMedia {
            kind,
            media_id: id.to_string(),
            file_name: Some("arquivo.pdf".to_string()),
            mime_type: Some("application/pdf".to_string()),
            caption: caption.map(|c| c.to_string()),
        }
    }

    #[test]
    fn image_message_selects_the_image() {
        let message = message_with_image(Some(image("media-1", "image/jpeg")));
        let selected = select_media(&message);

        assert_eq!(selected.as_ref().map(|m| m.kind), Some("image"));
        assert_eq!(
            selected.as_ref().map(|m| m.media_id.clone()),
            Some("media-1".to_string())
        );
    }

    #[test]
    fn audio_message_selects_the_audio() {
        let mut message = message_with_image(None);
        message.message_type = Some("audio".to_string());
        message.audio = Some(AudioContent {
            id: Some("audio-1".to_string()),
            mime_type: Some("audio/ogg; codecs=opus".to_string()),
        });

        let selected = select_media(&message).unwrap();
        assert_eq!(selected.kind, "audio");
        assert_eq!(selected.media_id, "audio-1");
    }

    #[test]
    fn media_without_id_selects_nothing() {
        let message = message_with_image(Some(ImageContent {
            id: None,
            mime_type: None,
            caption: None,
        }));

        assert!(select_media(&message).is_none());
    }

    #[test]
    fn text_only_message_selects_no_media() {
        let mut message = message_with_image(None);
        message.message_type = Some("text".to_string());
        message.text = Some(crate::models::TextContent {
            body: Some("bom dia".to_string()),
        });

        assert!(select_media(&message).is_none());
    }

    #[test]
    fn document_selects_the_document() {
        let mut message = message_with_image(None);
        message.message_type = Some("document".to_string());
        message.document = Some(DocumentContent {
            id: Some("doc-1".to_string()),
            mime_type: Some("application/pdf".to_string()),
            filename: Some("contrato.pdf".to_string()),
            caption: Some("nota fiscal".to_string()),
        });

        let selected = select_media(&message).unwrap();
        assert_eq!(selected.kind, "document");
        assert_eq!(selected.caption.as_deref(), Some("nota fiscal"));
    }

    #[test]
    fn marker_without_caption_is_a_single_line() {
        assert_eq!(marker("image", "inbox/a1.jpg", ""), "[image] inbox/a1.jpg");
    }

    #[test]
    fn marker_appends_the_caption() {
        assert_eq!(
            marker("document", "inbox/b2-contrato.pdf", "nota fiscal"),
            "[document] inbox/b2-contrato.pdf\nnota fiscal"
        );
    }

    #[test]
    fn marker_reports_media_that_could_not_be_stored() {
        let detail = media("audio", "audio-1", None);
        assert_eq!(
            marker(detail.kind, "(not stored: download failed)", ""),
            "[audio] (not stored: download failed)"
        );
    }
}