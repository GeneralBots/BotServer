use super::*;

impl PlayOptions {
    pub fn from_string(options_str: &str) -> Self {
        let mut opts = Self {
            controls: true,
            ..Self::default()
        };

        for opt in options_str.split(',').map(|s| s.trim().to_lowercase()) {
            match opt.as_str() {
                "autoplay" => opts.autoplay = true,
                "loop" => opts.loop_content = true,
                "fullscreen" => opts.fullscreen = true,
                "muted" => opts.muted = true,
                "nocontrols" => opts.controls = false,
                "linenumbers" => opts.line_numbers = Some(true),
                _ => {
                    if let Some((key, value)) = opt.split_once('=') {
                        match key {
                            "start" => opts.start_time = value.parse().ok(),
                            "end" => opts.end_time = value.parse().ok(),
                            "width" => opts.width = value.parse().ok(),
                            "height" => opts.height = value.parse().ok(),
                            "theme" => opts.theme = Some(value.to_string()),
                            "slide" => opts.slide = value.parse().ok(),
                            "page" => opts.page = value.parse().ok(),
                            "zoom" => opts.zoom = value.parse().ok(),
                            "highlight" => {
                                opts.highlight_lines =
                                    Some(value.split('-').filter_map(|s| s.parse().ok()).collect());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        opts
    }
}
