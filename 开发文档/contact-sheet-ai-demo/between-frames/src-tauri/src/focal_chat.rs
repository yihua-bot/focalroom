//! Focal AI 自由对话：类 GPT 的摄影助手体验。
//! 不走 Agent 任务引擎，直接调用 hosted 网关 `/responses`；
//! 授权、计量、频率限制全部由网关侧统一执行。
//! 上下文注入：项目 brief + 选中照片摘要。

use crate::{
    cloud_auth::CloudAuth,
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocalChatCommand {
    pub project_id: String,
    pub message: String,
    pub history: Vec<ChatTurn>,
    pub photo_ids: Vec<String>,
    /// 附加选中照片的预览图（多模态视觉分析）
    pub include_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocalChatReply {
    pub reply: String,
    pub suggestion: String,
}

const MAX_MESSAGE: usize = 4000;
const MAX_HISTORY: usize = 20;
const SYSTEM_PROMPT: &str = r#"你是 Focal AI——FocalRoom（本地优先的摄影创作工作台）内置的垂直摄影助手。

你的定位：像一位懂摄影的对话伙伴，不是命令机器人。
- 用自然对话回应：摄影选题、构图、色彩、编辑策略、叙事编排、印前准备
- 引用项目上下文时具体说（比如"你选中的第 3 张"），不编造不存在的照片
- 回复简洁（一般 200 字以内），跟随用户语言
- 不确定时如实说，不编造照片内容
- 引用照片时用确切文件名（如 IMG_2041.jpg）

回复格式：正常正文之后，如需建议下一步操作，在最后一行单独输出：
[[suggest:create_task]] 表示用户想选片/筛照片/确定最终组照
[[suggest:analyze_photos]] 表示用户想深入分析当前选中照片
没有建议时不输出任何标记。"#;

impl Store {
    pub async fn focal_chat(
        &self,
        auth: &Arc<CloudAuth>,
        cmd: &FocalChatCommand,
    ) -> Result<FocalChatReply> {
        let message = cmd.message.trim();
        if message.is_empty() || message.chars().count() > MAX_MESSAGE {
            return Err(AppError::new("VALIDATION_ERROR", "消息须为 1–4000 字符"));
        }

        // 项目上下文
        let brief: Option<String> =
            sqlx::query_scalar("SELECT brief FROM projects WHERE id=? AND deleted_at IS NULL")
                .bind(&cmd.project_id)
                .fetch_optional(&self.pool)
                .await?;
        let brief = brief.unwrap_or_default();

        let photo_rows: Vec<(String, String, i64, Option<String>)> = sqlx::query_as(
            "SELECT p.id, p.filename, d.rating, d.user_note FROM photos p JOIN photo_decisions d ON d.photo_id=p.id AND d.project_id=p.project_id WHERE p.project_id=? LIMIT 200",
        )
        .bind(&cmd.project_id)
        .fetch_all(&self.pool)
        .await?;
        let selected: std::collections::HashSet<&str> =
            cmd.photo_ids.iter().map(String::as_str).collect();
        let photo_ctx: Vec<String> = photo_rows
            .iter()
            .filter(|(id, _, _, _)| selected.contains(id.as_str()))
            .take(50)
            .map(|(_id, name, rating, note)| {
                let n = note
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("，备注：{s}"))
                    .unwrap_or_default();
                format!("- {name}（{rating} 星{n}）")
            })
            .collect();

        let ctx = json!({
            "项目简介": if brief.is_empty() { "（未填写）" } else { &brief },
            "选中照片": if photo_ctx.is_empty() { json!("（无）") } else { json!(photo_ctx) },
            "项目照片总数": photo_rows.len(),
        });

        // 构造 Responses API 请求（网关兼容格式）
        let mut input = vec![
            json!({"role": "system", "content": SYSTEM_PROMPT}),
            json!({"role": "system", "content": format!("项目上下文：{ctx}")}),
        ];
        for turn in cmd.history.iter().take(MAX_HISTORY) {
            input.push(json!({"role": turn.role, "content": turn.content}));
        }
        // 多模态：选中照片附预览图（最多 6 张，1400px JPEG）
        let mut user_content: Vec<serde_json::Value> = vec![json!({
            "type": "input_text", "text": message,
        })];
        if cmd.include_images && !cmd.photo_ids.is_empty() {
            let previews: Vec<(String, String)> = sqlx::query_as(
                "SELECT id, preview_path FROM photos WHERE project_id=? AND id IN (SELECT value FROM json_each(?)) LIMIT 6",
            )
            .bind(&cmd.project_id)
            .bind(serde_json::to_string(&cmd.photo_ids).unwrap_or_else(|_| "[]".into()))
            .fetch_all(&self.pool)
            .await?;
            for (pid, path) in &previews {
                if let Ok(data) = std::fs::read(path) {
                    let encoded =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);
                    user_content
                        .push(json!({"type": "input_text", "text": format!("photo_id: {pid}")}));
                    user_content.push(json!({"type": "input_image", "image_url": format!("data:image/jpeg;base64,{encoded}")}));
                }
            }
        }
        input.push(json!({"role": "user", "content": user_content}));

        let body = json!({
            "model": "deepseek-flash",
            "input": input,
            "max_output_tokens": 1024,
        });

        let origin = auth.origin().to_owned();
        let token = auth.bearer(&origin).await?;
        let url = format!("{origin}/api/v2/ai/deepseek-flash-v1/responses");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::new("AI_NOT_READY", e.to_string()))?;
        let response = client
            .post(&url)
            .bearer_auth(token)
            .header("x-focalroom-ai-policy", "deepseek-flash-v1")
            .header("x-focalroom-request-id", uuid::Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|_| AppError::new("AI_CONNECTION_FAILED", "Focal AI 连接中断，请稍后重试"))?;

        let status = response.status();
        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|_| AppError::new("AI_RESULT_UNKNOWN", "Focal AI 返回异常，请重试"))?;
        if !status.is_success() {
            let msg = payload["error"]["message"]
                .as_str()
                .unwrap_or("Focal AI 暂时不可用");
            return Err(AppError::new("AI_ERROR", msg.to_owned()));
        }

        // Responses API 输出格式：output[].content[].text
        let reply = payload["output"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|item| item["content"].as_array())
            .and_then(|c| c.first())
            .and_then(|t| t["text"].as_str())
            .unwrap_or("")
            .to_owned();
        if reply.is_empty() {
            return Err(AppError::new("AI_EMPTY", "Focal AI 未返回内容，请重试"));
        }

        // 意图识别（简单可靠：选片动词 + 数字）
        let mut reply_text = reply;
        let mut suggestion = String::new();
        for marker in ["create_task", "analyze_photos"] {
            let tag = format!("[[suggest:{marker}]]");
            if let Some(pos) = reply_text.find(&tag) {
                suggestion = marker.to_owned();
                reply_text = reply_text[..pos].trim_end().to_owned();
                break;
            }
        }
        if suggestion.is_empty() {
            let wants_task = ["选", "挑", "筛", "select", "choose", "pick"]
                .iter()
                .any(|k| message.contains(k))
                && message.chars().any(|c| c.is_ascii_digit());
            if wants_task {
                suggestion = "create_task".to_owned();
            }
        }

        // SQLite 持久化（发送前存 user，回复后存 assistant）
        let _ = self
            .save_message(&cmd.project_id, "user", message, "")
            .await;
        let _ = self
            .save_message(&cmd.project_id, "assistant", &reply_text, &suggestion)
            .await;

        Ok(FocalChatReply {
            reply: reply_text,
            suggestion,
        })
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn suggestion_logic() {
        // 纯数字选片意图
        let msg = "帮我选20张";
        let wants = ["选", "挑", "筛", "select", "choose", "pick"]
            .iter()
            .any(|k| msg.contains(k))
            && msg.chars().any(|c| c.is_ascii_digit());
        assert!(wants);
        // 纯对话
        let msg2 = "这张构图怎么样";
        let wants2 = ["选", "挑", "筛", "select", "choose", "pick"]
            .iter()
            .any(|k| msg2.contains(k))
            && msg2.chars().any(|c| c.is_ascii_digit());
        assert!(!wants2);
    }
}

// ── SQLite 持久化 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub suggestion: String,
    pub created_at: String,
}

impl Store {
    /// 读取项目对话历史（最近 N 条）。
    pub async fn focal_chat_history(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<StoredMessage>> {
        let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
            "SELECT id, role, content, suggestion, created_at FROM focal_chat_messages WHERE project_id=? ORDER BY id DESC LIMIT ?",
        )
        .bind(project_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        // 倒序取，正序返回
        Ok(rows
            .into_iter()
            .rev()
            .map(
                |(id, role, content, suggestion, created_at)| StoredMessage {
                    id,
                    role,
                    content,
                    suggestion,
                    created_at,
                },
            )
            .collect())
    }

    /// 清空项目对话。
    pub async fn focal_chat_clear(&self, project_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM focal_chat_messages WHERE project_id=?")
            .bind(project_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 写入单条消息（内部）。
    async fn save_message(
        &self,
        project_id: &str,
        role: &str,
        content: &str,
        suggestion: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO focal_chat_messages(project_id, role, content, suggestion, created_at) VALUES(?,?,?,?,?)",
        )
        .bind(project_id)
        .bind(role)
        .bind(content)
        .bind(suggestion)
        .bind(crate::domain::timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
