use super::*;

impl ContentType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "mp4" | "webm" | "ogg" | "mov" | "avi" | "mkv" | "m4v" => Self::Video,

            "mp3" | "wav" | "flac" | "aac" | "m4a" | "wma" => Self::Audio,

            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" => Self::Image,

            "pptx" | "ppt" | "odp" | "key" => Self::Presentation,

            "docx" | "doc" | "odt" | "rtf" => Self::Document,

            "xlsx" | "xls" | "csv" | "ods" => Self::Spreadsheet,

            "pdf" => Self::Pdf,

            "rs" | "py" | "js" | "ts" | "java" | "c" | "cpp" | "h" | "go" | "rb" | "php"
            | "swift" | "kt" | "scala" | "r" | "sql" | "sh" | "bash" | "zsh" | "ps1" | "yaml"
            | "yml" | "toml" | "json" | "xml" | "bas" | "basic" => Self::Code,

            "md" | "markdown" => Self::Markdown,

            "html" | "htm" => Self::Html,
            _ => Self::Unknown,
        }
    }

    pub fn from_mime(mime: &str) -> Self {
        if mime.starts_with("video/") {
            Self::Video
        } else if mime.starts_with("audio/") {
            Self::Audio
        } else if mime.starts_with("image/") {
            Self::Image
        } else if mime == "application/pdf" {
            Self::Pdf
        } else if mime.contains("presentation") || mime.contains("powerpoint") {
            Self::Presentation
        } else if mime.contains("spreadsheet") || mime.contains("excel") {
            Self::Spreadsheet
        } else if mime.contains("document") || mime.contains("word") {
            Self::Document
        } else if mime.starts_with("text/") {
            if mime.contains("html") {
                Self::Html
            } else if mime.contains("markdown") {
                Self::Markdown
            } else {
                Self::Code
            }
        } else {
            Self::Unknown
        }
    }

    pub fn player_component(&self) -> &'static str {
        match self {
            Self::Video => "video-player",
            Self::Audio => "audio-player",
            Self::Image => "image-viewer",
            Self::Presentation => "presentation-viewer",
            Self::Document => "document-viewer",
            Self::Code => "code-viewer",
            Self::Spreadsheet => "spreadsheet-viewer",
            Self::Pdf => "pdf-viewer",
            Self::Markdown => "markdown-viewer",
            Self::Html => "html-viewer",
            Self::Iframe => "iframe-viewer",
            Self::Unknown => "generic-viewer",
        }
    }
}
