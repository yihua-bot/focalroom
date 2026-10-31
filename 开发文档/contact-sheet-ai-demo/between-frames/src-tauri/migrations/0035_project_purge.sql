-- 桌面项目深度删除：purged_at 标记已清理的归档项目。
-- 删除语义（与云端 0035 一致）：清空可删除的项目数据（照片记录/资产/草稿/看板/提案等），
-- append-only 审计表（command_events/decision_events/comparison_events/sequence_versions 等）
-- 按既有触发器保护保留；projects 行保留用于审计；list 归档视图排除 purged。
ALTER TABLE projects ADD COLUMN purged_at TEXT;
