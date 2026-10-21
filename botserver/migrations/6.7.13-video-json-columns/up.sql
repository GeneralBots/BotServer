-- Columns the video crate reads and writes but no migration created.
--
-- `VideoLayer.keyframes_json`, `VideoAudioTrack.waveform_json` /
-- `beat_markers_json` and `VideoExport.gbdrive_path` were declared against
-- these tables, so every insert/update referencing them failed with
-- "column does not exist" on a database built from `botserver/migrations`.
-- The JSON payloads are nullable (generated on demand), so they are appended
-- as nullable columns to keep existing rows valid.

ALTER TABLE video_layers
    ADD COLUMN IF NOT EXISTS keyframes_json JSONB;

ALTER TABLE video_audio_tracks
    ADD COLUMN IF NOT EXISTS waveform_json JSONB,
    ADD COLUMN IF NOT EXISTS beat_markers_json JSONB;

ALTER TABLE video_exports
    ADD COLUMN IF NOT EXISTS gbdrive_path TEXT;
