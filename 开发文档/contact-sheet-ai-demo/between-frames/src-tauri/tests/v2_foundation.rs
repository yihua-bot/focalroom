use betweenframes_local_domain::{
    Actor, AuthorityTarget, Principal, ProjectAuthority, Role, dispatch, parse_command, schema,
};
use serde_json::{Value, json};

fn principal(value: &Value) -> Principal {
    let id = value["id"].as_str().unwrap().to_owned();
    let actor = if value["type"] == "agent" {
        Actor::Agent {
            id,
            expires_at_ms: value["expires_at_ms"].as_u64().unwrap(),
        }
    } else {
        Actor::User {
            id,
            role: serde_json::from_value::<Role>(value["role"].clone()).unwrap(),
            can_approve: value["can_approve"].as_bool().unwrap(),
            can_edit_formal: value["can_edit_formal"].as_bool().unwrap(),
            can_propose: value["can_propose"].as_bool().unwrap(),
        }
    };
    Principal {
        workspace_id: value["workspace_id"].as_str().unwrap().into(),
        project_id: value["project_id"].as_str().unwrap().into(),
        actor,
    }
}

#[test]
fn ts_rust_command_fixtures_agree() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../packages/contracts/fixtures/commands.json"
    ))
    .unwrap();
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        assert_eq!(
            parse_command(fixture["command"].clone()).is_ok(),
            fixture["valid"].as_bool().unwrap(),
            "{}",
            fixture["name"]
        );
    }
}

#[test]
fn ts_rust_permission_fixtures_agree_and_never_write() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../packages/contracts/fixtures/permissions.json"
    ))
    .unwrap();
    assert_eq!(fixtures.len(), 21);
    for fixture in fixtures {
        let project: ProjectAuthority = serde_json::from_value(fixture["project"].clone()).unwrap();
        let target = if fixture["target"] == "LOCAL" {
            AuthorityTarget::Local
        } else {
            AuthorityTarget::Cloud
        };
        let reply = dispatch(
            fixture["command"].clone(),
            &principal(&fixture["principal"]),
            &project,
            target,
            fixture["now"].as_u64().unwrap(),
        );
        assert_eq!(reply["ok"], false, "M0 不得伪造成功");
        assert_eq!(
            reply["error"]["code"], fixture["code"],
            "{}",
            fixture["name"]
        );
    }
}

#[test]
fn unknown_schema_constraints_fail_closed() {
    assert!(!schema::valid(
        &json!({"type":"string","pattern":"^private$"}),
        &json!("other")
    ));
    assert!(!schema::valid(
        &json!({"type":"number","minimum":0,"maximum":1}),
        &json!(1.1)
    ));
    assert!(!schema::valid(&json!({"type":"integer"}), &json!(0.5)));
    assert!(schema::valid(
        &json!({"type":["string","null"]}),
        &Value::Null
    ));
}

#[tokio::test]
async fn existing_local_schema_remains_authoritative_and_transactional() {
    use between_frames_lib::db::Store;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("projects.sqlite");
    let db = Store::open(&path).await.unwrap();
    let project = db.create("v2 兼容样例", "保留本地权威").await.unwrap();
    let migrations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        migrations, 31,
        "v2 切片经 0011–0022 新增迁移引入；既有 0001–0010 未改"
    );
    let parallel: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'local_%'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(parallel, 0, "不得建立平行权威表");
    let before = serde_json::to_value(&project).unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert!(sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,thumbnail_path,preview_path,created_at) VALUES('bad','other-project',0,'none','none',0,'hash',1,1,'none','none','now')").execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    db.pool.close().await;
    let reopened = Store::open(&path).await.unwrap();
    assert_eq!(
        serde_json::to_value(reopened.get(&project.id).await.unwrap()).unwrap(),
        before
    );
}
