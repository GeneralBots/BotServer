//! Single-course page.
use axum::{extract::Path, response::Html};
use uuid::Uuid;

pub async fn handle_learn_course_page(
    Path(course_id): Path<Uuid>,
) -> Html<String> {
    let course_id_safe = course_id;
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Course</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 24px; }}
        .back-link {{ color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }}
        .course-header {{ background: white; border-radius: 12px; padding: 32px; margin-bottom: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
        .course-title {{ font-size: 28px; font-weight: 600; margin-bottom: 16px; }}
        .course-meta {{ display: flex; gap: 24px; color: #666; margin-bottom: 16px; }}
        .course-description {{ line-height: 1.6; color: #444; margin-bottom: 20px; }}
        .progress-section {{ background: #f9f9f9; border-radius: 8px; padding: 16px; }}
        .progress-bar {{ height: 8px; background: #e0e0e0; border-radius: 4px; overflow: hidden; margin-top: 8px; }}
        .progress-fill {{ height: 100%; background: #4caf50; transition: width 0.3s; }}
        .content-grid {{ display: grid; grid-template-columns: 2fr 1fr; gap: 24px; }}
        .lessons-section {{ background: white; border-radius: 12px; padding: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
        .section-title {{ font-size: 18px; font-weight: 600; margin-bottom: 16px; }}
        .lesson-item {{ display: flex; align-items: center; gap: 16px; padding: 16px; border: 1px solid #e0e0e0; border-radius: 8px; margin-bottom: 12px; cursor: pointer; transition: background 0.15s; }}
        .lesson-item:hover {{ background: #f8f9fa; }}
        .lesson-item.completed {{ background: #e8f5e9; border-color: #c8e6c9; }}
        .lesson-item.current {{ background: #e3f2fd; border-color: #90caf9; }}
        .lesson-number {{ width: 32px; height: 32px; border-radius: 50%; background: #e0e0e0; display: flex; align-items: center; justify-content: center; font-weight: 600; font-size: 14px; flex-shrink: 0; }}
        .lesson-item.completed .lesson-number {{ background: #4caf50; color: white; }}
        .lesson-item.current .lesson-number {{ background: #2196f3; color: white; }}
        .lesson-info {{ flex: 1; }}
        .lesson-title {{ font-weight: 500; margin-bottom: 4px; }}
        .lesson-meta {{ font-size: 13px; color: #666; }}
        .sidebar-card {{ background: white; border-radius: 12px; padding: 20px; margin-bottom: 16px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
        .btn {{ padding: 12px 24px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; width: 100%; }}
        .btn-primary {{ background: #0066cc; color: white; }}
        .btn-primary:hover {{ background: #0052a3; }}
        .instructor {{ display: flex; align-items: center; gap: 12px; }}
        .instructor-avatar {{ width: 48px; height: 48px; border-radius: 50%; background: #e0e0e0; }}
        .instructor-name {{ font-weight: 600; }}
        .instructor-title {{ font-size: 13px; color: #666; }}
    </style>
</head>
<body>
    <div class="container">
        <a href="/suite/learn" class="back-link">← Back to Courses</a>
        <div class="course-header">
            <h1 class="course-title" id="courseTitle">Loading...</h1>
            <div class="course-meta">
                <span id="courseLessons">0 lessons</span>
                <span id="courseDuration">0h</span>
                <span id="courseLevel">All levels</span>
                <span id="courseRating">⭐ 0.0</span>
            </div>
            <p class="course-description" id="courseDescription"></p>
            <div class="progress-section">
                <strong>Your Progress: <span id="progressPercent">0%</span></strong>
                <div class="progress-bar"><div class="progress-fill" id="progressFill" style="width: 0%"></div></div>
            </div>
        </div>
        <div class="content-grid">
            <div class="lessons-section">
                <h2 class="section-title">Course Content</h2>
                <div id="lessonsList"></div>
            </div>
            <div class="sidebar">
                <div class="sidebar-card">
                    <button class="btn btn-primary" id="continueBtn" onclick="continueLearning()">Start Learning</button>
                </div>
                <div class="sidebar-card">
                    <h3 class="section-title">Instructor</h3>
                    <div class="instructor">
                        <div class="instructor-avatar"></div>
                        <div>
                            <div class="instructor-name" id="instructorName">Loading...</div>
                            <div class="instructor-title" id="instructorTitle"></div>
                        </div>
                    </div>
                </div>
                <div class="sidebar-card">
                    <h3 class="section-title">What You'll Learn</h3>
                    <ul id="learningObjectives" style="padding-left: 20px; color: #444; line-height: 1.8;"></ul>
                </div>
            </div>
        </div>
    </div>
    <script>
        const courseId = '{course_id_safe}';
        let currentLessonIndex = 0;

        async function loadCourse() {{
            try {{
                const response = await fetch(`/api/learn/courses/${{courseId}}`);
                const course = await response.json();
                if (course) {{
                    document.getElementById('courseTitle').textContent = course.title;
                    document.getElementById('courseDescription').textContent = course.description || '';
                    document.getElementById('courseLessons').textContent = `${{course.lessons_count || 0}} lessons`;
                    document.getElementById('courseDuration').textContent = course.duration || '0h';
                    document.getElementById('courseLevel').textContent = course.level || 'All levels';
                    document.getElementById('courseRating').textContent = `⭐ ${{course.rating || '0.0'}}`;
                    document.getElementById('instructorName').textContent = course.author || 'Unknown';

                    const progress = course.progress || 0;
                    document.getElementById('progressPercent').textContent = progress + '%';
                    document.getElementById('progressFill').style.width = progress + '%';
                    document.getElementById('continueBtn').textContent = progress > 0 ? 'Continue Learning' : 'Start Learning';

                    if (course.lessons && course.lessons.length > 0) {{
                        renderLessons(course.lessons);
                    }}

                    if (course.objectives && course.objectives.length > 0) {{
                        document.getElementById('learningObjectives').innerHTML = course.objectives.map(o => `<li>${{o}}</li>`).join('');
                    }}
                }}
            }} catch (e) {{
                console.error('Failed to load course:', e);
            }}
        }}

        function renderLessons(lessons) {{
            const list = document.getElementById('lessonsList');
            list.innerHTML = lessons.map((l, i) => `
                <div class="lesson-item ${{l.completed ? 'completed' : ''}} ${{l.current ? 'current' : ''}}" onclick="openLesson('${{l.id}}')">
                    <div class="lesson-number">${{l.completed ? '✓' : i + 1}}</div>
                    <div class="lesson-info">
                        <div class="lesson-title">${{l.title}}</div>
                        <div class="lesson-meta">${{l.type || 'Video'}} • ${{l.duration || '5 min'}}</div>
                    </div>
                </div>
            `).join('');
        }}

        function openLesson(lessonId) {{
            window.location = `/suite/learn/${{courseId}}/lesson/${{lessonId}}`;
        }}

        function continueLearning() {{
            window.location = `/suite/learn/${{courseId}}/lesson/next`;
        }}

        loadCourse();
    </script>
</body>
</html>"#);
    Html(html)
}

