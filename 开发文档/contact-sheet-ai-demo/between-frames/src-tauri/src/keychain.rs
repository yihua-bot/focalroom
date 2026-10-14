// macOS Keychain 集成：API key 存入系统钥匙串（规格 §12.1 P0）。
// 安全边界：key 不写 SQLite/日志/前端；进程参数短暂可见于本机 ps（单人桌面可接受，ADR 0006/0008 说明）；
// 显式设置的环境变量优先于钥匙串（开发意图优先）。
use crate::domain::{AppError, Result};

const SERVICE: &str = "between-frames.openai";
const ACCOUNT: &str = "api-key";

fn security(args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new("security")
        .args(args)
        .output()
        .map_err(|e| AppError::new("KEYCHAIN_ERROR", format!("无法调用系统钥匙串：{e}")))
}

#[cfg(target_os = "macos")]
pub fn store(service: &str, key: &str) -> Result<()> {
    if key.trim().is_empty() || key.len() > 4096 {
        return Err(AppError::new("VALIDATION_ERROR", "API key 不合法"));
    }
    let status = security(&[
        "add-generic-password",
        "-s",
        service,
        "-a",
        ACCOUNT,
        "-w",
        key.trim(),
        "-U",
    ])?;
    if !status.status.success() {
        return Err(AppError::new(
            "KEYCHAIN_ERROR",
            format!(
                "钥匙串写入失败：{}",
                String::from_utf8_lossy(&status.stderr)
                    .chars()
                    .take(200)
                    .collect::<String>()
            ),
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn load(service: &str) -> Option<String> {
    let status = security(&["find-generic-password", "-s", service, "-w"]).ok()?;
    if !status.status.success() {
        return None;
    }
    let key = String::from_utf8_lossy(&status.stdout).trim().to_owned();
    if key.is_empty() { None } else { Some(key) }
}

#[cfg(target_os = "macos")]
pub fn delete(service: &str) -> Result<()> {
    let status = security(&["delete-generic-password", "-s", service])?;
    if !status.status.success() {
        return Err(AppError::new(
            "KEYCHAIN_ERROR",
            "钥匙串中没有该条目或删除失败",
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn store(_service: &str, _key: &str) -> Result<()> {
    Err(AppError::new("KEYCHAIN_ERROR", "当前平台未集成系统钥匙串"))
}
#[cfg(not(target_os = "macos"))]
pub fn load(_service: &str) -> Option<String> {
    None
}
#[cfg(not(target_os = "macos"))]
pub fn delete(_service: &str) -> Result<()> {
    Err(AppError::new("KEYCHAIN_ERROR", "当前平台未集成系统钥匙串"))
}

pub fn store_api_key(key: &str) -> Result<()> {
    store(SERVICE, key)
}
pub fn load_api_key() -> Option<String> {
    load(SERVICE)
}
pub fn delete_api_key() -> Result<()> {
    delete(SERVICE)
}
/// 解析顺序：显式环境变量（开发）→ 钥匙串（用户存储）。返回 (key, source)。
pub fn resolve_api_key() -> Option<(String, &'static str)> {
    if let Ok(key) = std::env::var(crate::agent::openai::KEY_ENV)
        && !key.trim().is_empty()
    {
        return Some((key.trim().to_owned(), "env"));
    }
    load_api_key().map(|key| (key, "keychain"))
}
pub fn tail(key: &str) -> String {
    key.chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
