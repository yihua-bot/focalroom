// 展览的规格不写在页面上：观看距离与悬挂中心线说的是"这面墙怎么看"，不是"这一页多大"。
// 拿书本的 300 PPI 去判一张两米五外看的放印，是把排版当成光学——一幅 1200mm 宽的画
// 在 2.5 米外根本看不出网点，但按书本阈值它永远"分辨率不足"，于是整条展览线报的都是假故障。
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::{AppError, Result};

/// 人眼极限约 1 角分（1/60 度 = 0.000291 弧度）：像素周期张角小于这个数就糊成一片。
/// 所需 PPI = 25.4 / (张角 × 观看距离毫米)，即约 87300 / 距离毫米。
const ACUITY_RAD: f64 = 0.000290_8882;
/// 再远的距离也不把要求降到 25 PPI 以下：比这更稀的网点，往前两步就是网格本身。
const PPI_FLOOR: f64 = 25.0;

pub const DISTANCE_MIN_MM: f64 = 300.0;
pub const DISTANCE_MAX_MM: f64 = 20_000.0;
pub const HANG_MIN_MM: f64 = 300.0;
pub const HANG_MAX_MM: f64 = 3_000.0;

/// 命令带进来的两个数各管一件事：观看距离决定分辨率怎么判，悬挂中心线决定展签与画框定位。
/// 整条一起替换（`null` 就是明确不记录），不留"改了一个、另一个还是上一次的"半份规格。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ExhibitionSpec {
    pub viewing_distance_mm: Option<f64>,
    pub hang_center_mm: Option<f64>,
}

impl ExhibitionSpec {
    /// 按观看距离推出的最低 PPI。
    /// 它是"可以比印刷基线松到哪里"的上限放宽，不是更严的要求，所以结果永不超过 `baseline`，
    /// 也永不低过 25：用户把基线调到 72 时生效的仍是 72。
    pub fn required_ppi(&self, baseline: f64) -> Option<f64> {
        let distance = self.viewing_distance_mm?;
        if !distance.is_finite() || distance <= 0.0 {
            return None;
        }
        let derived = 25.4 / (ACUITY_RAD * distance);
        Some(derived.clamp(PPI_FLOOR, baseline.max(PPI_FLOOR)))
    }
}

/// 从 `spec_json` 读回展览规格。脏值只当没填：一本展览的规格里混进一个字符串，
/// 不该把整本印前检查一起拖垮，也不该让读的人以为那两个数真的记录过。
pub fn read(spec_json: &str) -> ExhibitionSpec {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(spec_json) else {
        return ExhibitionSpec::default();
    };
    ExhibitionSpec {
        viewing_distance_mm: pick(
            &map,
            "viewing_distance_mm",
            DISTANCE_MIN_MM,
            DISTANCE_MAX_MM,
        ),
        hang_center_mm: pick(&map, "hang_center_mm", HANG_MIN_MM, HANG_MAX_MM),
    }
}

fn pick(map: &Map<String, Value>, key: &str, min: f64, max: f64) -> Option<f64> {
    match map.get(key) {
        Some(Value::Number(number)) => number
            .as_f64()
            .filter(|value| value.is_finite() && (*value >= min && *value <= max)),
        _ => None,
    }
}

/// 校验并合并出新的一份 `spec_json`：开本那两个毫米数原样留着，只换这两个展览字段。
pub fn store(current: &str, spec: &ExhibitionSpec) -> Result<String> {
    check(
        spec.viewing_distance_mm,
        DISTANCE_MIN_MM,
        DISTANCE_MAX_MM,
        "观看距离",
    )?;
    check(spec.hang_center_mm, HANG_MIN_MM, HANG_MAX_MM, "悬挂中心线")?;
    // 读不出对象时从空开始：这本的规格本来就坏了，把垃圾留在表里救不回来，
    // 而版面校验（必须是 JSON 对象）也仍然满足。
    let mut map = match serde_json::from_str::<Value>(current) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    };
    place(&mut map, "viewing_distance_mm", spec.viewing_distance_mm);
    place(&mut map, "hang_center_mm", spec.hang_center_mm);
    Ok(Value::Object(map).to_string())
}

fn check(value: Option<f64>, min: f64, max: f64, name: &str) -> Result<()> {
    if let Some(value) = value
        && (!value.is_finite() || value < min || value > max)
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            format!("{name}须为 {:.0}–{:.0} 毫米，或留空表示不记录", min, max),
        ));
    }
    Ok(())
}

