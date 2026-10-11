
#[cfg(feature = "kb-extraction")]
pub(super) fn extract_text_from_word_document(stream_data: &[u8]) -> String {
    if stream_data.len() < 10 {
        return String::new();
    }

    if let Some(offset_end) = stream_data.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]])) {
        if (offset_end as usize) < stream_data.len() && offset_end > 0 {
            let text_start = offset_end as usize;
            if let Some(ccb) = stream_data.get(2..4).map(|b| u16::from_le_bytes([b[0], b[1]])) {
                if ccb > 0 {
                    let piece_table_offset = text_start + ccb as usize;
                    if piece_table_offset < stream_data.len() {
                        return String::new();
                    }
                }
            }

            let raw_text: String = stream_data[text_start..]
                .chunks(2)
                .filter_map(|chunk| {
                    if chunk.len() >= 2 {
                        let code = u16::from_le_bytes([chunk[0], chunk[1]]);
                        char::from_u32(code as u32)
                    } else if chunk.len() == 1 {
                        char::from_u32(chunk[0] as u32)
                    } else {
                        None
                    }
                })
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
                .collect();

            let cleaned: String = raw_text
                .split('\r')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            if !cleaned.trim().is_empty() {
                return cleaned;
            }
        }
    }

    String::from_utf8_lossy(stream_data)
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ',' || *c == ';' || *c == ':' || *c == '!' || *c == '?')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
