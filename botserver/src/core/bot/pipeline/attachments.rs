//! Staged chat attachment detection (#1357 follow-up).
//!
//! Both media producers stage the upload in the bot's Drive `inbox/` before
//! the model sees the message, but they mark it differently:
//! - web (`pipeline::exec`) appends
//!   `[User attached a file stored at inbox/x.jpg; …]` after the typed text;
//! - channel adapters (`bottelegram`/`botwhatsapp` media) send
//!   `[image] inbox/x.jpg` with the caption on the following line.
//!
//! Split out of `tool_exec.rs` to keep every pipeline file under the 450-line
//! repository limit.

/// A file staged in the bot's Drive `inbox/` by the current user message.
pub struct StagedAttachment {
    pub path: String,
    pub caption: String,
}

/// Extracts the staged attachment carried by the current user message, in
/// either producer format. Returns None when the message stages nothing; a
/// marker whose path is not an `inbox/` object (e.g. "(not stored: …)")
/// counts as no attachment so a failed upload never reaches a filing tool.
pub fn find_staged_attachment(user_text: &str) -> Option<StagedAttachment> {
    const WEB_MARKER: &str = "[User attached a file stored at ";
    if let Some(start) = user_text.find(WEB_MARKER) {
        let rest = &user_text[start + WEB_MARKER.len()..];
        let end = rest.find(';').unwrap_or(rest.len());
        let path = rest[..end].trim().to_string();
        if !path.starts_with("inbox/") {
            return None;
        }
        let caption = user_text[..start].trim().to_string();
        return Some(StagedAttachment { path, caption });
    }
    const CHANNEL_MARKERS: [&str; 6] = [
        "[image] ", "[document] ", "[voice] ", "[audio] ", "[video] ", "[sticker] ",
    ];
    for marker in CHANNEL_MARKERS {
        let Some(start) = user_text.find(marker) else {
            continue;
        };
        let rest = &user_text[start + marker.len()..];
        let mut parts = rest.splitn(2, char::is_whitespace);
        let path = parts.next().unwrap_or("").trim().to_string();
        if !path.starts_with("inbox/") {
            continue;
        }
        // The caption is whatever the sender typed after the path (the
        // producers emit "[kind] path\ncaption").
        let caption = parts.next().unwrap_or("").trim().to_string();
        return Some(StagedAttachment { path, caption });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::find_staged_attachment;

    /// Telegram photo with caption: `bottelegram::media::marker` emits
    /// `[image] inbox/…` followed by the caption on the next line.
    #[test]
    fn channel_marker_with_caption_yields_path_and_caption() {
        let text = "[image] inbox/40d1b057.jpg\nrecibo do pagamento";
        let staged = find_staged_attachment(text).expect("attachment must be found");
        assert_eq!(staged.path, "inbox/40d1b057.jpg");
        assert_eq!(staged.caption, "recibo do pagamento");
    }

    /// A bare marker without caption — the observed Telegram case where the
    /// model called classify_media with `{}` — must still stage the path.
    #[test]
    fn channel_marker_without_caption_yields_empty_caption() {
        let text = "[image] inbox/40d1b057-505b-4042-813a-c084196d323e.jpg";
        let staged = find_staged_attachment(text).expect("attachment must be found");
        assert_eq!(staged.path, "inbox/40d1b057-505b-4042-813a-c084196d323e.jpg");
        assert_eq!(staged.caption, "");
    }

    /// Every channel adapter marker kind stages its file.
    #[test]
    fn every_channel_marker_kind_is_recognized() {
        for (marker, path) in [
            ("[document] ", "inbox/contrato.pdf"),
            ("[voice] ", "inbox/nota.ogg"),
            ("[audio] ", "inbox/podcast.mp3"),
            ("[video] ", "inbox/clip.mp4"),
            ("[sticker] ", "inbox/figurinha.webp"),
        ] {
            let text = format!("{marker}{path}");
            let staged = find_staged_attachment(&text)
                .unwrap_or_else(|| panic!("{marker} must be recognized"));
            assert_eq!(staged.path, path);
            assert_eq!(staged.caption, "");
        }
    }

    /// Web attachments keep their own marker semantics: caption is the text
    /// typed before the marker, path ends at the semicolon.
    #[test]
    fn web_marker_keeps_caption_and_stops_at_semicolon() {
        let text = "arquiva isso\n[User attached a file stored at inbox/foto.png; when a media filing tool such as classify_media is available for this session, call it with path=inbox/foto.png]";
        let staged = find_staged_attachment(text).expect("attachment must be found");
        assert_eq!(staged.path, "inbox/foto.png");
        assert_eq!(staged.caption, "arquiva isso");
    }

    /// "(not stored: …)" means the upload itself failed; a filing tool must
    /// never receive such a path.
    #[test]
    fn failed_upload_marker_stages_nothing() {
        let text = "[image] (not stored: download failed)";
        assert!(find_staged_attachment(text).is_none());
    }

    /// Plain conversation text must not fabricate an attachment.
    #[test]
    fn plain_text_stages_nothing() {
        assert!(find_staged_attachment("bom dia, tudo bem?").is_none());
        assert!(find_staged_attachment("").is_none());
    }

    /// The caption can carry a colon or bracketed words without being
    /// mistaken for a second marker.
    #[test]
    fn caption_with_bracketed_words_stays_the_caption() {
        let text = "[video] inbox/a.mp4\nnota: [urgente] revisar amanha";
        let staged = find_staged_attachment(text).expect("attachment must be found");
        assert_eq!(staged.path, "inbox/a.mp4");
        assert_eq!(staged.caption, "nota: [urgente] revisar amanha");
    }
}