fn place(map: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    match value {
        Some(value) => {
            map.insert(key.to_owned(), Value::from(value));
        }
        None => {
            map.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(distance: Option<f64>, hang: Option<f64>) -> ExhibitionSpec {
        ExhibitionSpec {
            viewing_distance_mm: distance,
            hang_center_mm: hang,
        }
    }

    #[test]
    fn two_metres_five_apart_needs_nowhere_near_three_hundred_ppi() {
        // 2.5 米：约 35 PPI；一本 1200mm 宽的放印在这个距离上肉眼看不出网点。
        let required = spec(Some(2_500.0), None).required_ppi(300.0).unwrap();
        assert!((34.0..=36.0).contains(&required), "{required}");
        // 手持画册的距离（约 254mm）算出来仍高于印刷基线，于是基线说话。
        assert_eq!(spec(Some(254.0), None).required_ppi(300.0), Some(300.0));
    }

    #[test]
    fn the_relaxation_has_a_floor_and_never_beats_a_stricter_baseline() {
        // 100 米外的高速路广告牌：不给出"1 PPI 也合格"这种荒唐话。
        assert_eq!(spec(Some(100_000.0), None).required_ppi(300.0), Some(25.0));
        // 用户把基线收到 72：更严的那个生效，距离推算不能反过来放宽用户自己设的线。
        assert_eq!(spec(Some(254.0), None).required_ppi(72.0), Some(72.0));
        assert_eq!(spec(Some(2_500.0), None).required_ppi(20.0), Some(25.0));
    }

    #[test]
    fn no_distance_recorded_means_no_judgement_rubber_stamped() {
        assert_eq!(spec(None, Some(1_450.0)).required_ppi(300.0), None);
        assert_eq!(spec(Some(0.0), None).required_ppi(300.0), None);
        assert_eq!(spec(Some(f64::NAN), None).required_ppi(300.0), None);
    }

    #[test]
    fn page_size_stays_in_the_spec_and_only_the_two_exhibition_fields_move() {
        let current = "{\"width_mm\":190,\"height_mm\":257}";
        let merged = store(current, &spec(Some(2_500.0), Some(1_450.0))).unwrap();
        let again = store(&merged, &spec(Some(3_000.0), None)).unwrap();
        let value: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(value["width_mm"], 190);
        assert_eq!(value["height_mm"], 257);
        assert_eq!(value["viewing_distance_mm"], 3000.0);
        // 留空是"明确不记录"，那个键就此消失，而不是留下一个会被当成 0 的数。
        assert!(value.get("hang_center_mm").is_none());
        // 语义相同就不算改动：键序与空格是 JSON 序列化的细节，不该被当成"用户又改了一次规格"。
        assert_eq!(
            read(&store(current, &spec(None, None)).unwrap()),
            read(current)
        );
    }

    #[test]
    fn a_spec_outside_the_physical_range_is_refused_before_it_reaches_the_database() {
        for bad in [
            spec(Some(120.0), None),
            spec(Some(90_000.0), None),
            spec(None, Some(4_500.0)),
            spec(Some(f64::INFINITY), None),
        ] {
            assert!(store("{\"width_mm\":190}", &bad).is_err(), "{bad:?}");
        }
        assert!(store("{\"width_mm\":190}", &spec(Some(2_500.0), Some(1_450.0))).is_ok());
    }

    #[test]
    fn garbage_in_the_spec_reads_as_not_recorded_rather_than_crashing_the_check() {
        for junk in [
            "",
            "not json",
            "[]",
            "{\"viewing_distance_mm\":\"很远\"}",
            "{\"viewing_distance_mm\":-5}",
            "{\"viewing_distance_mm\":1e99}",
        ] {
            assert_eq!(read(junk), ExhibitionSpec::default(), "{junk}");
            assert_eq!(read(junk).required_ppi(300.0), None, "{junk}");
        }
        let parsed =
            read("{\"viewing_distance_mm\":2500,\"hang_center_mm\":1450,\"width_mm\":190}");
        assert_eq!(parsed, spec(Some(2_500.0), Some(1_450.0)));
    }

    #[test]
    fn a_broken_spec_can_still_be_repaired_through_the_command() {
        let merged = store("this is not json", &spec(Some(2_500.0), None)).unwrap();
        assert_eq!(merged, "{\"viewing_distance_mm\":2500.0}");
        assert_eq!(read(&merged).viewing_distance_mm, Some(2_500.0));
    }
}
