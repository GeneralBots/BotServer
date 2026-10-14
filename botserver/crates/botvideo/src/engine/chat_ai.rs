use crate::engine::VideoEngine;
use crate::models::*;
use crate::requests::*;
use crate::responses::*;
use crate::schema::*;

use diesel::prelude::*;
use tracing::info;
use uuid::Uuid;

impl VideoEngine {
    pub async fn auto_reframe(
        &self,
        project_id: Uuid,
        clip_id: Uuid,
        target_width: i32,
        target_height: i32,
        output_dir: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let clips = self.get_clips(project_id).await?;
        let clip = clips.iter().find(|c| c.id == clip_id).ok_or("Clip not found")?;

        let output_filename = format!("reframed_{}_{}.mp4", clip_id, target_width);
        let output_path = format!("{}/{}", output_dir, output_filename);

        let cmd = crate::safe_command::SafeCommand::new("ffmpeg")?
            .arg("-i")?
            .arg(&clip.source_url)?
            .arg("-vf")?
            .arg(format!(
                "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2",
                target_width, target_height, target_width, target_height
            ))?
            .arg("-c:a")?
            .arg("copy")?
            .arg(&output_path)?;

        let result = cmd.execute()?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(format!("Auto-reframe failed: {stderr}").into());
        }

        Ok(format!("/video/reframed/{}", output_filename))
    }

    pub fn get_available_templates(&self) -> Vec<TemplateInfo> {
        vec![
            TemplateInfo {
                id: "social-promo".to_string(),
                name: "Social Media Promo".to_string(),
                description: "Quick promo video with text animations".to_string(),
                thumbnail_url: "/templates/social-promo.jpg".to_string(),
                duration_ms: 15000,
                category: "social".to_string(),
            },
            TemplateInfo {
                id: "youtube-intro".to_string(),
                name: "YouTube Intro".to_string(),
                description: "Professional channel intro".to_string(),
                thumbnail_url: "/templates/youtube-intro.jpg".to_string(),
                duration_ms: 5000,
                category: "intro".to_string(),
            },
            TemplateInfo {
                id: "product-showcase".to_string(),
                name: "Product Showcase".to_string(),
                description: "Clean product presentation".to_string(),
                thumbnail_url: "/templates/product-showcase.jpg".to_string(),
                duration_ms: 30000,
                category: "business".to_string(),
            },
            TemplateInfo {
                id: "talking-head".to_string(),
                name: "Talking Head".to_string(),
                description: "Speaker with lower thirds".to_string(),
                thumbnail_url: "/templates/talking-head.jpg".to_string(),
                duration_ms: 60000,
                category: "presentation".to_string(),
            },
        ]
    }

    pub async fn apply_template(
        &self,
        project_id: Uuid,
        template_id: &str,
        customizations: Option<serde_json::Value>,
    ) -> Result<(), diesel::result::Error> {
        let custom = customizations.unwrap_or(serde_json::json!({}));
        let title = custom.get("title").and_then(|v| v.as_str()).unwrap_or("Title");

        match template_id {
            "social-promo" | "youtube-intro" => {
                self.add_layer(project_id, AddLayerRequest {
                    name: Some("Title".to_string()),
                    layer_type: "text".to_string(),
                    start_ms: Some(0),
                    end_ms: Some(3000),
                    x: Some(0.5),
                    y: Some(0.5),
                    width: Some(0.8),
                    height: Some(0.2),
                    properties: Some(serde_json::json!({
                        "content": title,
                        "font_size": 72,
                        "color": "#FFFFFF",
                        "text_align": "center",
                    })),
                }).await?;
            }
            _ => {}
        }

        info!("Applied template {} to project {}", template_id, project_id);
        Ok(())
    }

    pub async fn remove_background(
        &self,
        project_id: Uuid,
        clip_id: Uuid,
        replacement: Option<String>,
    ) -> Result<BackgroundRemovalResponse, Box<dyn std::error::Error + Send + Sync>> {
        let clips = self.get_clips(project_id).await?;
        let clip = clips.iter().find(|c| c.id == clip_id).ok_or("Clip not found")?;

        let botmodels_url = std::env::var("BOTMODELS_URL").unwrap_or_default();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/video/remove-background", botmodels_url))
            .json(&serde_json::json!({
                "video_url": &clip.source_url,
                "replacement": replacement,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Background removal failed: {error}").into());
        }

        let result: serde_json::Value = response.json().await?;

        Ok(BackgroundRemovalResponse {
            processed_url: result["processed_url"].as_str().unwrap_or("").to_string(),
            duration_ms: result["duration_ms"].as_i64().unwrap_or(0),
        })
    }

    pub async fn enhance_video(
        &self,
        project_id: Uuid,
        req: VideoEnhanceRequest,
    ) -> Result<VideoEnhanceResponse, Box<dyn std::error::Error + Send + Sync>> {
        let clips = self.get_clips(project_id).await?;
        let clip = clips.iter().find(|c| c.id == req.clip_id).ok_or("Clip not found")?;

        let botmodels_url = std::env::var("BOTMODELS_URL").unwrap_or_default();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/video/enhance", botmodels_url))
            .json(&serde_json::json!({
                "video_url": &clip.source_url,
                "upscale_factor": req.upscale_factor,
                "denoise": req.denoise,
                "stabilize": req.stabilize,
                "color_correct": req.color_correct,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Enhancement failed: {error}").into());
        }

        let result: serde_json::Value = response.json().await?;

        Ok(VideoEnhanceResponse {
            enhanced_url: result["enhanced_url"].as_str().unwrap_or("").to_string(),
            enhancements_applied: result["enhancements"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        })
    }

    pub async fn detect_beats(
        &self,
        project_id: Uuid,
        audio_track_id: Uuid,
        sensitivity: Option<f32>,
    ) -> Result<BeatSyncResponse, Box<dyn std::error::Error + Send + Sync>> {
        let tracks = self.get_audio_tracks(project_id).await?;
        let track = tracks.iter().find(|t| t.id == audio_track_id).ok_or("Audio track not found")?;

        let botmodels_url = std::env::var("BOTMODELS_URL").unwrap_or_default();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/audio/detect-beats", botmodels_url))
            .json(&serde_json::json!({
                "audio_url": &track.source_url,
                "sensitivity": sensitivity.unwrap_or(0.5),
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Beat detection failed: {error}").into());
        }

        let result: serde_json::Value = response.json().await?;

        let beats: Vec<BeatMarker> = result["beats"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|b| BeatMarker {
                time_ms: b["time_ms"].as_i64().unwrap_or(0),
                strength: b["strength"].as_f64().unwrap_or(1.0) as f32,
                beat_type: b["type"].as_str().unwrap_or("beat").to_string(),
            })
            .collect();

        Ok(BeatSyncResponse {
            beats,
            tempo_bpm: result["tempo_bpm"].as_f64().unwrap_or(120.0) as f32,
        })
    }

    pub async fn generate_waveform(
        &self,
        project_id: Uuid,
        audio_track_id: Uuid,
        samples_per_second: Option<i32>,
    ) -> Result<WaveformResponse, Box<dyn std::error::Error + Send + Sync>> {
        let tracks = self.get_audio_tracks(project_id).await?;
        let track = tracks.iter().find(|t| t.id == audio_track_id).ok_or("Audio track not found")?;

        let botmodels_url = std::env::var("BOTMODELS_URL").unwrap_or_default();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/audio/waveform", botmodels_url))
            .json(&serde_json::json!({
                "audio_url": &track.source_url,
                "samples_per_second": samples_per_second.unwrap_or(10),
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Waveform generation failed: {error}").into());
        }

        let result: serde_json::Value = response.json().await?;

        Ok(WaveformResponse {
            samples: result["samples"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
                .unwrap_or_default(),
            duration_ms: result["duration_ms"].as_i64().unwrap_or(0),
            sample_rate: result["sample_rate"].as_i64().unwrap_or(10) as i32,
        })
    }

    pub async fn process_chat_command(
        &self,
        project_id: Uuid,
        message: &str,
        playhead_ms: Option<i64>,
        selection: Option<serde_json::Value>,
    ) -> Result<ChatEditResponse, Box<dyn std::error::Error + Send + Sync>> {
        let lower_msg = message.to_lowercase();
        let mut commands_executed = Vec::new();

        if lower_msg.contains("add text") || lower_msg.contains("add title") {
            let content = extract_quoted_text(message).unwrap_or_else(|| "Text".to_string());
            let at_ms = playhead_ms.unwrap_or(0);

            self.add_layer(
                project_id,
                AddLayerRequest {
                    name: Some("Text".to_string()),
                    layer_type: "text".to_string(),
                    start_ms: Some(at_ms),
                    end_ms: Some(at_ms + 5000),
                    x: Some(0.5),
                    y: Some(0.5),
                    width: Some(0.8),
                    height: Some(0.2),
                    properties: Some(serde_json::json!({
                        "content": content,
                        "font_family": "Arial",
                        "font_size": 48,
                        "color": "#FFFFFF",
                    })),
                },
            )
            .await?;

            commands_executed.push("Added text layer".to_string());
        }

        if lower_msg.contains("delete") || lower_msg.contains("remove") {
            if let Some(sel) = &selection {
                if let Some(layer_id) = sel.get("layer_id").and_then(|v| v.as_str()) {
                    if let Ok(id) = Uuid::parse_str(layer_id) {
                        self.delete_layer(id).await?;
                        commands_executed.push("Deleted layer".to_string());
                    }
                } else if let Some(clip_id) = sel.get("clip_id").and_then(|v| v.as_str()) {
                    if let Ok(id) = Uuid::parse_str(clip_id) {
                        self.delete_clip(id).await?;
                        commands_executed.push("Deleted clip".to_string());
                    }
                }
            }
        }

        if lower_msg.contains("split") {
            if let Some(sel) = &selection {
                if let Some(clip_id) = sel.get("clip_id").and_then(|v| v.as_str()) {
                    if let Ok(id) = Uuid::parse_str(clip_id) {
                        let at = playhead_ms.unwrap_or(0);
                        self.split_clip(id, at).await?;
                        commands_executed.push("Split clip".to_string());
                    }
                }
            }
        }

        if lower_msg.contains("bigger") || lower_msg.contains("larger") {
            if let Some(sel) = &selection {
                if let Some(layer_id) = sel.get("layer_id").and_then(|v| v.as_str()) {
                    if let Ok(id) = Uuid::parse_str(layer_id) {
                        let layer = video_layers::table.find(id).first::<VideoLayer>(&mut self.get_conn()?)?;
                        self.update_layer(
                            id,
                            UpdateLayerRequest {
                                width: Some(layer.width * 1.2),
                                height: Some(layer.height * 1.2),
                                ..Default::default()
                            },
                        )
                        .await?;
                        commands_executed.push("Made layer bigger".to_string());
                    }
                }
            }
        }

        if lower_msg.contains("smaller") {
            if let Some(sel) = &selection {
                if let Some(layer_id) = sel.get("layer_id").and_then(|v| v.as_str()) {
                    if let Ok(id) = Uuid::parse_str(layer_id) {
                        let layer = video_layers::table.find(id).first::<VideoLayer>(&mut self.get_conn()?)?;
                        self.update_layer(
                            id,
                            UpdateLayerRequest {
                                width: Some(layer.width * 0.8),
                                height: Some(layer.height * 0.8),
                                ..Default::default()
                            },
                        )
                        .await?;
                        commands_executed.push("Made layer smaller".to_string());
                    }
                }
            }
        }

        let response_message = if commands_executed.is_empty() {
            "I couldn't understand that command. Try: add text \"Hello\", delete, split, make it bigger/smaller".to_string()
        } else {
            commands_executed.join(", ")
        };

        let project_detail = self.get_project_detail(project_id).await.ok();

        Ok(ChatEditResponse {
            success: !commands_executed.is_empty(),
            message: response_message,
            commands_executed,
            project: project_detail,
        })
    }
}

fn extract_quoted_text(message: &str) -> Option<String> {
    let chars: Vec<char> = message.chars().collect();
    let mut start = None;
    let mut end = None;

    for (i, c) in chars.iter().enumerate() {
        if *c == '"' || *c == '\'' || *c == '\u{201c}' || *c == '\u{201d}' {
            if start.is_none() {
                start = Some(i + 1);
            } else {
                end = Some(i);
                break;
            }
        }
    }

    match (start, end) {
        (Some(s), Some(e)) if e > s => Some(chars[s..e].iter().collect()),
        _ => None,
    }
}
