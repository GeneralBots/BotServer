use super::*;

pub async fn handle_meet_room_page(
    State(_state): State<Arc<AppState>>,
    Path(room_id): Path<Uuid>,
) -> Html<String> {
    let room_id_safe = room_id;
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Meeting Room</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #1a1a1a; color: white; height: 100vh; overflow: hidden; }}
        .meeting-container {{ display: flex; height: 100vh; }}
        .video-area {{ flex: 1; display: flex; flex-direction: column; }}
        .video-grid {{ flex: 1; display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; padding: 16px; }}
        .video-tile {{ background: #2d2d2d; border-radius: 12px; position: relative; aspect-ratio: 16/9; display: flex; align-items: center; justify-content: center; overflow: hidden; }}
        .video-tile video {{ width: 100%; height: 100%; object-fit: cover; }}
        .video-tile .participant-name {{ position: absolute; bottom: 12px; left: 12px; background: rgba(0,0,0,0.6); padding: 4px 12px; border-radius: 4px; font-size: 13px; }}
        .video-tile .muted-indicator {{ position: absolute; bottom: 12px; right: 12px; background: rgba(0,0,0,0.6); padding: 4px 8px; border-radius: 4px; font-size: 12px; }}
        .video-tile.screen-share {{ grid-column: span 2; grid-row: span 2; }}
        .controls-bar {{ background: #2d2d2d; padding: 16px; display: flex; justify-content: center; gap: 12px; }}
        .control-btn {{ width: 56px; height: 56px; border-radius: 50%; border: none; cursor: pointer; font-size: 20px; display: flex; align-items: center; justify-content: center; transition: background 0.2s; }}
        .control-btn.active {{ background: #3d3d3d; color: white; }}
        .control-btn.inactive {{ background: #c62828; color: white; }}
        .control-btn.end {{ background: #c62828; color: white; width: auto; border-radius: 28px; padding: 0 24px; font-size: 14px; font-weight: 500; }}
        .control-btn:hover {{ opacity: 0.9; }}
        .sidebar {{ width: 320px; background: #2d2d2d; display: none; flex-direction: column; }}
        .sidebar.open {{ display: flex; }}
        .sidebar-header {{ padding: 16px; border-bottom: 1px solid #3d3d3d; display: flex; justify-content: space-between; align-items: center; }}
        .sidebar-header h3 {{ font-size: 16px; }}
        .sidebar-close {{ background: none; border: none; color: white; font-size: 20px; cursor: pointer; }}
        .sidebar-content {{ flex: 1; overflow-y: auto; padding: 16px; }}
        .participant-item {{ display: flex; align-items: center; gap: 12px; padding: 12px 0; border-bottom: 1px solid #3d3d3d; }}
        .participant-avatar {{ width: 40px; height: 40px; border-radius: 50%; background: #4a4a4a; display: flex; align-items: center; justify-content: center; }}
        .participant-info {{ flex: 1; }}
        .participant-name-list {{ font-weight: 500; }}
        .participant-status {{ font-size: 12px; color: #999; }}
        .chat-messages {{ flex: 1; overflow-y: auto; padding: 16px; }}
        .chat-message {{ margin-bottom: 16px; }}
        .chat-sender {{ font-weight: 500; font-size: 13px; margin-bottom: 4px; }}
        .chat-text {{ font-size: 14px; line-height: 1.4; color: #ccc; }}
        .chat-input {{ display: flex; gap: 8px; padding: 16px; border-top: 1px solid #3d3d3d; }}
        .chat-input input {{ flex: 1; padding: 10px 16px; border: 1px solid #3d3d3d; border-radius: 8px; background: #1a1a1a; color: white; }}
        .chat-input button {{ padding: 10px 20px; background: #0066cc; border: none; border-radius: 8px; color: white; cursor: pointer; }}
        .meeting-info {{ position: absolute; top: 16px; left: 16px; background: rgba(0,0,0,0.6); padding: 8px 16px; border-radius: 8px; font-size: 13px; }}
        .meeting-timer {{ font-weight: 600; }}
    </style>
</head>
<body>
    <div class="meeting-container">
        <div class="video-area">
            <div class="meeting-info">
                <span class="meeting-timer" id="meetingTimer">00:00:00</span>
                <span> • Meeting ID: {room_id}</span>
            </div>
            <div class="video-grid" id="videoGrid">
                <div class="video-tile">
                    <video id="localVideo" autoplay muted playsinline></video>
                    <span class="participant-name">You</span>
                </div>
            </div>
            <div class="controls-bar">
                <button class="control-btn active" id="micBtn" onclick="toggleMic()">🎤</button>
                <button class="control-btn active" id="camBtn" onclick="toggleCam()">📹</button>
                <button class="control-btn active" id="screenBtn" onclick="toggleScreen()">🖥️</button>
                <button class="control-btn active" onclick="toggleChat()">💬</button>
                <button class="control-btn active" onclick="toggleParticipants()">👥</button>
                <button class="control-btn active" onclick="toggleWhiteboard()">📝</button>
                <button class="control-btn active" id="recordBtn" data-recording="false" onclick="toggleRecord()">⏺️ Record</button>
                <button class="control-btn end" onclick="leaveMeeting()">Leave Meeting</button>
            </div>
        </div>
        <div class="sidebar" id="participantsSidebar">
            <div class="sidebar-header">
                <h3>Participants (<span id="participantCount">1</span>)</h3>
                <button class="sidebar-close" onclick="toggleParticipants()">×</button>
            </div>
            <div class="sidebar-content" id="participantsList">
                <div class="participant-item">
                    <div class="participant-avatar">Y</div>
                    <div class="participant-info">
                        <div class="participant-name-list">You (Host)</div>
                        <div class="participant-status">Connected</div>
                    </div>
                </div>
            </div>
        </div>
        <div class="sidebar" id="chatSidebar">
            <div class="sidebar-header">
                <h3>Chat</h3>
                <button class="sidebar-close" onclick="toggleChat()">×</button>
            </div>
            <div class="chat-messages" id="chatMessages">
                <div class="chat-message">
                    <div class="chat-sender">System</div>
                    <div class="chat-text">Meeting started. Share the meeting link to invite others.</div>
                </div>
            </div>
            <div class="chat-input">
                <input type="text" id="chatInput" placeholder="Type a message..." onkeypress="if(event.key==='Enter')sendChat()">
                <button onclick="sendChat()">Send</button>
            </div>
        </div>
    </div>
    <script>
        const roomId = '{room_id_safe}';
        let micEnabled = true;
        let camEnabled = true;
        let screenSharing = false;
        let startTime = new Date();

        async function initMedia() {{
            try {{
                const stream = await navigator.mediaDevices.getUserMedia({{ video: true, audio: true }});
                document.getElementById('localVideo').srcObject = stream;
            }} catch (e) {{
                console.error('Failed to get media:', e);
            }}
        }}

        function toggleMic() {{
            micEnabled = !micEnabled;
            const btn = document.getElementById('micBtn');
            btn.className = `control-btn ${{micEnabled ? 'active' : 'inactive'}}`;
            btn.textContent = micEnabled ? '🎤' : '🔇';
        }}

        function toggleCam() {{
            camEnabled = !camEnabled;
            const btn = document.getElementById('camBtn');
            btn.className = `control-btn ${{camEnabled ? 'active' : 'inactive'}}`;
            btn.textContent = camEnabled ? '📹' : '📷';
        }}

        async function toggleScreen() {{
            const btn = document.getElementById('screenBtn');
            if (!screenSharing) {{
                try {{
                    const stream = await navigator.mediaDevices.getDisplayMedia({{ video: true }});
                    screenSharing = true;
                    btn.className = 'control-btn inactive';
                }} catch (e) {{
                    console.error('Screen share failed:', e);
                }}
            }} else {{
                screenSharing = false;
                btn.className = 'control-btn active';
            }}
        }}

        function toggleChat() {{
            const sidebar = document.getElementById('chatSidebar');
            const participantsSidebar = document.getElementById('participantsSidebar');
            participantsSidebar.classList.remove('open');
            sidebar.classList.toggle('open');
        }}

        function toggleParticipants() {{
            const sidebar = document.getElementById('participantsSidebar');
            const chatSidebar = document.getElementById('chatSidebar');
            chatSidebar.classList.remove('open');
            sidebar.classList.toggle('open');
        }}

        function toggleWhiteboard() {{
            window.open(`/suite/meet/room/${{roomId}}/whiteboard`, '_blank', 'width=1200,height=800');
        }}

        function toggleRecord() {{
            const btn = document.getElementById('recordBtn');
            const recording = btn && btn.dataset.recording === 'true';
            const token = localStorage.getItem('gb_access_token') || sessionStorage.getItem('gb_access_token') || '';
            const url = recording
                ? `/api/meet/rooms/${{roomId}}/recording/stop`
                : `/api/meet/rooms/${{roomId}}/recording/start`;
            fetch(url, {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json', ...(token ? {{ 'Authorization': 'Bearer ' + token }} : {{}}) }},
                body: JSON.stringify({{ enable_transcription: true, transcription_language: 'en-US' }})
            }})
                .then(r => r.json())
                .then(() => {{
                    if (btn) {{
                        btn.dataset.recording = recording ? 'false' : 'true';
                        btn.textContent = recording ? 'Record' : 'Stop';
                    }}
                }})
                .catch(err => console.error('Recording toggle failed:', err));
        }}

        function sendChat() {{
            const input = document.getElementById('chatInput');
            const message = input.value.trim();
            if (!message) return;

            const messagesEl = document.getElementById('chatMessages');
            messagesEl.innerHTML += `
                <div class="chat-message">
                    <div class="chat-sender">You</div>
                    <div class="chat-text">${{message}}</div>
                </div>
            `;
            messagesEl.scrollTop = messagesEl.scrollHeight;
            input.value = '';
        }}

        function leaveMeeting() {{
            if (confirm('Are you sure you want to leave the meeting?')) {{
                window.location = '/suite/meet';
            }}
        }}

        function updateTimer() {{
            const now = new Date();
            const diff = now - startTime;
            const hours = Math.floor(diff / 3600000).toString().padStart(2, '0');
            const minutes = Math.floor((diff % 3600000) / 60000).toString().padStart(2, '0');
            const seconds = Math.floor((diff % 60000) / 1000).toString().padStart(2, '0');
            document.getElementById('meetingTimer').textContent = `${{hours}}:${{minutes}}:${{seconds}}`;
        }}

        setInterval(updateTimer, 1000);
        initMedia();
    </script>
</body>
</html>"#);
    Html(html)
}
