//! Create-course page.
use axum::response::Html;

pub async fn handle_learn_create_page() -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Create Course</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; padding: 24px; }
        .back-link { color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }
        .form-card { background: white; border-radius: 12px; padding: 32px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
        h1 { font-size: 24px; margin-bottom: 24px; }
        .form-group { margin-bottom: 20px; }
        .form-group label { display: block; font-weight: 500; margin-bottom: 8px; }
        .form-group input, .form-group textarea, .form-group select { width: 100%; padding: 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
        .form-group textarea { min-height: 120px; resize: vertical; }
        .form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
        .btn { padding: 12px 24px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
        .btn-primary { background: #0066cc; color: white; }
        .btn-primary:hover { background: #0052a3; }
        .btn-secondary { background: #f5f5f5; color: #333; }
        .form-actions { display: flex; gap: 12px; justify-content: flex-end; }
    </style>
</head>
<body>
    <div class="container">
        <a href="/suite/learn" class="back-link">← Back to Courses</a>
        <div class="form-card">
            <h1>Create New Course</h1>
            <form id="courseForm">
                <div class="form-group">
                    <label>Course Title</label>
                    <input type="text" id="title" required placeholder="Enter course title">
                </div>
                <div class="form-group">
                    <label>Description</label>
                    <textarea id="description" placeholder="Describe what students will learn"></textarea>
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label>Category</label>
                        <select id="category">
                            <option value="development">Development</option>
                            <option value="business">Business</option>
                            <option value="design">Design</option>
                            <option value="marketing">Marketing</option>
                            <option value="compliance">Compliance</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>Level</label>
                        <select id="level">
                            <option value="beginner">Beginner</option>
                            <option value="intermediate">Intermediate</option>
                            <option value="advanced">Advanced</option>
                        </select>
                    </div>
                </div>
                <div class="form-group">
                    <label>Learning Objectives (one per line)</label>
                    <textarea id="objectives" placeholder="What will students learn?"></textarea>
                </div>
                <div class="form-actions">
                    <button type="button" class="btn btn-secondary" onclick="window.location='/suite/learn'">Cancel</button>
                    <button type="submit" class="btn btn-primary">Create Course</button>
                </div>
            </form>
        </div>
    </div>
    <script>
        document.getElementById('courseForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const data = {
                title: document.getElementById('title').value,
                description: document.getElementById('description').value,
                category: document.getElementById('category').value,
                level: document.getElementById('level').value,
                objectives: document.getElementById('objectives').value.split('\n').filter(o => o.trim())
            };

            try {
                const response = await fetch('/api/learn/courses', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });

                if (response.ok) {
                    const course = await response.json();
                    window.location = `/suite/learn/${course.id}`;
                } else {
                    alert('Failed to create course');
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

