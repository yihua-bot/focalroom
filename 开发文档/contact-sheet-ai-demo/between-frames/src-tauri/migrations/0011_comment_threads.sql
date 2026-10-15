
CREATE TABLE comment_threads (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 anchor_json TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'open' CHECK(state IN('open','resolved')),
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE INDEX threads_project ON comment_threads(project_id, state, created_at);
CREATE TABLE comments (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 thread_id TEXT NOT NULL REFERENCES comment_threads(id),
 body TEXT NOT NULL,
 author TEXT NOT NULL,
 created_at TEXT NOT NULL
);
CREATE INDEX comments_thread ON comments(thread_id, id);
