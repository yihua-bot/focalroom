//! M4 桌面端云登录：临时本地 HTTP 服务器接收 Web 端回传的认证 token。
//! 流程：启动随机端口监听 → 打开浏览器 → 用户在 Web 登录（支持验证码/Google）
//! → Web 重定向到 localhost 回调 → 捕获 token → 关闭服务器。
use crate::domain::{AppError, Result};
use std::io::{BufRead, BufRead, Write};
use std::net::TcpListener;
use tokio::task::spawn_blocking;

/// 启动临时回调服务器，返回登录 URL 和 token 接收器。
/// 调用方打开浏览器到 login_url，用户在 Web 登录后 Web 重定向到 callback_url。
pub async fn start_cloud_login(base_url: &str) -> Result<(String, String)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| AppError::new("AUTH_SERVER", format!("本地端口绑定失败：{e}")))?;

    let port = listener
        .local_addr()
        .map_err(|e| AppError::new("AUTH_SERVER", format!("获取端口失败：{e}")))?
        .port();

    let login_url = format!(
        "{base_url}/shared?desktop_login={port}"
    );
    let callback_url = format!("http://localhost:{port}/callback");

    // 在阻塞线程中等待回调。
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String>>();
    spawn_blocking(move || {
        let (stream, _) = match listener.accept() {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(Err(AppError::new("AUTH_SERVER", format!("接受连接失败：{e}"))));
                return;
            }
        };
        let mut writer = stream.try_clone().unwrap_or_else(|_| {
            std::io::sink() as std::io::Stdin
        });
        let reader = std::io::BufReader::new(stream);

        // 解析请求行获取 query params。
        let request_line = reader.lines().next();
        let path = request_line
            .map(|l| l.unwrap_or_default())
            .unwrap_or_default();

        // 提取 token 参数。
        let token = path
            .split('?')
            .nth(1)
            .and_then(|qs| {
                qs.split('&').find_map(|pair| {
                    let mut kv = pair.splitn(2, '=');
                    let k = kv.next()?;
                    if k == "token" {
                        kv.next().map(String::from)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default();

        // 发送响应到浏览器（让用户看到"登录成功"页面）。
        let html = if token.is_empty() {
            "<html><body><h2>登录回调未包含 token</h2><p>请关闭此页面返回应用重试。</p></body></html>".to_string()
        } else {
            "<html><body><h2>✅ 登录成功</h2><p>请关闭此页面返回 FocalRoom。</p></body></html>".to_string()
        };
        let _ = write!(
            writer,
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html.len(),
            html
        );
        let _ = writer.flush();

        let _ = tx.send(if token.is_empty() {
            Err(AppError::new("AUTH_SERVER", "回调未包含 token"))
        } else {
            Ok(token)
        });
    });

    Ok((login_url, rx.await.map_err(|_| AppError::new("AUTH_SERVER", "内部通道错误"))?))
}

/// 将 token 存储到钥匙串（与共享功能使用同一存储路径）。
pub async fn store_cloud_token(
    store: &crate::db::Store,
    base_url: &str,
    token: &str,
) -> Result<()> {
    store
        .pool
        .execute(
            "INSERT OR REPLACE INTO app_settings(key,value) VALUES('cloud_auth_token',?)",
            (format!("{base_url}|{token}"),),
        )
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(())
}
