use super::*;

pub async fn handle_meet_schedule_page(State(_state): State<Arc<AppState>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Schedule Meeting</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
        .container { max-width: 700px; margin: 0 auto; padding: 24px; }
        .back-link { color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }
        .form-card { background: white; border-radius: 12px; padding: 32px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
        h1 { font-size: 24px; margin-bottom: 24px; }
        .form-group { margin-bottom: 20px; }
        .form-group label { display: block; font-weight: 500; margin-bottom: 8px; }
        .form-group input, .form-group textarea, .form-group select { width: 100%; padding: 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
        .form-group textarea { min-height: 100px; resize: vertical; }
        .form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
        .checkbox-group { display: flex; align-items: center; gap: 8px; }
        .checkbox-group input { width: auto; }
        .btn { padding: 12px 24px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
        .btn-primary { background: #0066cc; color: white; }
        .btn-primary:hover { background: #0052a3; }
        .btn-secondary { background: #f5f5f5; color: #333; }
        .form-actions { display: flex; gap: 12px; justify-content: flex-end; }
        .settings-section { background: #f9f9f9; border-radius: 8px; padding: 20px; margin-bottom: 20px; }
        .settings-title { font-weight: 600; margin-bottom: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <a href="/suite/meet" class="back-link">← Back to Meetings</a>
        <div class="form-card">
            <h1>Schedule Meeting</h1>
            <form id="meetingForm">
                <div class="form-group">
                    <label>Meeting Title</label>
                    <input type="text" id="name" required placeholder="Enter meeting title">
                </div>
                <div class="form-group">
                    <label>Description (optional)</label>
                    <textarea id="description" placeholder="Add meeting description or agenda"></textarea>
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label>Date</label>
                        <input type="date" id="date" required>
                    </div>
                    <div class="form-group">
                        <label>Time</label>
                        <input type="time" id="time" required>
                    </div>
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label>Duration</label>
                        <select id="duration">
                            <option value="15">15 minutes</option>
                            <option value="30">30 minutes</option>
                            <option value="45">45 minutes</option>
                            <option value="60" selected>1 hour</option>
                            <option value="90">1.5 hours</option>
                            <option value="120">2 hours</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Meeting Type</label>
                        <select id="meetingType">
                            <option value="scheduled">One-time Meeting</option>
                            <option value="recurring">Recurring Meeting</option>
                        </select>
                    </div>
                </div>
                <div class="settings-section">
                    <div class="settings-title">Meeting Settings</div>
                    <div class="form-group">
                        <label class="checkbox-group">
                            <input type="checkbox" id="waitingRoom" checked>
                            <span>Enable waiting room</span>
                        </label>
                    </div>
                    <div class="form-group">
                        <label class="checkbox-group">
                            <input type="checkbox" id="muteOnEntry" checked>
                            <span>Mute participants on entry</span>
                        </label>
                    </div>
                    <div class="form-group">
                        <label class="checkbox-group">
                            <input type="checkbox" id="allowRecording">
                            <span>Allow recording</span>
                        </label>
                    </div>
                </div>
                <div class="form-actions">
                    <button type="button" class="btn btn-secondary" onclick="window.location='/suite/meet'">Cancel</button>
                    <button type="submit" class="btn btn-primary">Schedule Meeting</button>
                </div>
            </form>
        </div>
    </div>
    <script>
        const today = new Date();
        document.getElementById('date').valueAsDate = today;
        document.getElementById('time').value = '09:00';

        document.getElementById('meetingForm').addEventListener('submit', async (e) => {
            e.preventDefault();

            const date = document.getElementById('date').value;
            const time = document.getElementById('time').value;
            const scheduledAt = new Date(`${date}T${time}`).toISOString();

            const data = {
                name: document.getElementById('name').value,
                description: document.getElementById('description').value || null,
                scheduled_at: scheduledAt,
                duration: parseInt(document.getElementById('duration').value),
                meeting_type: document.getElementById('meetingType').value,
                settings: {
                    waiting_room: document.getElementById('waitingRoom').checked,
                    mute_on_entry: document.getElementById('muteOnEntry').checked,
                    allow_recording: document.getElementById('allowRecording').checked
                }
            };

            try {
                const response = await fetch('/api/meet/create', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ name: data.title, created_by: 'user', settings: data.settings })
                });

                if (response.ok) {
                    const meeting = await response.json();
                    window.location = `/suite/meet/${meeting.id}`;
                } else {
                    alert('Failed to schedule meeting');
                }
            } catch (e) {
                alert('Error: ' + e.message);
            }
        });
    </script>
</body>
</html>"#;
    Html(html.to_string())
}
pub fn configure_meet_ui_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/suite/meet", get(handle_meet_list_page))
        .route("/suite/meet/schedule", get(handle_meet_schedule_page))
        .route("/suite/meet/room/:id", get(handle_meet_room_page))
}
