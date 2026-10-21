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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasPhotoSource {
    pub asset_id: String,
    pub thumbnail_path: String,
}

fn validate_rectangle(x: f64, y: f64, width: f64, height: f64) -> Result<()> {
    if ![x, y, width, height]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1e9)
        || width <= 0.0
        || height <= 0.0
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "节点位置须为有限数值，尺寸须大于零",
        ));
    }
    Ok(())
}

impl Store {
    /// 只提供画布所属项目中已关联照片的缓存路径，不暴露任意文件读取入口。
    pub async fn board_photo_sources(&self, board_id: &str) -> Result<Vec<CanvasPhotoSource>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT a.id, MIN(p.thumbnail_path) FROM boards b JOIN canvas_nodes n ON n.board_id=b.id JOIN source_assets a ON a.id=n.asset_id AND a.project_id=b.project_id JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id AND p.project_id=b.project_id WHERE b.id=? AND n.node_type='photo' AND n.visible=1 GROUP BY a.id",
        ).bind(board_id).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|(asset_id, thumbnail_path)| CanvasPhotoSource {
                asset_id,
                thumbnail_path,
            })
            .collect())
    }
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
        validate_rectangle(cmd.x, cmd.y, cmd.width, cmd.height)?;
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
            "SELECT id FROM source_assets WHERE project_id=? AND (id=? OR id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?))",
        )
        .bind(project).bind(&cmd.asset_id).bind(&cmd.asset_id)
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
        // H0-07：记录撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "add_node",
            &serde_json::json!({"nodeId": id, "nodeType": "photo"}),
            &serde_json::json!({"nodeId": id}),
        )
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
        // H0-07：记录撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "move",
            &serde_json::json!({"nodeIds": cmd.node_ids, "dx": cmd.dx, "dy": cmd.dy}),
            &serde_json::json!({"nodeIds": cmd.node_ids, "dx": cmd.dx, "dy": cmd.dy}),
        )
        .await?;
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

    #[tokio::test]
    async fn text_arrow_and_ink_persist_correct_geometry_style_and_order() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("node-types.sqlite");
        let db = Store::open(&path).await.unwrap();
        let project = db.create("叙事", "").await.unwrap();
        let board = db.create_board(&project.id, "第一章").await.unwrap();
        let text = db
            .add_text_node(
                &project.id,
                &AddTextNodeCommand {
                    board_id: board.id.clone(),
                    node_type: "sticky".to_owned(),
                    x: 20.0,
                    y: 30.0,
                    width: 200.0,
                    height: 120.0,
                    content: "停顿与转场".to_owned(),
                    color: Some("quoted\"color".to_owned()),
                },
            )
            .await
            .unwrap();
        let ink = db
            .add_ink(
                &project.id,
                &AddInkCommand {
                    board_id: board.id.clone(),
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 0.0,
                    points: vec![
                        InkPoint {
                            x: 0.0,
                            y: 0.0,
                            pressure: None,
                        },
                        InkPoint {
                            x: 100.0,
                            y: 0.0,
                            pressure: Some(0.5),
                        },
                    ],
                    color: None,
                    stroke_width: Some(2.0),
                },
            )
            .await
            .unwrap();
        let arrow = db
            .add_arrow(
                &project.id,
                &AddArrowCommand {
                    board_id: board.id.clone(),
                    from_node_id: text.id.clone(),
                    to_node_id: ink.id.clone(),
                    color: None,
                },
            )
            .await
            .unwrap();
        db.pool.close().await;
        let reopened = Store::open(&path).await.unwrap();
        let nodes = reopened.get_board_nodes(&board.id).await.unwrap();
        assert_eq!(
            nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>(),
            vec![text.id, ink.id, arrow.id]
        );
        assert_eq!(
            nodes.iter().map(|n| n.z_order).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(nodes.iter().all(|n| n.rotation == 0.0));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(nodes[0].style_json.as_ref().unwrap())
                .unwrap()["color"],
            "quoted\"color"
        );
    }

    #[tokio::test]
    async fn foreign_assets_and_invalid_photo_geometry_are_rejected_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("node-scopes.sqlite"))
            .await
            .unwrap();
        let project = db.create("项目甲", "").await.unwrap();
        let other = db.create("项目乙", "").await.unwrap();
        let board = db.create_board(&project.id, "画布").await.unwrap();
        sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES('foreign',?,'hash','[]',100,100,100,'jpeg','now')")
            .bind(&other.id).execute(&db.pool).await.unwrap();
        let mut command = PlacePhotoNodeCommand {
            board_id: board.id.clone(),
            asset_id: "foreign".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert_eq!(
            db.place_photo_node(&project.id, &command)
                .await
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
        for value in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            command.width = value;
            assert_eq!(
                db.place_photo_node(&project.id, &command)
                    .await
                    .unwrap_err()
                    .code,
                "VALIDATION_ERROR"
            );
        }
        assert!(db.get_board_nodes(&board.id).await.unwrap().is_empty());
        assert!(db.board_photo_sources(&board.id).await.unwrap().is_empty());
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
        validate_rectangle(cmd.x, cmd.y, cmd.width, cmd.height)?;
        if cmd.content.chars().count() > 5000 {
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
        let style_json = cmd
            .color
            .as_ref()
            .map(|c| serde_json::json!({ "color": c }).to_string());
        sqlx::query(
            "INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,content_json,style_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,0,?,?,?,?,?)",
        )
        .bind(&id).bind(&cmd.board_id).bind(&cmd.node_type)
        .bind(cmd.x).bind(cmd.y).bind(cmd.width).bind(cmd.height)
        .bind(z).bind(&content_json).bind(&style_json)
        .bind(&now).bind(&now)
        .execute(&mut *tx).await?;
        // H0-07：撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "add_node",
            &serde_json::json!({"nodeId": id, "nodeType": cmd.node_type}),
            &serde_json::json!({"nodeId": id}),
        )
        .await?;
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
        // H0-07：记录撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "delete_node",
            &serde_json::json!({"nodeId": cmd.node_id}),
            &serde_json::json!({"nodeId": cmd.node_id}),
        )
        .await?;
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

