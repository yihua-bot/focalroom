-- Focal AI 对话持久化：每项目独立历史，重启保留。
CREATE TABLE IF NOT EXISTS focal_chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id),
    role TEXT NOT NULL CHECK(role IN ('user','assistant')),
    content TEXT NOT NULL,
    suggestion TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_focal_chat_project ON focal_chat_messages(project_id, id);
