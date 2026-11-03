-- 同一项目里，相同画面内容可以保存为两条独立照片记录：各自有自己的保留/淘汰状态与序列位置。
-- photos 上的 UNIQUE(project_id, content_sha256) 会阻断这件事，只能重建表去掉它。
-- source_assets 仍保持 (project_id, content_sha256) 唯一：相同内容只有一个母版资产，多份照片共用它。
-- SQLite 不允许在事务内关闭外键，而迁移执行器把每个文件包进事务，
-- 所以建库路径先在唯一这条连接上开一个关闭外键的窗口（见 db.rs 的迁移前置检查），本文件只负责重建。
-- 引用 photos 的跨表触发器先摘掉：DROP TABLE 时它们会因表暂时不存在而无法编译，重建后原样装回（同 0034）。
DROP TRIGGER IF EXISTS decision_owner;
DROP TRIGGER IF EXISTS decision_owner_update;
DROP TRIGGER IF EXISTS comparison_owner;
DROP TRIGGER IF EXISTS draft_item_owner;
CREATE TABLE photos_v37 (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 import_index INTEGER NOT NULL, original_path TEXT NOT NULL, filename TEXT NOT NULL,
 file_size INTEGER NOT NULL, content_sha256 TEXT NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL,
 orientation INTEGER NOT NULL DEFAULT 1, thumbnail_path TEXT NOT NULL, preview_path TEXT NOT NULL,
 created_at TEXT NOT NULL, UNIQUE(project_id, import_index)
);
INSERT INTO photos_v37(rowid,id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,orientation,thumbnail_path,preview_path,created_at)
 SELECT rowid,id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,orientation,thumbnail_path,preview_path,created_at
 FROM photos ORDER BY rowid;
DROP TABLE photos;
ALTER TABLE photos_v37 RENAME TO photos;
CREATE INDEX idx_photos_project_hash ON photos(project_id, content_sha256);
CREATE TRIGGER photo_owner_immutable BEFORE UPDATE OF project_id ON photos WHEN OLD.project_id<>NEW.project_id BEGIN SELECT RAISE(ABORT,'photo project is immutable'); END;
CREATE TRIGGER decision_owner BEFORE INSERT ON photo_decisions WHEN NOT EXISTS(SELECT 1 FROM photos WHERE id=NEW.photo_id AND project_id=NEW.project_id) BEGIN SELECT RAISE(ABORT,'photo project mismatch'); END;
CREATE TRIGGER decision_owner_update BEFORE UPDATE OF project_id,photo_id ON photo_decisions WHEN NOT EXISTS(SELECT 1 FROM photos WHERE id=NEW.photo_id AND project_id=NEW.project_id) BEGIN SELECT RAISE(ABORT,'photo project mismatch'); END;
CREATE TRIGGER comparison_owner BEFORE INSERT ON comparison_events WHEN (SELECT COUNT(*) FROM photos WHERE project_id=NEW.project_id AND id IN(NEW.left_photo_id,NEW.right_photo_id))<>2 BEGIN SELECT RAISE(ABORT,'comparison project mismatch'); END;
CREATE TRIGGER draft_item_owner BEFORE INSERT ON draft_items WHEN NOT EXISTS(SELECT 1 FROM photos p JOIN drafts d ON d.project_id=p.project_id WHERE p.id=NEW.photo_id AND d.id=NEW.draft_id) BEGIN SELECT RAISE(ABORT,'draft photo project mismatch'); END;
