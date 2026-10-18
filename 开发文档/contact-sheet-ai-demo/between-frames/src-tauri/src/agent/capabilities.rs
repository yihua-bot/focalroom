//! 可信部署能力声明：精确绑定端点/模型，不把协议、声明或冒烟当作质量评测。
use crate::domain::{AppError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Read, path::Path};

pub const PROFILE_PATH_ENV: &str = "BETWEEN_FRAMES_AI_CAPABILITY_PROFILE";
pub const OBSERVATION_PROFILE_PATH_ENV: &str = "BETWEEN_FRAMES_AI_OBSERVATION_CAPABILITY_PROFILE";
const MAX_PROFILE_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Vision,
    ToolCalling,
    StructuredOutput,
    InstructionFollowing,
}
impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vision => "vision",
            Self::ToolCalling => "tool_calling",
            Self::StructuredOutput => "structured_output",
            Self::InstructionFollowing => "instruction_following",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityProfile {
    version: u32,
    endpoint_sha256: String,
    model_id: String,
    capabilities: BTreeSet<Capability>,
}
impl CapabilityProfile {
    /// 仅部署端/测试注入使用，不接受任务、工具或模型输出作为能力声明。
    pub fn declared(endpoint: &str, model: &str, capabilities: &[Capability]) -> Self {
        Self {
            version: 1,
            endpoint_sha256: endpoint_sha256(endpoint),
            model_id: model.into(),
            capabilities: capabilities.iter().copied().collect(),
        }
    }
    pub fn from_file(path: &Path, endpoint: &str, model: &str) -> Result<Self> {
        // 不读取目录、设备或管道；有界读取且不在错误中回显路径/配置正文。
        if !std::fs::metadata(path).map_err(|_| invalid())?.is_file() {
            return Err(invalid());
        }
        let file = std::fs::File::open(path).map_err(|_| invalid())?;
        if !file.metadata().map_err(|_| invalid())?.is_file() {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        file.take(MAX_PROFILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if bytes.len() as u64 > MAX_PROFILE_BYTES {
            return Err(invalid());
        }
        let profile: Self = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        profile.validate(endpoint, model)?;
        Ok(profile)
    }
    pub(crate) fn validate(&self, endpoint: &str, model: &str) -> Result<()> {
        if self.version != 1
            || self.endpoint_sha256 != endpoint_sha256(endpoint)
            || self.model_id != model
            || model.trim().is_empty()
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn names(&self) -> Vec<&'static str> {
        self.capabilities.iter().map(|cap| cap.as_str()).collect()
    }
    pub fn sha256(&self) -> String {
        // BTreeSet 统一集合顺序；空白、顺序和重复项不改变有效能力身份。
        format!("{:x}", Sha256::digest(serde_json::to_vec(self).unwrap()))
    }
}

pub struct ResolvedProfile {
    pub profile: CapabilityProfile,
    pub source: &'static str,
}
pub(crate) fn resolve(endpoint: &str, model: &str) -> Result<ResolvedProfile> {
    resolve_configured(endpoint, model, std::env::var_os(PROFILE_PATH_ENV))
}
pub(crate) fn resolve_observation(endpoint: &str, model: &str) -> Result<ResolvedProfile> {
    resolve_configured(
        endpoint,
        model,
        std::env::var_os(OBSERVATION_PROFILE_PATH_ENV),
    )
}
fn resolve_configured(
    endpoint: &str,
    model: &str,
    path: Option<std::ffi::OsString>,
) -> Result<ResolvedProfile> {
    if let Some(path) = path {
        return Ok(ResolvedProfile {
            // 显式配置无效时不回退内置配置；实例冻结此结果，运行中不重读文件。
            profile: CapabilityProfile::from_file(Path::new(&path), endpoint, model)?,
            source: "operator_declaration",
        });
    }
    builtin(endpoint, model)
}
pub fn builtin(endpoint: &str, model: &str) -> Result<ResolvedProfile> {
    let (source, capabilities) = match (endpoint.trim().trim_end_matches('/'), model) {
        // 官方文档 2026-10-16：developers.openai.com/api/docs/models/gpt-4o-mini
        ("https://api.openai.com/v1", "gpt-4o-mini" | "gpt-4o-mini-2024-07-18") => (
            "official_documentation",
            vec![
                Capability::Vision,
                Capability::ToolCalling,
                Capability::StructuredOutput,
                Capability::InstructionFollowing,
            ],
        ),
        // 本仓库真实工具/视觉协议冒烟；证据见 V2-M3-EVALUATION-TEST-REPORT.md。
        ("https://api.deepseek.com", "deepseek-flash") => (
            "local_protocol_smoke",
            vec![
                Capability::Vision,
                Capability::ToolCalling,
                Capability::StructuredOutput,
                Capability::InstructionFollowing,
            ],
        ),
        // 官方文档 2026-10-18：docs.anthropic.com（Messages API）。
        // 原生适配仅文本编排；如实不声明 Vision，视觉任务由路由拒绝并指引。
        ("https://api.anthropic.com/v1", _) if model.starts_with("claude-") => (
            "official_documentation",
            vec![
                Capability::ToolCalling,
                Capability::StructuredOutput,
                Capability::InstructionFollowing,
            ],
        ),
        // 官方文档 2026-10-18：ai.google.dev（generateContent）。
        ("https://generativelanguage.googleapis.com/v1beta", _) if model.starts_with("gemini-") => {
            (
                "official_documentation",
                vec![
                    Capability::ToolCalling,
                    Capability::StructuredOutput,
                    Capability::InstructionFollowing,
                ],
            )
        }
        _ => {
            return Err(AppError::new(
                "MODEL_CAPABILITY_MISSING",
                "当前端点与模型缺少可信能力配置；请由部署端配置后重启，任务不会自动切换服务方",
            ));
        }
    };
    Ok(ResolvedProfile {
        profile: CapabilityProfile::declared(endpoint, model, &capabilities),
        source,
    })
}
pub(crate) fn endpoint_sha256(endpoint: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(endpoint.trim().trim_end_matches('/').as_bytes())
    )
}
fn invalid() -> AppError {
    AppError::new(
        "MODEL_CAPABILITY_MISSING",
        "模型能力配置无效或与当前端点/模型不符；请由部署端检查后重启，未使用默认配置代替",
    )
}
