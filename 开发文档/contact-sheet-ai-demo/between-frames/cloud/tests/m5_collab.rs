//! M5 协作全量集成测试：评论/提案/审批/权限/并发冲突/快照恢复/AI ACL。
use reqwest::{Client, Response};
use serde_json::{json, Value};

fn val(r: Value, k: &str) -> String {
    r[k].as_str()
        .unwrap_or_else(|| panic!("字段 {k} 缺失: {r}"))
        .to_owned()
}

async fn spawn() -> (String, sqlx::SqlitePool) {
    let temp = tempfile::tempdir().unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let dir = temp.path().join("previews");
    std::fs::create_dir_all(&dir).unwrap();
    let router = bf_cloud::build_router(pool.clone(), dir).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    std::mem::forget(temp); // 保持 tempdir 存活
    (format!("http://{addr}"), pool)
}

async fn register(client: &Client, base: &str, email: &str) -> String {
    let resp = client
        .post(format!("{base}/auth/register"))
        .json(&json!({"email": email, "display_name": email, "password": "password123"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "注册失败");
    let v: Value = resp.json().await.unwrap();
    format!("Bearer {}", val(v, "token"))
}

async fn create_project(client: &Client, base: &str, token: &str) -> String {
    let resp = client
        .post(format!("{base}/projects"))
        .header("authorization", token)
        .json(&json!({"title": "M5 测试"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    val(v, "id")
}

async fn grant(
    client: &Client,
    base: &str,
    owner: &str,
    pid: &str,
    email: &str,
    role: &str,
) -> String {
    let resp = client
        .post(format!("{base}/projects/{pid}/grants"))
        .header("authorization", owner)
        .json(&json!({"grantee_email": email, "role": role}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "授权失败");
    let v: Value = resp.json().await.unwrap();
    val(v, "grant_id")
}

#[tokio::test]
async fn m5_full_collaboration_suite() {
    let (base, _pool) = spawn().await;
    let client = Client::new();

    // 三个用户：owner、editor（可审批）、reviewer（只能评论）。
    let owner = register(&client, &base, "owner@m5.dev").await;
    let editor = register(&client, &base, "editor@m5.dev").await;
    let reviewer = register(&client, &base, "reviewer@m5.dev").await;
    let outsider = register(&client, &base, "outsider@m5.dev").await;

    let pid = create_project(&client, &base, &owner).await;

    // 授权：editor 和 reviewer。
    let editor_grant_id = grant(&client, &base, &owner, &pid, "editor@m5.dev", "reviewer").await;
    grant(&client, &base, &owner, &pid, "reviewer@m5.dev", "viewer").await;

    // === C04: 评论坐标与线程 ===
    let sha = "a".repeat(64);
    let resp = client.post(format!("{base}/projects/{pid}/comments"))
        .header("authorization", &reviewer)
        .json(&json!({"photo_sha256": sha, "anchor_x": 0.5, "anchor_y": 0.3, "body": "这张构图偏左，主体空间不够"}))
        .send().await.unwrap();
    let dbg_status = resp.status().as_u16();
    let dbg_body = resp.text().await.unwrap_or_default();
    eprintln!("DEBUG comment resp: {dbg_status} {dbg_body}");
    assert_eq!(dbg_status, 200, "评论创建失败: {dbg_body}");
    let comment: Value = serde_json::from_str(&dbg_body).unwrap();
    let cid = val(comment.clone(), "id");
    assert_eq!(comment["anchor_x"], 0.5, "坐标保留");
    assert_eq!(comment["author"], "reviewer@m5.dev", "评论者身份保留");

    // 线程回复。
    let resp = client
        .post(format!("{base}/projects/{pid}/comments"))
        .header("authorization", &editor)
        .json(&json!({"body": "同意，裁切后可以改善", "parent_id": cid}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "回复创建失败");
    let reply: Value = resp.json().await.unwrap();
    assert_eq!(reply["parent_id"], cid, "线程关联正确");

    // 未授权者不能评论。
    let resp = client
        .post(format!("{base}/projects/{pid}/comments"))
        .header("authorization", &outsider)
        .json(&json!({"body": "未授权评论"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "未授权者不能评论");

    // resolve。
    let resp = client
        .post(format!("{base}/projects/{pid}/comments/{cid}/resolve"))
        .header("authorization", &owner)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "resolve 失败");

    // === C01: Reviewer 不能审批提案 ===
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals"))
        .header("authorization", &editor)
        .json(&json!({"title": "编辑提案", "base_revision": 0, "operations": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "提案创建失败");
    let proposal: Value = resp.json().await.unwrap();
    let prop_id = val(proposal.clone(), "proposal_id");
    let nonce = val(proposal, "approval_nonce");

    // C01: viewer 角色不能审批。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals/{prop_id}/decide"))
        .header("authorization", &reviewer) // viewer 角色
        .json(&json!({"approve": true, "approval_nonce": nonce}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "C01: viewer 不能审批");

    // outsider 不能审批。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals/{prop_id}/decide"))
        .header("authorization", &outsider)
        .json(&json!({"approve": true, "approval_nonce": nonce}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "C01: 未授权者不能审批");

    // === Owner 审批通过 ===
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals/{prop_id}/decide"))
        .header("authorization", &owner)
        .json(&json!({"approve": true, "approval_nonce": nonce}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "Owner 审批失败");
    let decided: Value = resp.json().await.unwrap();
    assert_eq!(decided["decided"], "applied");
    assert_eq!(decided["revision"], 1, "版本推进到 1");

    // === C02: 并发提案基线冲突 ===
    // editor 基于旧版本 v0 创建提案。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals"))
        .header("authorization", &editor)
        .json(&json!({"title": "基于旧版本的提案", "base_revision": 0, "operations": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409, "C02: 过时基线应返回冲突");
    let conflict: Value = resp.json().await.unwrap();
    assert!(
        conflict["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("过期"),
        "冲突信息明确"
    );

    // 基于新版本 v1 创建提案成功。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals"))
        .header("authorization", &editor)
        .json(&json!({"title": "基于新版本", "base_revision": 1, "operations": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let prop2: Value = resp.json().await.unwrap();
    let prop2_id = val(prop2.clone(), "proposal_id");
    let nonce2 = val(prop2, "approval_nonce");

    // Owner 先审批通过 prop2（推进到 v2）。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals/{prop2_id}/decide"))
        .header("authorization", &owner)
        .json(&json!({"approve": true, "approval_nonce": nonce2}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // 第三方（editor）再提交基于 v1 的提案 → 过时。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals"))
        .header("authorization", &editor)
        .json(&json!({"title": "再来一个旧基线", "base_revision": 1, "operations": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409, "C02: 提交时检测到基线过期");

    // === C03: 断线重连——快照恢复 ===
    let resp = client
        .get(format!("{base}/projects/{pid}/snapshot"))
        .header("authorization", &editor)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "快照获取失败");
    let snapshot: Value = resp.json().await.unwrap();
    assert_eq!(snapshot["revision"], 2, "C03: 权威版本 v2");
    assert!(snapshot["snapshot"].is_object(), "C03: 快照数据非空");

    // === AI 按权限获取评论 ===
    let resp = client
        .get(format!("{base}/ai/comments?project_id={pid}"))
        .header("authorization", &editor)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "AI 授权访问评论失败");
    let ai_comments: Value = resp.json().await.unwrap();
    assert!(
        ai_comments["comments"].as_array().unwrap().len() >= 1,
        "AI 获取评论"
    );
    assert!(
        ai_comments["comments"][0]["reviewer"].is_string(),
        "reviewer 身份保留"
    );

    // 未授权 AI 访问被拒。
    let resp = client
        .get(format!("{base}/ai/comments?project_id={pid}"))
        .header("authorization", &outsider)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "未授权 AI 访问被拒");

    // === 权限撤销后评论也拒 ===
    // Owner 撤销 editor 的访问。
    let resp = client
        .get(format!("{base}/projects/{pid}/previews"))
        .header("authorization", &owner)
        .send()
        .await
        .unwrap();
    // 先获取 grant 列表来撤销——简化：直接撤销所有。
    let _resp2 = client
        .delete(format!("{base}/projects/{pid}/grants/{editor_grant_id}"))
        .header("authorization", &owner)
        .send()
        .await;
    // 撤销后 editor 不能再评论。
    let resp3 = client
        .post(format!("{base}/projects/{pid}/comments"))
        .header("authorization", &editor)
        .json(&json!({"body": "撤销后评论"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp3.status(), 403, "撤销后评论被拒");
}

#[tokio::test]
async fn m5_wrong_nonce_rejected() {
    let (base, _) = spawn().await;
    let client = Client::new();
    let owner = register(&client, &base, "owner@nonce.dev").await;
    let pid = create_project(&client, &base, &owner).await;

    let resp = client
        .post(format!("{base}/projects/{pid}/proposals"))
        .header("authorization", &owner)
        .json(&json!({"title": "测试", "base_revision": 0, "operations": []}))
        .send()
        .await
        .unwrap();
    let prop: Value = resp.json().await.unwrap();
    let prop_id = val(prop.clone(), "proposal_id");

    // 错误 nonce 被拒。
    let resp = client
        .post(format!("{base}/projects/{pid}/proposals/{prop_id}/decide"))
        .header("authorization", &owner)
        .json(&json!({"approve": true, "approval_nonce": "wrong-nonce"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "错误 nonce 必须被拒");
}
