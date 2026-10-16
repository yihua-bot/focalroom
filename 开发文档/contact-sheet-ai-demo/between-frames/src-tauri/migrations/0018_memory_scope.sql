
-- 作者级跨项目记忆授权（M3）：用户显式开启后，同一本地作者的项目间共享偏好记忆检索。
-- 默认关闭——记忆隔离是隐私保守默认（M02）。
CREATE TABLE IF NOT EXISTS memory_scope_settings (
    user_id TEXT PRIMARY KEY DEFAULT('local'),
    cross_project_enabled INTEGER NOT NULL DEFAULT(0) CHECK(cross_project_enabled IN(0,1)),
    updated_at TEXT NOT NULL
);
