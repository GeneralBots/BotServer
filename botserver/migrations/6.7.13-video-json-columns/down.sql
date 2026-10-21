-- Reverts 6.7.13-video-json-columns.

ALTER TABLE video_exports
    DROP COLUMN IF EXISTS gbdrive_path;

ALTER TABLE video_audio_tracks
    DROP COLUMN IF EXISTS beat_markers_json,
    DROP COLUMN IF EXISTS waveform_json;

ALTER TABLE video_layers
    DROP COLUMN IF EXISTS keyframes_json;
