use super::*;

pub async fn handle_meet_list_page(State(_state): State<Arc<AppState>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Meetings</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
        .container { max-width: 1400px; margin: 0 auto; padding: 24px; }
        .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
        .header h1 { font-size: 28px; color: #1a1a1a; }
        .btn { padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
        .btn-primary { background: #0066cc; color: white; }
        .btn-primary:hover { background: #0052a3; }
        .btn-success { background: #2e7d32; color: white; }
        .btn-success:hover { background: #1b5e20; }
        .stats-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px; }
        .stat-card { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
        .stat-value { font-size: 28px; font-weight: 600; color: #1a1a1a; }
        .stat-label { font-size: 13px; color: #666; margin-top: 4px; }
        .tabs { display: flex; gap: 4px; margin-bottom: 24px; border-bottom: 1px solid #e0e0e0; }
        .tab { padding: 12px 24px; background: none; border: none; cursor: pointer; font-size: 14px; color: #666; border-bottom: 2px solid transparent; }
        .tab.active { color: #0066cc; border-bottom-color: #0066cc; }
        .meeting-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(350px, 1fr)); gap: 24px; }
        .meeting-card { background: white; border-radius: 12px; padding: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; }
        .meeting-card:hover { transform: translateY(-2px); box-shadow: 0 4px 16px rgba(0,0,0,0.12); }
        .meeting-card.live { border-left: 4px solid #2e7d32; }
        .meeting-status { display: inline-block; padding: 4px 12px; border-radius: 20px; font-size: 12px; font-weight: 500; margin-bottom: 12px; }
        .status-live { background: #e8f5e9; color: #2e7d32; }
        .status-scheduled { background: #e3f2fd; color: #1565c0; }
        .status-ended { background: #f5f5f5; color: #666; }
        .meeting-title { font-size: 18px; font-weight: 600; color: #1a1a1a; margin-bottom: 8px; }
        .meeting-time { font-size: 14px; color: #666; margin-bottom: 12px; display: flex; align-items: center; gap: 8px; }
        .meeting-participants { display: flex; align-items: center; gap: 8px; }
        .participant-avatars { display: flex; }
        .participant-avatar { width: 32px; height: 32px; border-radius: 50%; background: #e0e0e0; border: 2px solid white; margin-left: -8px; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 500; color: #666; }
        .participant-avatar:first-child { margin-left: 0; }
        .participant-count { font-size: 13px; color: #666; }
        .meeting-actions { display: flex; gap: 8px; margin-top: 16px; }
        .meeting-actions .btn { padding: 8px 16px; font-size: 13px; }
        .filters { display: flex; gap: 12px; margin-bottom: 24px; }
        .search-box { flex: 1; padding: 10px 16px; border: 1px solid #ddd; border-radius: 8px; }
        .filter-select { padding: 8px 16px; border: 1px solid #ddd; border-radius: 8px; background: white; }
        .empty-state { text-align: center; padding: 80px 24px; color: #666; }
        .empty-state h3 { margin-bottom: 8px; color: #1a1a1a; }
        .quick-join { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); margin-bottom: 24px; display: flex; gap: 12px; align-items: center; }
        .quick-join input { flex: 1; padding: 12px 16px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Meetings</h1>
            <div style="display: flex; gap: 12px;">
                <button class="btn btn-primary" onclick="scheduleMeeting()">📅 Schedule Meeting</button>
                <button class="btn btn-success" onclick="startInstantMeeting()">🎥 Start Instant Meeting</button>
            </div>
        </div>
        <div class="quick-join">
            <span style="font-weight: 500;">Join a meeting:</span>
            <input type="text" id="meetingCode" placeholder="Enter meeting code or link">
            <button class="btn btn-primary" onclick="joinMeeting()">Join</button>
        </div>
        <div class="stats-row">
            <div class="stat-card">
                <div class="stat-value" id="liveMeetings">0</div>
                <div class="stat-label">Live Now</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="todayMeetings">0</div>
                <div class="stat-label">Today's Meetings</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="weekMeetings">0</div>
                <div class="stat-label">This Week</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="totalHours">0h</div>
                <div class="stat-label">Meeting Hours (Month)</div>
            </div>
        </div>
        <div class="tabs">
            <button class="tab active" data-view="upcoming">Upcoming</button>
            <button class="tab" data-view="live">Live Now</button>
            <button class="tab" data-view="past">Past Meetings</button>
            <button class="tab" data-view="recordings">Recordings</button>
        </div>
        <div class="filters">
            <input type="text" class="search-box" placeholder="Search meetings..." id="searchInput" oninput="filterMeetings()">
            <select class="filter-select" id="typeFilter" onchange="filterMeetings()">
                <option value="">All Types</option>
                <option value="instant">Instant</option>
                <option value="scheduled">Scheduled</option>
                <option value="recurring">Recurring</option>
            </select>
            <select class="filter-select" id="sortBy" onchange="filterMeetings()">
                <option value="date">Sort by Date</option>
                <option value="name">Sort by Name</option>
                <option value="participants">Sort by Participants</option>
            </select>
        </div>
        <div class="meeting-grid" id="meetingGrid">
            <div class="empty-state">
                <h3>No meetings scheduled</h3>
                <p>Schedule a meeting or start an instant meeting to get started</p>
            </div>
        </div>
    </div>
    <script>
        let meetings = [];
        let currentView = 'upcoming';

        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                tab.classList.add('active');
                currentView = tab.dataset.view;
                filterMeetings();
            });
        });

        async function loadMeetings() {
            try {
                const response = await fetch('/api/meet/rooms');
                meetings = await response.json();
                filterMeetings();
                updateStats();
            } catch (e) {
                console.error('Failed to load meetings:', e);
            }
        }

        function filterMeetings() {
            let filtered = meetings;
            const query = document.getElementById('searchInput').value.toLowerCase();
            const type = document.getElementById('typeFilter').value;

            if (currentView === 'live') {
                filtered = filtered.filter(m => m.status === 'live' || m.is_active);
            } else if (currentView === 'upcoming') {
                filtered = filtered.filter(m => m.status === 'scheduled' || (!m.ended_at && !m.is_active));
            } else if (currentView === 'past') {
                filtered = filtered.filter(m => m.status === 'ended' || m.ended_at);
            }

            if (query) {
                filtered = filtered.filter(m =>
                    (m.name && m.name.toLowerCase().includes(query)) ||
                    (m.topic && m.topic.toLowerCase().includes(query))
                );
            }

            if (type) {
                filtered = filtered.filter(m => m.meeting_type === type);
            }

            renderMeetings(filtered);
        }

        function renderMeetings(meetings) {
            const grid = document.getElementById('meetingGrid');
            if (!meetings || meetings.length === 0) {
                grid.innerHTML = '<div class="empty-state"><h3>No meetings found</h3><p>Try a different filter or create a new meeting</p></div>';
                return;
            }

            grid.innerHTML = meetings.map(m => {
                const isLive = m.status === 'live' || m.is_active;
                return `
                    <div class="meeting-card ${isLive ? 'live' : ''}" onclick="openMeeting('${m.id}')">
                        <span class="meeting-status status-${isLive ? 'live' : (m.ended_at ? 'ended' : 'scheduled')}">
                            ${isLive ? '🔴 Live' : (m.ended_at ? 'Ended' : 'Scheduled')}
                        </span>
                        <div class="meeting-title">${m.name || m.topic || 'Untitled Meeting'}</div>
                        <div class="meeting-time">
                            📅 ${formatDateTime(m.scheduled_at || m.created_at)}
                            ${m.duration ? ` • ${m.duration} min` : ''}
                        </div>
                        <div class="meeting-participants">
                            <div class="participant-avatars">
                                ${(m.participants || []).slice(0, 3).map((p, i) => `
                                    <div class="participant-avatar" style="background: ${getAvatarColor(i)}">${(p.name || 'U')[0]}</div>
                                `).join('')}
                            </div>
                            <span class="participant-count">${m.participant_count || (m.participants || []).length} participants</span>
                        </div>
                        <div class="meeting-actions">
                            ${isLive ?
                                `<button class="btn btn-success" onclick="event.stopPropagation(); joinRoom('${m.id}')">Join Now</button>` :
                                `<button class="btn btn-primary" onclick="event.stopPropagation(); startMeeting('${m.id}')">Start</button>`
                            }
                            <button class="btn" style="background: #f5f5f5;" onclick="event.stopPropagation(); copyLink('${m.id}')">Copy Link</button>
                        </div>
                    </div>
                `;
            }).join('');
        }

        function formatDateTime(dateStr) {
            if (!dateStr) return 'Not scheduled';
            const date = new Date(dateStr);
            const now = new Date();
            const tomorrow = new Date(now);
            tomorrow.setDate(tomorrow.getDate() + 1);

            if (date.toDateString() === now.toDateString()) {
                return `Today at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
            }
            if (date.toDateString() === tomorrow.toDateString()) {
                return `Tomorrow at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
            }
            return date.toLocaleDateString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
        }

        function getAvatarColor(index) {
            const colors = ['#e3f2fd', '#f3e5f5', '#e8f5e9', '#fff3e0', '#fce4ec'];
            return colors[index % colors.length];
        }

        function updateStats() {
            const live = meetings.filter(m => m.status === 'live' || m.is_active).length;
            document.getElementById('liveMeetings').textContent = live;
            document.getElementById('todayMeetings').textContent = meetings.length;
        }

        function scheduleMeeting() {
            window.location = '/suite/meet/schedule';
        }

        async function startInstantMeeting() {
            try {
                const response = await fetch('/api/meet/create', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ name: 'Instant Meeting', created_by: 'user' })
                });
                const meeting = await response.json();
                window.location = `/suite/meet/room/${meeting.id}`;
            } catch (e) {
                alert('Failed to create meeting');
            }
        }

        function joinMeeting() {
            const code = document.getElementById('meetingCode').value.trim();
            if (!code) {
                alert('Please enter a meeting code');
                return;
            }
            window.location = `/suite/meet/join/${code}`;
        }

        function openMeeting(id) {
            window.location = `/suite/meet/${id}`;
        }

        function joinRoom(id) {
            window.location = `/suite/meet/room/${id}`;
        }

        function startMeeting(id) {
            window.location = `/suite/meet/room/${id}`;
        }

        function copyLink(id) {
            const link = `${window.location.origin}/suite/meet/join/${id}`;
            navigator.clipboard.writeText(link);
            alert('Meeting link copied to clipboard!');
        }

        loadMeetings();
    </script>
</body>
</html>"#;
    Html(html.to_string())
}
