//! 文字方向只改变框内排字；保留物理框、正式来源与既有版本通道。
use super::invalid;
use crate::{command_audit::serialization, domain::Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical-rl")]
    VerticalRl,
    #[serde(rename = "vertical-lr")]
    VerticalLr,
    #[serde(rename = "rotate-90")]
    Rotate90,
    #[serde(rename = "rotate-270")]
    Rotate270,
}
pub fn stored(raw: Option<&str>) -> Result<Option<Direction>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let style: serde_json::Value = serde_json::from_str(raw).map_err(serialization)?;
    if !style.is_object() {
        return Err(invalid("文字样式无法读取，请恢复之前的版本"));
    }
    style
        .get("textDirection")
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|_| invalid("文字方向暂不支持，请恢复之前的版本"))
        })
        .transpose()
}
