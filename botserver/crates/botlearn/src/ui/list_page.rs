//! Learning-center list page.
use axum::response::Html;

pub async fn handle_learn_list_page() -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Learning Center</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
        .container { max-width: 1400px; margin: 0 auto; padding: 24px; }
        .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
        .header h1 { font-size: 28px; color: #1a1a1a; }
        .btn { padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
        .btn-primary { background: #0066cc; color: white; }
        .btn-primary:hover { background: #0052a3; }
        .stats-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px; }
        .stat-card { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
        .stat-value { font-size: 28px; font-weight: 600; color: #0066cc; }
        .stat-label { font-size: 13px; color: #666; margin-top: 4px; }
        .tabs { display: flex; gap: 4px; margin-bottom: 24px; border-bottom: 1px solid #e0e0e0; }
        .tab { padding: 12px 24px; background: none; border: none; cursor: pointer; font-size: 14px; color: #666; border-bottom: 2px solid transparent; }
        .tab.active { color: #0066cc; border-bottom-color: #0066cc; }
        .course-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 24px; }
        .course-card { background: white; border-radius: 12px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.08); cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; }
        .course-card:hover { transform: translateY(-2px); box-shadow: 0 4px 16px rgba(0,0,0,0.12); }
        .course-thumbnail { width: 100%; aspect-ratio: 16/9; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); position: relative; }
        .course-thumbnail img { width: 100%; height: 100%; object-fit: cover; }
        .course-badge { position: absolute; top: 12px; left: 12px; padding: 4px 10px; background: rgba(0,0,0,0.7); color: white; border-radius: 4px; font-size: 11px; font-weight: 500; }
        .course-progress-bar { position: absolute; bottom: 0; left: 0; right: 0; height: 4px; background: rgba(255,255,255,0.3); }
        .course-progress-fill { height: 100%; background: #4caf50; }
        .course-info { padding: 16px; }
        .course-title { font-size: 16px; font-weight: 600; color: #1a1a1a; margin-bottom: 8px; }
        .course-meta { font-size: 13px; color: #666; display: flex; gap: 12px; margin-bottom: 12px; }
        .course-description { font-size: 14px; color: #666; line-height: 1.5; }
        .course-footer { display: flex; justify-content: space-between; align-items: center; margin-top: 12px; padding-top: 12px; border-top: 1px solid #f0f0f0; }
        .course-author { font-size: 13px; color: #666; }
        .course-rating { display: flex; align-items: center; gap: 4px; font-size: 13px; }
        .filters { display: flex; gap: 12px; margin-bottom: 24px; }
        .search-box { flex: 1; padding: 10px 16px; border: 1px solid #ddd; border-radius: 8px; }
        .filter-select { padding: 8px 16px; border: 1px solid #ddd; border-radius: 8px; background: white; }
        .empty-state { text-align: center; padding: 80px 24px; color: #666; }
        .empty-state h3 { margin-bottom: 8px; color: #1a1a1a; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Learning Center</h1>
            <button class="btn btn-primary" onclick="createCourse()">Create Course</button>
        </div>
        <div class="stats-row">
            <div class="stat-card">
                <div class="stat-value" id="coursesInProgress">0</div>
                <div class="stat-label">Courses In Progress</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="coursesCompleted">0</div>
                <div class="stat-label">Completed</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="totalHours">0h</div>
                <div class="stat-label">Learning Hours</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="certificates">0</div>
                <div class="stat-label">Certificates Earned</div>
            </div>
        </div>
        <div class="tabs">
            <button class="tab active" data-view="all">All Courses</button>
            <button class="tab" data-view="my-courses">My Courses</button>
            <button class="tab" data-view="in-progress">In Progress</button>
            <button class="tab" data-view="completed">Completed</button>
            <button class="tab" data-view="bookmarked">Bookmarked</button>
        </div>
        <div class="filters">
            <input type="text" class="search-box" placeholder="Search courses..." id="searchInput">
            <select class="filter-select" id="categoryFilter">
                <option value="">All Categories</option>
                <option value="development">Development</option>
                <option value="business">Business</option>
                <option value="design">Design</option>
                <option value="marketing">Marketing</option>
                <option value="compliance">Compliance</option>
            </select>
            <select class="filter-select" id="levelFilter">
                <option value="">All Levels</option>
                <option value="beginner">Beginner</option>
                <option value="intermediate">Intermediate</option>
                <option value="advanced">Advanced</option>
            </select>
            <select class="filter-select" id="sortBy">
                <option value="popular">Most Popular</option>
                <option value="newest">Newest</option>
                <option value="rating">Highest Rated</option>
            </select>
        </div>
        <div class="course-grid" id="courseGrid">
            <div class="empty-state">
                <h3>No courses available</h3>
                <p>Check back later for new learning content</p>
            </div>
        </div>
    </div>
    <script>
        let currentView = 'all';

        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                tab.classList.add('active');
                currentView = tab.dataset.view;
                loadCourses();
            });
        });

        async function loadCourses() {
            try {
                const response = await fetch('/api/learn/courses');
                const courses = await response.json();
                renderCourses(courses);
                updateStats(courses);
            } catch (e) {
                console.error('Failed to load courses:', e);
            }
        }

        function renderCourses(courses) {
            const grid = document.getElementById('courseGrid');
            if (!courses || courses.length === 0) {
                grid.innerHTML = '<div class="empty-state"><h3>No courses available</h3><p>Check back later for new learning content</p></div>';
                return;
            }

            grid.innerHTML = courses.map(c => `
                <div class="course-card" onclick="window.location='/suite/learn/${c.id}'">
                    <div class="course-thumbnail" style="${c.thumbnail_url ? '' : 'background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);'}">
                        ${c.thumbnail_url ? `<img src="${c.thumbnail_url}" alt="${c.title}">` : ''}
                        <span class="course-badge">${c.category || 'General'}</span>
                        ${c.progress !== undefined ? `<div class="course-progress-bar"><div class="course-progress-fill" style="width: ${c.progress}%"></div></div>` : ''}
                    </div>
                    <div class="course-info">
                        <div class="course-title">${c.title}</div>
                        <div class="course-meta">
                            <span>📚 ${c.lessons_count || 0} lessons</span>
                            <span>⏱️ ${c.duration || '0h'}</span>
                            <span>📊 ${c.level || 'All levels'}</span>
                        </div>
                        <div class="course-description">${truncate(c.description || '', 100)}</div>
                        <div class="course-footer">
                            <span class="course-author">by ${c.author || 'Unknown'}</span>
                            <span class="course-rating">⭐ ${c.rating || '0.0'} (${c.reviews_count || 0})</span>
                        </div>
                    </div>
                </div>
            `).join('');
        }

        function truncate(str, len) {
            return str.length > len ? str.substring(0, len) + '...' : str;
        }

        function updateStats(courses) {
            const inProgress = courses.filter(c => c.progress > 0 && c.progress < 100).length;
            const completed = courses.filter(c => c.progress === 100).length;
            document.getElementById('coursesInProgress').textContent = inProgress;
            document.getElementById('coursesCompleted').textContent = completed;
        }

        function createCourse() {
            window.location = '/suite/learn/create';
        }

        loadCourses();
    </script>
</body>
</html>"#;
    Html(html.to_string())
}

