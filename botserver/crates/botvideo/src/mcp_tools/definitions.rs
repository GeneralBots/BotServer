//! MCP tool JSON schemas advertised to the model.

pub fn get_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "create_video_project",
            "description": "Create a new video editing project",
            "input_schema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name of the video project" },
                    "description": { "type": "string", "description": "Optional description of the project" },
                    "resolution_width": { "type": "integer", "description": "Video width in pixels (default: 1920)" },
                    "resolution_height": { "type": "integer", "description": "Video height in pixels (default: 1080)" },
                    "fps": { "type": "integer", "description": "Frames per second (default: 30)" }
                },
                "required": ["name"]
            }
        }),
        serde_json::json!({
            "name": "add_video_clip",
            "description": "Add a video clip to an existing project",
            "input_schema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "UUID of the project" },
                    "source_url": { "type": "string", "description": "URL or path to the video file" },
                    "name": { "type": "string", "description": "Optional name for the clip" },
                    "at_ms": { "type": "integer", "description": "Position in timeline (milliseconds)" },
                    "duration_ms": { "type": "integer", "description": "Duration of the clip (milliseconds)" }
                },
                "required": ["project_id", "source_url"]
            }
        }),
        serde_json::json!({
            "name": "generate_captions",
            "description": "Generate captions from audio transcription using AI",
            "input_schema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "UUID of the project" },
                    "style": { "type": "string", "description": "Caption style (default, bold, minimal)" },
                    "max_chars_per_line": { "type": "integer", "description": "Maximum characters per caption line" },
                    "font_size": { "type": "integer", "description": "Font size for captions" },
                    "color": { "type": "string", "description": "Text color (hex format)" },
                    "with_background": { "type": "boolean", "description": "Add background box behind captions" },
                    "language": { "type": "string", "description": "Language code for transcription" }
                },
                "required": ["project_id"]
            }
        }),
        serde_json::json!({
            "name": "export_video",
            "description": "Export a video project to a file, optionally saving to .gbdrive library",
            "input_schema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "UUID of the project" },
                    "format": { "type": "string", "description": "Output format (mp4, webm, mov)" },
                    "quality": { "type": "string", "description": "Quality preset (low, medium, high, 4k)" },
                    "save_to_library": { "type": "boolean", "description": "Save to .gbdrive/videos library (default: true)" }
                },
                "required": ["project_id"]
            }
        }),
        serde_json::json!({
            "name": "add_text_overlay",
            "description": "Add a text overlay to a video project",
            "input_schema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "UUID of the project" },
                    "content": { "type": "string", "description": "Text content to display" },
                    "at_ms": { "type": "integer", "description": "Start time in milliseconds" },
                    "duration_ms": { "type": "integer", "description": "Duration to display (milliseconds)" },
                    "x": { "type": "number", "description": "Horizontal position (0.0 to 1.0)" },
                    "y": { "type": "number", "description": "Vertical position (0.0 to 1.0)" },
                    "font_size": { "type": "integer", "description": "Font size in pixels" },
                    "color": { "type": "string", "description": "Text color (hex format)" }
                },
                "required": ["project_id", "content"]
            }
        }),
        serde_json::json!({
            "name": "add_audio_track",
            "description": "Add an audio track to a video project",
            "input_schema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "UUID of the project" },
                    "source_url": { "type": "string", "description": "URL or path to the audio file" },
                    "name": { "type": "string", "description": "Optional name for the track" },
                    "track_type": { "type": "string", "description": "Type of track (music, narration, sound_effect)" },
                    "start_ms": { "type": "integer", "description": "Start time in timeline (milliseconds)" },
                    "volume": { "type": "number", "description": "Volume level (0.0 to 1.0)" }
                },
                "required": ["project_id", "source_url"]
            }
        }),
    ]
}
