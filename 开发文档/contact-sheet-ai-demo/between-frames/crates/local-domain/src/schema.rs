use serde_json::Value;
use std::collections::HashSet;

/// 应用契约使用的严格 JSON Schema 子集；未知校验关键字拒绝，不静默放宽。
pub fn valid(schema: &Value, value: &Value) -> bool {
    const SUPPORTED: &[&str] = &[
        "$schema",
        "$id",
        "title",
        "description",
        "type",
        "const",
        "enum",
        "oneOf",
        "anyOf",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "uniqueItems",
        "minItems",
        "maxItems",
        "minLength",
        "maxLength",
        "minimum",
        "maximum",
    ];
    let Some(schema_object) = schema.as_object() else {
        return false;
    };
    if schema_object
        .keys()
        .any(|key| !SUPPORTED.contains(&key.as_str()))
    {
        return false;
    }
    if schema
        .get("const")
        .is_some_and(|constant| constant != value)
        || schema.get("enum").is_some_and(|choices| {
            !choices
                .as_array()
                .is_some_and(|choices| choices.contains(value))
        })
    {
        return false;
    }
    for (keyword, exactly_one) in [("oneOf", true), ("anyOf", false)] {
        if let Some(branches) = schema.get(keyword) {
            let Some(branches) = branches.as_array() else {
                return false;
            };
            let matches = branches
                .iter()
                .filter(|branch| valid(branch, value))
                .count();
            if (exactly_one && matches != 1) || (!exactly_one && matches == 0) {
                return false;
            }
        }
    }
    let is_type = |kind: &str| match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some(),
        "number" => value.is_number(),
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        _ => false,
    };
    if let Some(kind) = schema.get("type") {
        let matches = if let Some(kind) = kind.as_str() {
            is_type(kind)
        } else {
            kind.as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str().is_some_and(is_type)))
        };
        if !matches {
            return false;
        }
    }
    if let Some(object) = value.as_object() {
        if schema.get("required").is_some_and(|required| {
            !required.as_array().is_some_and(|keys| {
                keys.iter()
                    .all(|key| key.as_str().is_some_and(|key| object.contains_key(key)))
            })
        }) {
            return false;
        }
        for (key, item) in object {
            if let Some(property) = schema["properties"].get(key) {
                if !valid(property, item) {
                    return false;
                }
            } else if schema["additionalProperties"] == false {
                return false;
            }
        }
    }
    if let Some(items) = value.as_array() {
        let size = items.len() as u64;
        if schema["minItems"].as_u64().is_some_and(|min| size < min)
            || schema["maxItems"].as_u64().is_some_and(|max| size > max)
            || (schema["uniqueItems"] == true
                && items
                    .iter()
                    .map(Value::to_string)
                    .collect::<HashSet<_>>()
                    .len()
                    != items.len())
        {
            return false;
        }
        if let Some(item_schema) = schema.get("items")
            && items.iter().any(|item| !valid(item_schema, item))
        {
            return false;
        }
    }
    if let Some(text) = value.as_str() {
        let size = text.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|min| size < min)
            || schema["maxLength"].as_u64().is_some_and(|max| size > max)
        {
            return false;
        }
    }
    if let Some(number) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|min| number < min)
            || schema["maximum"].as_f64().is_some_and(|max| number > max))
    {
        return false;
    }
    true
}