// ── Arrow Connector（箭头连接）──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddArrowCommand {
    pub board_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub color: Option<String>,
}

impl CommandHandler<'_> {
    /// 添加箭头连接（from → to，节点移动时端点跟随）。
    pub async fn add_arrow(&self, project: &str, cmd: &AddArrowCommand) -> Result<CanvasNode> {
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
        // 验证两端节点存在于同一画布
        for node_id in [&cmd.from_node_id, &cmd.to_node_id] {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM canvas_nodes WHERE id=? AND board_id=? AND visible=1",
            )
            .bind(node_id)
            .bind(&cmd.board_id)
            .fetch_one(&mut *tx)
            .await?;
            if exists == 0 {
                return Err(AppError::new("NOT_FOUND", format!("节点 {node_id} 不存在")));
            }
        }
        let id = domain::id();
        let now = domain::timestamp();
        let z: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(z_order),0)+1 FROM canvas_nodes WHERE board_id=?",
        )
        .bind(&cmd.board_id)
        .fetch_one(&mut *tx)
        .await?;
        // 箭头的 children_json 存 [from_id, to_id]；style_json 存颜色
        let children =
            serde_json::to_string(&serde_json::json!([cmd.from_node_id, cmd.to_node_id]))
                .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        let style = cmd
            .color
            .as_ref()
            .map(|c| serde_json::json!({ "color": c }).to_string());
        sqlx::query(
            "INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,style_json,children_json,created_at,updated_at) VALUES(?,?, 'arrow',0,0,0,0,0,?,?,?,?,?)",
        )
        .bind(&id).bind(&cmd.board_id).bind(z).bind(&style).bind(&children)
        .bind(&now).bind(&now)
        .execute(&mut *tx).await?;
        // H0-07：撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "add_node",
            &serde_json::json!({"nodeId": id, "nodeType": "arrow"}),
            &serde_json::json!({"nodeId": id}),
        )
        .await?;
        tx.commit().await?;
        Ok(CanvasNode {
            id,
            board_id: cmd.board_id.clone(),
            node_type: "arrow".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            rotation: 0.0,
            z_order: z,
            asset_id: None,
            rendition_ref: None,
            content_json: None,
            style_json: style,
            children_json: Some(children),
            locked: false,
            visible: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

impl Store {
    pub async fn add_arrow(&self, project: &str, cmd: &AddArrowCommand) -> Result<CanvasNode> {
        CommandHandler::new(self).add_arrow(project, cmd).await
    }
}

// ── Canvas Undo（画布撤销）──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoCanvasCommand {
    pub board_id: String,
}

impl CommandHandler<'_> {
    /// 写撤销日志（H0-07：画布操作的逆操作记录，append-only）。
    async fn log_canvas_op(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        project: &str,
        board_id: &str,
        op_type: &str,
        op_data: &serde_json::Value,
        undo_data: &serde_json::Value,
    ) -> Result<()> {
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query(
            "INSERT INTO canvas_op_log(id,project_id,board_id,op_type,op_data,undo_data,undone,created_at) VALUES(?,?,?,?,?,?,0,?)",
        )
        .bind(&id).bind(project).bind(board_id).bind(op_type)
        .bind(op_data.to_string()).bind(undo_data.to_string()).bind(&now)
        .execute(&mut **tx).await?;
        Ok(())
    }

    /// 撤销画布上最后一次操作（H0-07：从 canvas_op_log 读取，不修改 command_events）。
    pub async fn undo_canvas(&self, project: &str, cmd: &UndoCanvasCommand) -> Result<bool> {
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

        // 从 canvas_op_log 读取最近一个未撤销的操作（append-only 查询，不修改审计事件）
        let last: Option<(String, String, String)> = sqlx::query_as(
            "SELECT id, op_type, undo_data FROM canvas_op_log WHERE board_id=? AND undone=0 ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(&cmd.board_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((log_id, op_type, undo_json)) = last else {
            return Ok(false); // 无可撤销操作
        };

        let undo: serde_json::Value = serde_json::from_str(&undo_json)
            .map_err(|e| AppError::new("VALIDATION_ERROR", format!("撤销数据格式错误：{e}")))?;

        let now = domain::timestamp();
        match op_type.as_str() {
            "move" => {
                let node_ids: Vec<String> = undo["nodeIds"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let dx = undo["dx"].as_f64().unwrap_or(0.0);
                let dy = undo["dy"].as_f64().unwrap_or(0.0);
                // H0 复核：锁定节点导致部分恢复 → 原子失败（不消费撤销记录）
                for node_id in &node_ids {
                    let result = sqlx::query(
                        "UPDATE canvas_nodes SET x=x-?, y=y-?, updated_at=? WHERE id=? AND board_id=? AND locked=0",
                    )
                    .bind(dx).bind(dy).bind(&now).bind(node_id).bind(&cmd.board_id)
                    .execute(&mut *tx).await?;
                    if result.rows_affected() == 0 {
                        let exists: i64 = sqlx::query_scalar(
                            "SELECT COUNT(*) FROM canvas_nodes WHERE id=? AND board_id=?",
                        )
                        .bind(node_id)
                        .bind(&cmd.board_id)
                        .fetch_one(&mut *tx)
                        .await?;
                        if exists > 0 {
                            return Err(AppError::new(
                                "CONFLICT",
                                format!("节点 {node_id} 已锁定，无法撤销移动"),
                            ));
                        }
                    }
                }
            }
            "add_node" => {
                let node_id = undo["nodeId"].as_str().unwrap_or("");
                if !node_id.is_empty() {
                    sqlx::query(
                        "UPDATE canvas_nodes SET visible=0, updated_at=? WHERE id=? AND board_id=?",
                    )
                    .bind(&now)
                    .bind(node_id)
                    .bind(&cmd.board_id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            "delete_node" => {
                let node_id = undo["nodeId"].as_str().unwrap_or("");
                if !node_id.is_empty() {
                    sqlx::query(
                        "UPDATE canvas_nodes SET visible=1, updated_at=? WHERE id=? AND board_id=?",
                    )
                    .bind(&now)
                    .bind(node_id)
                    .bind(&cmd.board_id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            _ => return Ok(false),
        }

        // 在 op_log 中标记已撤销（不是审计事件，是私有撤销栈）
        sqlx::query("UPDATE canvas_op_log SET undone=1, redone=0 WHERE id=?")
            .bind(&log_id)
            .execute(&mut *tx)
            .await?;
        // 栈语义：新撤销丢弃其后所有 redo 分支
        sqlx::query(
            "UPDATE canvas_op_log SET redone=0 WHERE board_id=? AND created_at < (SELECT created_at FROM canvas_op_log WHERE id=?) AND redone=1",
        )
        .bind(&cmd.board_id)
        .bind(&log_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }
}

impl Store {
    pub async fn undo_canvas(&self, project: &str, cmd: &UndoCanvasCommand) -> Result<bool> {
        CommandHandler::new(self).undo_canvas(project, cmd).await
    }
}

// ── Ink（画笔/涂鸦）──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddInkCommand {
    pub board_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// 矢量路径点（Board 坐标系）：[{x,y,pressure}, ...]
    pub points: Vec<InkPoint>,
    pub color: Option<String>,
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InkPoint {
    pub x: f64,
    pub y: f64,
    pub pressure: Option<f64>,
}

impl CommandHandler<'_> {
    /// 添加画笔笔迹（矢量路径点，存储在 Board 坐标系）。
    pub async fn add_ink(&self, project: &str, cmd: &AddInkCommand) -> Result<CanvasNode> {
        if cmd.points.is_empty() || cmd.points.len() > 10_000 {
            return Err(AppError::new("VALIDATION_ERROR", "笔迹须为 1–10000 个点"));
        }
        if ![cmd.x, cmd.y, cmd.width, cmd.height]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1e9)
            || cmd.width < 0.0
            || cmd.height < 0.0
            || cmd
                .stroke_width
                .is_some_and(|v| !v.is_finite() || !(0.1..=1000.0).contains(&v))
            || cmd.points.iter().any(|point| {
                !point.x.is_finite()
                    || !point.y.is_finite()
                    || point
                        .pressure
                        .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            })
        {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "笔迹的位置、压力或线宽无效",
            ));
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
        // content_json 存路径点；style_json 存颜色/线宽
        let content = serde_json::to_string(&serde_json::json!({
            "points": cmd.points,
        }))
        .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        let style = serde_json::to_string(&serde_json::json!({
            "color": cmd.color.clone().unwrap_or_else(|| "#ff6b6b".to_owned()),
            "strokeWidth": cmd.stroke_width.unwrap_or(2.0),
        }))
        .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        sqlx::query(
            "INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,content_json,style_json,created_at,updated_at) VALUES(?,?, 'ink',?,?,?,?,0,?,?,?,?,?)",
        )
        .bind(&id).bind(&cmd.board_id)
        .bind(cmd.x).bind(cmd.y).bind(cmd.width).bind(cmd.height)
        .bind(z).bind(&content).bind(&style)
        .bind(&now).bind(&now)
        .execute(&mut *tx).await?;
        // H0-07：撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "add_node",
            &serde_json::json!({"nodeId": id, "nodeType": "ink"}),
            &serde_json::json!({"nodeId": id}),
        )
        .await?;
        tx.commit().await?;
        Ok(CanvasNode {
            id,
            board_id: cmd.board_id.clone(),
            node_type: "ink".to_owned(),
            x: cmd.x,
            y: cmd.y,
            width: cmd.width,
            height: cmd.height,
            rotation: 0.0,
            z_order: z,
            asset_id: None,
            rendition_ref: None,
            content_json: Some(content),
            style_json: Some(style),
            children_json: None,
            locked: false,
            visible: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

impl Store {
    pub async fn add_ink(&self, project: &str, cmd: &AddInkCommand) -> Result<CanvasNode> {
        CommandHandler::new(self).add_ink(project, cmd).await
    }
}

// ── H0-03：人工采纳任务草案 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptBoardDraftCommand {
    pub board_id: String,
}

impl CommandHandler<'_> {
    /// 人工采纳 Agent 任务草案：清空 owning_task_id，画布进入人工编辑域。
    /// 这是人类正式命令（经 CommandHandler 通道），不是 Agent 工具。
    pub async fn adopt_board_draft(
        &self,
        project: &str,
        cmd: &AdoptBoardDraftCommand,
    ) -> Result<()> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let result = sqlx::query(
            "UPDATE boards SET owning_task_id=NULL, updated_at=? WHERE id=? AND project_id=? AND owning_task_id IS NOT NULL",
        )
        .bind(domain::timestamp())
        .bind(&cmd.board_id)
        .bind(project)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::new(
                "NOT_FOUND",
                "画布不存在、不属于此项目或不是任务草案",
            ));
        }
        // 记录撤销日志（采纳不可逆——人工决策不自动回退）
        tx.commit().await?;
        Ok(())
    }
}

impl Store {
    pub async fn adopt_board_draft(
        &self,
        project: &str,
        cmd: &AdoptBoardDraftCommand,
    ) -> Result<()> {
        CommandHandler::new(self)
            .adopt_board_draft(project, cmd)
            .await
    }
}

#[cfg(test)]
mod task_owned_tests {
    use super::*;

    #[tokio::test]
    async fn agent_board_is_task_owned_and_adopt_clears_owner() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(temp.path().join("task-owned.sqlite").as_path())
            .await
            .unwrap();
        let project = db.create("task-owned", "test").await.unwrap();

        // 人工创建画布（无 owning_task_id）
        let human_board = db.create_board(&project.id, "人工画布").await.unwrap();

        // Agent 任务专属画布（模拟 Agent 写入）
        let task_id = domain::id();
        let agent_board_id = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&task_id)
            .bind(&project.id)
            .bind("测试任务")
            .bind(1)
            .bind("[]")
            .bind("[]")
            .bind(1)
            .bind("completed")
            .bind("测试")
            .bind("{}")
            .bind("{}")
            .bind(&now)
            .bind(&now)
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO boards(id,project_id,title,state,owning_task_id,created_at,updated_at) VALUES(?,?, 'Agent 草案','draft',?,?,?)")
            .bind(&agent_board_id).bind(&project.id).bind(&task_id).bind(&now).bind(&now)
            .execute(&db.pool)
            .await
            .unwrap();

        // 人工采纳：清空 owning_task_id
        db.adopt_board_draft(
            &project.id,
            &AdoptBoardDraftCommand {
                board_id: agent_board_id.clone(),
            },
        )
        .await
        .unwrap();

        // 验证归属已清空
        let owner: Option<String> =
            sqlx::query_scalar("SELECT owning_task_id FROM boards WHERE id=?")
                .bind(&agent_board_id)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert!(owner.is_none(), "采纳后 owning_task_id 应为 NULL");

        // 人工画布不能被"采纳"（本来就无任务归属）
        let result = db
            .adopt_board_draft(
                &project.id,
                &AdoptBoardDraftCommand {
                    board_id: human_board.id.clone(),
                },
            )
            .await;
        assert!(result.is_err(), "人工画布不可被采纳");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedoCanvasCommand {
    pub board_id: String,
}

impl CommandHandler<'_> {
    /// H0-07：重做画布上最后一次被撤销的操作。
    /// 栈语义：最近一条 undone=1 且 redone=0 的记录可重做。
    pub async fn redo_canvas(&self, project: &str, cmd: &RedoCanvasCommand) -> Result<bool> {
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

        // 找最近一条已撤销且未重做的操作
        let last: Option<(String, String, String)> = sqlx::query_as(
            "SELECT id, op_type, undo_data FROM canvas_op_log WHERE board_id=? AND undone=1 AND redone=0 ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(&cmd.board_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((log_id, op_type, undo_json)) = last else {
            return Ok(false);
        };
        let undo: serde_json::Value = serde_json::from_str(&undo_json)
            .map_err(|e| AppError::new("VALIDATION_ERROR", format!("重做数据格式错误：{e}")))?;

        let now = domain::timestamp();
        match op_type.as_str() {
            "move" => {
                let node_ids: Vec<String> = undo["nodeIds"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let dx = undo["dx"].as_f64().unwrap_or(0.0);
                let dy = undo["dy"].as_f64().unwrap_or(0.0);
                for node_id in &node_ids {
                    sqlx::query("UPDATE canvas_nodes SET x=x+?, y=y+?, updated_at=? WHERE id=? AND board_id=? AND locked=0 AND visible=1")
                        .bind(dx).bind(dy).bind(&now).bind(node_id).bind(&cmd.board_id)
                        .execute(&mut *tx).await?;
                }
            }
            "add_node" => {
                let node_id = undo["nodeId"].as_str().unwrap_or("");
                if !node_id.is_empty() {
                    sqlx::query(
                        "UPDATE canvas_nodes SET visible=1, updated_at=? WHERE id=? AND board_id=?",
                    )
                    .bind(&now)
                    .bind(node_id)
                    .bind(&cmd.board_id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            "delete_node" => {
                let node_id = undo["nodeId"].as_str().unwrap_or("");
                if !node_id.is_empty() {
                    sqlx::query(
                        "UPDATE canvas_nodes SET visible=0, updated_at=? WHERE id=? AND board_id=?",
                    )
                    .bind(&now)
                    .bind(node_id)
                    .bind(&cmd.board_id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            _ => return Ok(false),
        }

        sqlx::query("UPDATE canvas_op_log SET undone=0, redone=1 WHERE id=?")
            .bind(&log_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }
}

impl Store {
    pub async fn redo_canvas(&self, project: &str, cmd: &RedoCanvasCommand) -> Result<bool> {
        CommandHandler::new(self).redo_canvas(project, cmd).await
    }
}

#[cfg(test)]
mod g1_representation_tests {
    use super::*;

    #[tokio::test]
    async fn backfill_uses_real_cache_dimensions_not_quarter() {
        // G1 修复回归：回填的 representation 尺寸必须来自真实缓存文件，
        // 不是 width/4 猜测。用 1600x900 源图验证。
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(temp.path().join("g1-dim.sqlite").as_path())
            .await
            .unwrap();
        let project = db.create("g1-dim", "test").await.unwrap();

        // 生成 1600x900 测试图（宽高比 16:9，缩略图应为 320x180 而非 400x225）
        let img_path = temp.path().join("wide.png");
        image::RgbImage::from_pixel(1600, 900, image::Rgb([50, 100, 150]))
            .save(&img_path)
            .unwrap();
        let prepared =
            crate::image_pipeline::prepare(&img_path, &temp.path().join("cache")).unwrap();

        // 走正式导入
        let inserted = db.insert_photo(&project.id, &prepared).await.unwrap();
        assert!(inserted, "首次导入应成功");

        // 验证 representation 尺寸来自真实文件
        let reps: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT kind, width, height FROM asset_representations WHERE asset_id=(SELECT id FROM source_assets LIMIT 1) ORDER BY kind",
        )
        .fetch_all(&db.pool)
        .await
        .unwrap();

        assert_eq!(reps.len(), 2, "应有 thumbnail + display_proxy 两条");
        // thumbnail：320x180（16:9 适配 320px 宽，不是 1600/4=400）
        let thumb = reps.iter().find(|(k, _, _)| k == "thumbnail").unwrap();
        assert_eq!(
            thumb.1, 320,
            "缩略图宽度应为 320（真实缓存）而非 width/4=400"
        );
        assert_eq!(thumb.2, 180, "缩略图高度应为 180（保持宽高比）");
        // display_proxy：1400x788（16:9 适配 1400px 宽）
        let proxy = reps.iter().find(|(k, _, _)| k == "display_proxy").unwrap();
        assert_eq!(proxy.1, 1400, "代理宽度应为 1400");
    }
}

// ── G3：用户分组命令 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupNodesCommand {
    pub board_id: String,
    pub node_ids: Vec<String>,
    pub label: String,
}

impl CommandHandler<'_> {
    /// 用户分组：把选中节点记录到一个 group 节点的 children 里。
    /// 分组本身也是一个节点（可整体拖动/删除）。
    pub async fn group_nodes_cmd(
        &self,
        project: &str,
        cmd: &GroupNodesCommand,
    ) -> Result<CanvasNode> {
        if cmd.node_ids.is_empty() || cmd.node_ids.len() > 100 {
            return Err(AppError::new("VALIDATION_ERROR", "分组须包含 1–100 个节点"));
        }
        let label = cmd.label.trim();
        if label.is_empty() || label.chars().count() > 50 {
            return Err(AppError::new("VALIDATION_ERROR", "分组标签须为 1–50 字符"));
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
        // 验证所有子节点存在且未锁定
        for nid in &cmd.node_ids {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM canvas_nodes WHERE id=? AND board_id=? AND visible=1 AND locked=0",
            )
            .bind(nid)
            .bind(&cmd.board_id)
            .fetch_one(&mut *tx)
            .await?;
            if exists == 0 {
                return Err(AppError::new(
                    "NOT_FOUND",
                    format!("节点 {nid} 不存在或已锁定"),
                ));
            }
        }
        // 计算包围盒
        let bbox: Option<(f64, f64, f64, f64)> = sqlx::query_as(
            "SELECT MIN(x), MIN(y), MAX(x+width), MAX(y+height) FROM canvas_nodes WHERE board_id=? AND id IN (SELECT value FROM json_each(?))",
        )
        .bind(&cmd.board_id)
        .bind(serde_json::to_string(&cmd.node_ids).unwrap_or_default())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((min_x, min_y, max_x, max_y)) = bbox else {
            return Err(AppError::new("VALIDATION_ERROR", "无法计算分组包围盒"));
        };
        let id = domain::id();
        let now = domain::timestamp();
        let z: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(z_order),0)+1 FROM canvas_nodes WHERE board_id=?",
        )
        .bind(&cmd.board_id)
        .fetch_one(&mut *tx)
        .await?;
        let content = serde_json::json!({"label": label}).to_string();
        let children = serde_json::to_string(&cmd.node_ids)
            .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        sqlx::query(
            "INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,content_json,children_json,created_at,updated_at) VALUES(?,?, 'group',?,?,?,?,0,?,?,?,?)",
        )
        .bind(&id)
        .bind(&cmd.board_id)
        .bind(min_x)
        .bind(min_y)
        .bind(max_x - min_x)
        .bind(max_y - min_y)
        .bind(z)
        .bind(&content)
        .bind(&children)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        // 撤销日志
        Self::log_canvas_op(
            &mut tx,
            project,
            &cmd.board_id,
            "add_node",
            &serde_json::json!({"nodeId": id, "nodeType": "group"}),
            &serde_json::json!({"nodeId": id}),
        )
        .await?;
        tx.commit().await?;
        Ok(CanvasNode {
            id,
            board_id: cmd.board_id.clone(),
            node_type: "group".to_owned(),
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
            rotation: 0.0,
            z_order: z,
            asset_id: None,
            rendition_ref: None,
            content_json: Some(content),
            style_json: None,
            children_json: Some(children),
            locked: false,
            visible: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

impl Store {
    pub async fn group_nodes_cmd(
        &self,
        project: &str,
        cmd: &GroupNodesCommand,
    ) -> Result<CanvasNode> {
        CommandHandler::new(self)
            .group_nodes_cmd(project, cmd)
            .await
    }
}

#[cfg(test)]
mod board_revision_tests {
    use super::*;

    #[tokio::test]
    async fn board_revision_bumps_on_node_writes() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(temp.path().join("rev-test.sqlite").as_path())
            .await
            .unwrap();
        let project = db.create("rev-test", "revision").await.unwrap();
        let board = db.create_board(&project.id, "rev").await.unwrap();

        // 初始 revision = 1
        let rev0: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
            .bind(&board.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(rev0, 1, "新画布 revision=1");

        // 插入照片资产+节点 → revision 递增
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES(?,?,?, '[]',10,10,10,'jpeg',?)")
            .bind(&id).bind(&project.id).bind("sha-rev").bind(&now)
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

        let rev1: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
            .bind(&board.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(rev1, 2, "插入节点后 revision=2");

        // 移动 → 再递增
        db.move_nodes(
            &project.id,
            &MoveNodesCommand {
                board_id: board.id.clone(),
                node_ids: vec![node.id.clone()],
                dx: 10.0,
                dy: 10.0,
            },
        )
        .await
        .unwrap();

        let rev2: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
            .bind(&board.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(rev2, 3, "移动后 revision=3");

        // 删除 → 再递增
        db.delete_node(
            &project.id,
            &DeleteNodeCommand {
                board_id: board.id.clone(),
                node_id: node.id.clone(),
            },
        )
        .await
        .unwrap();

        let rev3: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
            .bind(&board.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(rev3, 4, "删除后 revision=4");
    }
}

// ── G3：节点 Resize 与锁定 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeNodesCommand {
    pub board_id: String,
    /// 单节点 resize（多节点独立 resize 无明确语义，不支持）
    pub node_id: String,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeLockedCommand {
    pub board_id: String,
    pub node_id: String,
    pub locked: bool,
}

impl CommandHandler<'_> {
    /// 调整节点尺寸（最小 20x20，最大 10000x10000）。
    pub async fn resize_node(&self, project: &str, cmd: &ResizeNodesCommand) -> Result<()> {
        if cmd.width < 20.0 || cmd.height < 20.0 || cmd.width > 10000.0 || cmd.height > 10000.0 {
            return Err(AppError::new("VALIDATION_ERROR", "尺寸须在 20–10000 之间"));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let result = sqlx::query(
            "UPDATE canvas_nodes SET width=?, height=?, updated_at=? WHERE id=? AND board_id=? AND locked=0 AND EXISTS(SELECT 1 FROM boards WHERE id=? AND project_id=?)",
        )
        .bind(cmd.width).bind(cmd.height).bind(domain::timestamp())
        .bind(&cmd.node_id).bind(&cmd.board_id).bind(&cmd.board_id).bind(project)
        .execute(&mut *tx).await?;
        if result.rows_affected() == 0 {
            return Err(AppError::new("NOT_FOUND", "节点不存在或已锁定"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// 锁定/解锁节点（锁定后不可拖动/resize/删除）。
    pub async fn set_node_locked(&self, project: &str, cmd: &SetNodeLockedCommand) -> Result<()> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let result = sqlx::query(
            "UPDATE canvas_nodes SET locked=?, updated_at=? WHERE id=? AND board_id=? AND EXISTS(SELECT 1 FROM boards WHERE id=? AND project_id=?)",
        )
        .bind(if cmd.locked { 1 } else { 0 })
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
    pub async fn resize_node(&self, project: &str, cmd: &ResizeNodesCommand) -> Result<()> {
        CommandHandler::new(self).resize_node(project, cmd).await
    }
    pub async fn set_node_locked(&self, project: &str, cmd: &SetNodeLockedCommand) -> Result<()> {
        CommandHandler::new(self)
            .set_node_locked(project, cmd)
            .await
    }
}
