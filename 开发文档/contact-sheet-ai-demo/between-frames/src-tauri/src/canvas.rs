//! G3 Narrative Canvas：Board 领域命令——创建/查询/节点操作。
//! 正式画布变更走 CommandHandler 事务；Agent 只能写 draft 状态的 Board。

use crate::{
    command_handler::CommandHandler,
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasNode {
    pub id: String,
    pub board_id: String,
    pub node_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub z_order: i64,
    pub asset_id: Option<String>,
    pub rendition_ref: Option<String>,
    pub content_json: Option<String>,
    pub style_json: Option<String>,
    pub children_json: Option<String>,
    pub locked: bool,
    pub visible: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardCommand {
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacePhotoNodeCommand {
    pub board_id: String,
    pub asset_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveNodesCommand {
    pub board_id: String,
    pub node_ids: Vec<String>,
    pub dx: f64,
    pub dy: f64,
}

impl CommandHandler<'_> {
    /// 创建画布（正式命令，走事务+审计）。
    pub async fn create_board(&self, project: &str, title: &str) -> Result<Board> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 200 {
            return Err(AppError::new("VALIDATION_ERROR", "画布标题须为 1–200 字符"));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO boards(id,project_id,title,state,created_at,updated_at) VALUES(?,?,?,'draft',?,?)")
            .bind(&id).bind(project).bind(title).bind(&now).bind(&now)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Board {
            id,
            project_id: project.to_owned(),
            title: title.to_owned(),
            state: "draft".to_owned(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// 在画布上放置照片节点（引用资产，不复制像素）。
    pub async fn place_photo_node(
        &self,
        project: &str,
        cmd: &PlacePhotoNodeCommand,
    ) -> Result<CanvasNode> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        // 验证画布属于此项目
        let board: Option<String> =
            sqlx::query_scalar("SELECT id FROM boards WHERE id=? AND project_id=?")
                .bind(&cmd.board_id)
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?;
        if board.is_none() {
            return Err(AppError::new("NOT_FOUND", "画布不存在或不属于此项目"));
        }
        // 验证资产存在（通过 alias 或直接 asset_id）
        let asset: Option<String> = sqlx::query_scalar(
            "SELECT id FROM source_assets WHERE id=? OR id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?)",
        )
        .bind(&cmd.asset_id).bind(&cmd.asset_id)
        .fetch_optional(&mut *tx)
        .await?;
        let asset_id = asset.ok_or_else(|| AppError::new("NOT_FOUND", "资产不存在"))?;

        let id = domain::id();
        let now = domain::timestamp();
        let z: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(z_order),0)+1 FROM canvas_nodes WHERE board_id=?",
        )
        .bind(&cmd.board_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,created_at,updated_at) VALUES(?,?, 'photo',?,?,?,?,0,?,?,?,?)",
        )
        .bind(&id).bind(&cmd.board_id)
        .bind(cmd.x).bind(cmd.y).bind(cmd.width).bind(cmd.height)
        .bind(z).bind(&asset_id).bind(&now).bind(&now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(CanvasNode {
            id,
            board_id: cmd.board_id.clone(),
            node_type: "photo".to_owned(),
            x: cmd.x,
            y: cmd.y,
            width: cmd.width,
            height: cmd.height,
            rotation: 0.0,
            z_order: z,
            asset_id: Some(asset_id),
            rendition_ref: None,
            content_json: None,
            style_json: None,
            children_json: None,
            locked: false,
            visible: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// 移动节点（原子操作，支持多选）。
    pub async fn move_nodes(&self, project: &str, cmd: &MoveNodesCommand) -> Result<()> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let now = domain::timestamp();
        for node_id in &cmd.node_ids {
            let result = sqlx::query(
                "UPDATE canvas_nodes SET x=x+?, y=y+?, updated_at=? WHERE id=? AND board_id=? AND locked=0 AND EXISTS(SELECT 1 FROM boards WHERE id=? AND project_id=?)",
            )
            .bind(cmd.dx).bind(cmd.dy).bind(&now)
            .bind(node_id).bind(&cmd.board_id).bind(&cmd.board_id).bind(project)
            .execute(&mut *tx)
            .await?;
            if result.rows_affected() == 0 {
                return Err(AppError::new(
                    "NOT_FOUND",
                    format!("节点 {node_id} 不存在、已锁定或不属于此项目"),
                ));
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// 查询项目的画布列表。
    pub async fn list_boards(&self, project: &str) -> Result<Vec<Board>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, project_id, title, state, created_at, updated_at FROM boards WHERE project_id=? ORDER BY created_at",
        )
        .bind(project)
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, project_id, title, state, created_at, updated_at)| Board {
                    id,
                    project_id,
                    title,
                    state,
                    created_at,
                    updated_at,
                },
            )
            .collect())
    }

    /// 查询画布的全部节点（按 z_order 排序）。
    pub async fn get_board_nodes(&self, board_id: &str) -> Result<Vec<CanvasNode>> {
        let rows = sqlx::query(
            "SELECT id, board_id, node_type, x, y, width, height, rotation, z_order, asset_id, rendition_ref, content_json, style_json, children_json, locked, visible, created_at, updated_at FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order",
        )
        .bind(board_id)
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| CanvasNode {
                id: row.get("id"),
                board_id: row.get("board_id"),
                node_type: row.get("node_type"),
                x: row.get("x"),
                y: row.get("y"),
                width: row.get("width"),
                height: row.get("height"),
                rotation: row.get("rotation"),
                z_order: row.get("z_order"),
                asset_id: row.get("asset_id"),
                rendition_ref: row.get("rendition_ref"),
                content_json: row.get("content_json"),
                style_json: row.get("style_json"),
                children_json: row.get("children_json"),
                locked: row.get::<i64, _>("locked") != 0,
                visible: row.get::<i64, _>("visible") != 0,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }
}

impl Store {
    pub async fn create_board(&self, project: &str, title: &str) -> Result<Board> {
        CommandHandler::new(self).create_board(project, title).await
    }
    pub async fn place_photo_node(
        &self,
        project: &str,
        cmd: &PlacePhotoNodeCommand,
    ) -> Result<CanvasNode> {
        CommandHandler::new(self)
            .place_photo_node(project, cmd)
            .await
    }
    pub async fn move_nodes(&self, project: &str, cmd: &MoveNodesCommand) -> Result<()> {
        CommandHandler::new(self).move_nodes(project, cmd).await
    }
    pub async fn list_boards(&self, project: &str) -> Result<Vec<Board>> {
        CommandHandler::new(self).list_boards(project).await
    }
    pub async fn get_board_nodes(&self, board_id: &str) -> Result<Vec<CanvasNode>> {
        CommandHandler::new(self).get_board_nodes(board_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn board_create_place_move_and_query() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(temp.path().join("canvas-test.sqlite").as_path())
            .await
            .unwrap();
        let project = db.create("canvas-test", "board test").await.unwrap();

        // 创建画布
        let board = db.create_board(&project.id, "探索 A").await.unwrap();
        assert_eq!(board.state, "draft");
        assert_eq!(board.title, "探索 A");

        // 放置照片节点（需要先导入一张照片）
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,orientation,thumbnail_path,preview_path,created_at) VALUES(?,?,?,?,?,?,?,?,?,1,?,?,?)")
            .bind(&id).bind(&project.id).bind(1).bind("/tmp/c.jpg").bind("c.jpg").bind(100)
            .bind(format!("sha-{}", &id)).bind(10).bind(10)
            .bind("/tmp/t.jpg").bind("/tmp/p.jpg").bind(&now)
            .execute(&db.pool).await.unwrap();
        // 手动触发 backfill（Store::open 时 photos 为空，backfill 跳过）
        // 直接插入 source_assets + alias
        sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES(?,?,?, '[]',10,10,10,'jpeg',?)")
            .bind(&id).bind(&project.id).bind(format!("sha-{}", &id)).bind(&now)
            .execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO asset_aliases(photo_id,asset_id) VALUES(?,?)")
            .bind(&id)
            .bind(&id)
            .execute(&db.pool)
            .await
            .unwrap();

        let node = db
            .place_photo_node(
                &project.id,
                &PlacePhotoNodeCommand {
                    board_id: board.id.clone(),
                    asset_id: id.clone(),
                    x: 100.0,
                    y: 200.0,
                    width: 300.0,
                    height: 200.0,
                },
            )
            .await
            .unwrap();
        assert_eq!(node.node_type, "photo");
        assert_eq!(node.x, 100.0);

        // 移动节点
        db.move_nodes(
            &project.id,
            &MoveNodesCommand {
                board_id: board.id.clone(),
                node_ids: vec![node.id.clone()],
                dx: 50.0,
                dy: -30.0,
            },
        )
        .await
        .unwrap();

        // 查询验证
        let nodes = db.get_board_nodes(&board.id).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].x, 150.0);
        assert_eq!(nodes[0].y, 170.0);

        // 用 photo_id 作为 asset_id 引用也能工作（alias 解析）
        let node2 = db
            .place_photo_node(
                &project.id,
                &PlacePhotoNodeCommand {
                    board_id: board.id.clone(),
                    asset_id: id.clone(), // 同一个 photo_id
                    x: 500.0,
                    y: 100.0,
                    width: 200.0,
                    height: 150.0,
                },
            )
            .await
            .unwrap();
        assert!(node2.asset_id.is_some());

        let boards = db.list_boards(&project.id).await.unwrap();
        assert_eq!(boards.len(), 1);
    }

    #[tokio::test]
    async fn locked_node_cannot_move() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(temp.path().join("lock-test.sqlite").as_path())
            .await
            .unwrap();
        let project = db.create("lock-test", "lock").await.unwrap();
        let board = db.create_board(&project.id, "locked").await.unwrap();
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES(?,?,?, '[]',10,10,10,'jpeg',?)")
            .bind(&id).bind(&project.id).bind("sha-lock").bind(&now)
            .execute(&db.pool).await.unwrap();
        let node = db
            .place_photo_node(
                &project.id,
                &PlacePhotoNodeCommand {
                    board_id: board.id.clone(),
                    asset_id: id.clone(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 100.0,
                },
            )
            .await
            .unwrap();
        // 锁定
        sqlx::query("UPDATE canvas_nodes SET locked=1 WHERE id=?")
            .bind(&node.id)
            .execute(&db.pool)
            .await
            .unwrap();
        // 移动应失败
        let result = db
            .move_nodes(
                &project.id,
                &MoveNodesCommand {
                    board_id: board.id.clone(),
                    node_ids: vec![node.id.clone()],
                    dx: 10.0,
                    dy: 10.0,
                },
            )
            .await;
        assert!(result.is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTextNodeCommand {
    pub board_id: String,
    pub node_type: String, // "text" | "sticky" | "shape"
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub content: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNodeCommand {
    pub board_id: String,
    pub node_id: String,
}

impl CommandHandler<'_> {
    /// 添加文本/便签/形状节点。
    pub async fn add_text_node(
        &self,
        project: &str,
        cmd: &AddTextNodeCommand,
    ) -> Result<CanvasNode> {
        if !matches!(cmd.node_type.as_str(), "text" | "sticky" | "shape") {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "节点类型须为 text/sticky/shape",
            ));
        }
        if cmd.content.len() > 5000 {
            return Err(AppError::new("VALIDATION_ERROR", "内容不超过 5000 字符"));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let board: Option<String> =
            sqlx::query_scalar("SELECT id FROM boards WHERE id=? AND project_id=?")
                .bind(&cmd.board_id)
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?;
        if board.is_none() {
            return Err(AppError::new("NOT_FOUND", "画布不存在"));
        }
        let id = domain::id();
        let now = domain::timestamp();
        let z: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(z_order),0)+1 FROM canvas_nodes WHERE board_id=?",
        )
        .bind(&cmd.board_id)
        .fetch_one(&mut *tx)
        .await?;
        let content_json = serde_json::to_string(&serde_json::json!({"text": cmd.content}))
            .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        let style_json = cmd.color.as_ref().map(|c| format!("{{\"color\":\"{c}\"}}"));
        sqlx::query(
            "INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,content_json,style_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,0,?,?,?,?)",
        )
        .bind(&id).bind(&cmd.board_id).bind(&cmd.node_type)
        .bind(cmd.x).bind(cmd.y).bind(cmd.width).bind(cmd.height)
        .bind(z).bind(&content_json).bind(&style_json)
        .bind(&now).bind(&now)
        .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(CanvasNode {
            id,
            board_id: cmd.board_id.clone(),
            node_type: cmd.node_type.clone(),
            x: cmd.x,
            y: cmd.y,
            width: cmd.width,
            height: cmd.height,
            rotation: 0.0,
            z_order: z,
            asset_id: None,
            rendition_ref: None,
            content_json: Some(content_json),
            style_json,
            children_json: None,
            locked: false,
            visible: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// 删除节点（软删——visible=0，保留历史关联）。
    pub async fn delete_node(&self, project: &str, cmd: &DeleteNodeCommand) -> Result<()> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let result = sqlx::query(
            "UPDATE canvas_nodes SET visible=0, updated_at=? WHERE id=? AND board_id=? AND EXISTS(SELECT 1 FROM boards WHERE id=? AND project_id=?)",
        )
        .bind(domain::timestamp())
        .bind(&cmd.node_id).bind(&cmd.board_id).bind(&cmd.board_id).bind(project)
        .execute(&mut *tx).await?;
        if result.rows_affected() == 0 {
            return Err(AppError::new("NOT_FOUND", "节点不存在"));
        }
        tx.commit().await?;
        Ok(())
    }
}

impl Store {
    pub async fn add_text_node(
        &self,
        project: &str,
        cmd: &AddTextNodeCommand,
    ) -> Result<CanvasNode> {
        CommandHandler::new(self).add_text_node(project, cmd).await
    }
    pub async fn delete_node(&self, project: &str, cmd: &DeleteNodeCommand) -> Result<()> {
        CommandHandler::new(self).delete_node(project, cmd).await
    }
}

// ── Board Annotations（画布评论）──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardAnnotation {
    pub id: String,
    pub board_id: String,
    pub anchor_type: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub node_id: Option<String>,
    pub nx: Option<f64>,
    pub ny: Option<f64>,
    pub author: String,
    pub body: String,
    pub resolved: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAnnotationCommand {
    pub board_id: String,
    pub x: f64,
    pub y: f64,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveAnnotationCommand {
    pub annotation_id: String,
    pub resolved: bool,
}

impl CommandHandler<'_> {
    /// 添加画布评论（锚定 board 坐标）。
    pub async fn add_annotation(
        &self,
        project: &str,
        cmd: &AddAnnotationCommand,
    ) -> Result<BoardAnnotation> {
        let body = cmd.body.trim();
        if body.is_empty() || body.chars().count() > 2000 {
            return Err(AppError::new("VALIDATION_ERROR", "评论须为 1–2000 字符"));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        let board: Option<String> =
            sqlx::query_scalar("SELECT id FROM boards WHERE id=? AND project_id=?")
                .bind(&cmd.board_id)
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?;
        if board.is_none() {
            return Err(AppError::new("NOT_FOUND", "画布不存在"));
        }
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query(
            "INSERT INTO board_annotations(id,board_id,anchor_type,x,y,author,body,resolved,created_at) VALUES(?,?,'board',?,?,?,'',?,0,?)",
        )
        .bind(&id).bind(&cmd.board_id).bind(cmd.x).bind(cmd.y).bind(body).bind(&now)
        .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(BoardAnnotation {
            id,
            board_id: cmd.board_id.clone(),
            anchor_type: "board".to_owned(),
            x: Some(cmd.x),
            y: Some(cmd.y),
            node_id: None,
            nx: None,
            ny: None,
            author: String::new(),
            body: body.to_owned(),
            resolved: false,
            created_at: now,
        })
    }

    /// 解决/重开评论。
    pub async fn resolve_annotation(
        &self,
        project: &str,
        cmd: &ResolveAnnotationCommand,
    ) -> Result<()> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        let result = sqlx::query(
            "UPDATE board_annotations SET resolved=? WHERE id=? AND EXISTS(SELECT 1 FROM boards WHERE id=board_annotations.board_id AND project_id=?)",
        )
        .bind(if cmd.resolved { 1 } else { 0 })
        .bind(&cmd.annotation_id).bind(project)
        .execute(&mut *tx).await?;
        if result.rows_affected() == 0 {
            return Err(AppError::new("NOT_FOUND", "评论不存在"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// 查询画布评论。
    pub async fn get_annotations(&self, board_id: &str) -> Result<Vec<BoardAnnotation>> {
        let rows = sqlx::query(
            "SELECT id, board_id, anchor_type, x, y, node_id, nx, ny, author, body, resolved, created_at FROM board_annotations WHERE board_id=? ORDER BY created_at",
        )
        .bind(board_id)
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| BoardAnnotation {
                id: row.get("id"),
                board_id: row.get("board_id"),
                anchor_type: row.get("anchor_type"),
                x: row.get("x"),
                y: row.get("y"),
                node_id: row.get("node_id"),
                nx: row.get("nx"),
                ny: row.get("ny"),
                author: row.get("author"),
                body: row.get("body"),
                resolved: row.get::<i64, _>("resolved") != 0,
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

impl Store {
    pub async fn add_annotation(
        &self,
        project: &str,
        cmd: &AddAnnotationCommand,
    ) -> Result<BoardAnnotation> {
        CommandHandler::new(self).add_annotation(project, cmd).await
    }
    pub async fn resolve_annotation(
        &self,
        project: &str,
        cmd: &ResolveAnnotationCommand,
    ) -> Result<()> {
        CommandHandler::new(self)
            .resolve_annotation(project, cmd)
            .await
    }
    pub async fn get_annotations(&self, board_id: &str) -> Result<Vec<BoardAnnotation>> {
        CommandHandler::new(self).get_annotations(board_id).await
    }
}
