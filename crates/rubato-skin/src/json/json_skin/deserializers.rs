// Custom serde deserializers for JSON skin data types.
// Handles Lua coercion artifacts: numbers-as-strings, conditional blocks, etc.

use std::cell::RefCell;
use std::collections::HashSet;

use serde::{Deserialize, Deserializer};

use super::{Animation, Destination, Image, Text};

thread_local! {
    /// Enabled skin option IDs, set before deserializing a JSON skin.
    /// When `Some`, conditional blocks are evaluated against these options.
    /// When `None`, all conditional blocks are included (backward compat for tests/headers).
    static ENABLED_OPTIONS: RefCell<Option<HashSet<i32>>> = const { RefCell::new(None) };
}

/// Set the enabled skin options for conditional evaluation during deserialization.
/// Call this before `parse_skin_json` and clear it after.
pub fn set_enabled_options(options: Option<HashSet<i32>>) {
    ENABLED_OPTIONS.with(|eo| *eo.borrow_mut() = options);
}

/// Check if all option IDs in the given condition array are satisfied.
/// Returns true if no enabled options are set (no filter), or if all conditions are met.
fn conditions_satisfied(conditions: &[serde_json::Value]) -> bool {
    if conditions.is_empty() {
        return true;
    }
    ENABLED_OPTIONS.with(|eo| {
        let enabled = eo.borrow();
        match &*enabled {
            None => true, // No filter set -- include everything
            Some(set) => conditions.iter().all(|c| {
                if let Some(id) = c.as_i64() {
                    set.contains(&(id as i32))
                } else if let Some(arr) = c.as_array() {
                    arr.iter()
                        .any(|sub| sub.as_i64().is_some_and(|id| set.contains(&(id as i32))))
                } else {
                    false
                }
            }),
        }
    })
}

/// Deserialize an i32 that may come as either a JSON number or a string.
/// Lua skin coercion converts "id" numbers to strings; this allows Offset/CustomEvent/CustomTimer
/// id fields (which are i32) to still deserialize correctly from string-coerced values.
pub(super) fn deserialize_i32_lenient<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;
    struct I32LenientVisitor;
    impl<'de> de::Visitor<'de> for I32LenientVisitor {
        type Value = i32;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or string-encoded integer")
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i32, E> {
            Ok(v as i32)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i32, E> {
            Ok(v as i32)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<i32, E> {
            Ok(v as i32)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<i32, E> {
            v.parse::<i32>().map_err(de::Error::custom)
        }
    }
    deserializer.deserialize_any(I32LenientVisitor)
}

/// Deserialize an `Option<String>` that may come as a JSON string or a JSON number.
/// JSON skins use numeric IDs (e.g., `"id": 150`) while the Rust model expects strings.
/// Numeric values are converted to their string representation (e.g., 150 -> "150").
pub(super) fn deserialize_optional_string_from_int<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;
    struct OptionalStringOrIntVisitor;
    impl<'de> de::Visitor<'de> for OptionalStringOrIntVisitor {
        type Value = Option<String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string, integer, or null")
        }
        fn visit_none<E: de::Error>(self) -> Result<Option<String>, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Option<String>, E> {
            Ok(None)
        }
        fn visit_some<D2: Deserializer<'de>>(
            self,
            deserializer: D2,
        ) -> Result<Option<String>, D2::Error> {
            deserializer.deserialize_any(Self)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Option<String>, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_string<E: de::Error>(self, v: String) -> Result<Option<String>, E> {
            Ok(Some(v))
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Option<String>, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Option<String>, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Option<String>, E> {
            // Truncate to integer for clean conversion (150.0 -> "150")
            if v.fract() == 0.0 {
                Ok(Some((v as i64).to_string()))
            } else {
                Ok(Some(v.to_string()))
            }
        }
    }
    deserializer.deserialize_any(OptionalStringOrIntVisitor)
}

/// Deserialize an `Option<i32>` that may come as a JSON number or a Lua expression string.
/// JSON skins can have `"draw": 1` (integer condition) or `"draw": "gauge() >= 75"` (Lua expr).
/// Integer values are preserved; string expressions yield `None` since Lua eval is not yet implemented.
pub(super) fn deserialize_optional_i32_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;
    struct OptionalI32OrStringVisitor;
    impl<'de> de::Visitor<'de> for OptionalI32OrStringVisitor {
        type Value = Option<i32>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer, string, or null")
        }
        fn visit_none<E: de::Error>(self) -> Result<Option<i32>, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Option<i32>, E> {
            Ok(None)
        }
        fn visit_some<D2: Deserializer<'de>>(
            self,
            deserializer: D2,
        ) -> Result<Option<i32>, D2::Error> {
            deserializer.deserialize_any(Self)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Option<i32>, E> {
            Ok(Some(v as i32))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Option<i32>, E> {
            Ok(Some(v as i32))
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Option<i32>, E> {
            Ok(Some(v as i32))
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Option<i32>, E> {
            // Try parsing as integer first; if it fails, it's a Lua expression.
            // Use sentinel -1 to distinguish "has expression" from "no condition" (None).
            // TODO: Store the actual Lua expression string for deferred evaluation.
            Ok(Some(v.parse::<i32>().unwrap_or(-1)))
        }
    }
    deserializer.deserialize_any(OptionalI32OrStringVisitor)
}

pub(super) fn deserialize_flattened_conditional_images<'de, D>(
    deserializer: D,
) -> Result<Vec<Image>, D::Error>
where
    D: Deserializer<'de>,
{
    let items: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    deserialize_vec_with_conditionals(items).map_err(serde::de::Error::custom)
}

/// Deserialize a `Vec<String>` where elements may be JSON numbers.
/// Numeric values are converted to their string representation.
pub(super) fn deserialize_vec_string_from_ints<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        Str(String),
        Int(i64),
        Float(f64),
    }

    let items: Vec<StringOrInt> = Vec::deserialize(deserializer)?;
    Ok(items
        .into_iter()
        .map(|item| match item {
            StringOrInt::Str(s) => s,
            StringOrInt::Int(n) => n.to_string(),
            StringOrInt::Float(f) => {
                if f.fract() == 0.0 {
                    (f as i64).to_string()
                } else {
                    f.to_string()
                }
            }
        })
        .collect())
}

/// Generic helper: deserialize a `Vec<T>` from a JSON array that may contain two kinds
/// of conditional blocks:
///
/// 1. **Object-based**: `{"if":[...], "values":[item, item, ...]}` -- include values only if
///    all option IDs in the "if" array are enabled
/// 2. **Array-based**: `[{"if":[924],"value":{...}}, {"if":[],"value":{...}}]` -- pick the
///    first entry whose "if" conditions are satisfied, or fallback (empty "if")
/// 3. **Direct**: a plain `T` object
fn deserialize_vec_with_conditionals<T: serde::de::DeserializeOwned>(
    items: Vec<serde_json::Value>,
) -> Result<Vec<T>, String> {
    let mut result = Vec::new();
    for item in items {
        if item.is_array() {
            // Array-based conditional: [{"if":[...],"value":{...}}, ...]
            // Pick the first entry whose conditions are satisfied.
            if let Some(arr) = item.as_array() {
                let matched = arr
                    .iter()
                    .find(|entry| {
                        entry
                            .get("if")
                            .and_then(|v| v.as_array())
                            .is_some_and(|conds| conditions_satisfied(conds))
                    })
                    .or_else(|| {
                        // Fallback: entry with empty "if", or first entry
                        arr.iter()
                            .find(|entry| {
                                entry
                                    .get("if")
                                    .and_then(|v| v.as_array())
                                    .is_some_and(|a| a.is_empty())
                            })
                            .or_else(|| arr.first())
                    });
                if let Some(entry) = matched
                    && let Some(value) = entry.get("value")
                {
                    let val: T =
                        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                    result.push(val);
                }
            }
        } else if item.is_object() && item.get("if").is_some() && item.get("values").is_some() {
            // Object-based conditional (plural): {"if":[...], "values":[...]}
            // Include values only if the "if" conditions are satisfied.
            let conds = item
                .get("if")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if conditions_satisfied(&conds)
                && let Some(vals) = item.get("values").and_then(|v| v.as_array())
            {
                for v in vals {
                    let val: T = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
                    result.push(val);
                }
            }
        } else if item.is_object() && item.get("if").is_some() && item.get("value").is_some() {
            // Object-based conditional (singular): {"if":[...], "value":{...}}
            // Include the single value only if the "if" conditions are satisfied.
            let conds = item
                .get("if")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if conditions_satisfied(&conds)
                && let Some(value) = item.get("value")
            {
                let val: T = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                result.push(val);
            }
        } else {
            // Direct object
            let val: T = serde_json::from_value(item).map_err(|e| e.to_string())?;
            result.push(val);
        }
    }
    Ok(result)
}

pub(super) fn deserialize_animations_with_conditionals<'de, D>(
    deserializer: D,
) -> Result<Vec<Animation>, D::Error>
where
    D: Deserializer<'de>,
{
    let items: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    deserialize_vec_with_conditionals(items).map_err(serde::de::Error::custom)
}

pub(super) fn deserialize_flattened_conditional_destinations<'de, D>(
    deserializer: D,
) -> Result<Vec<Destination>, D::Error>
where
    D: Deserializer<'de>,
{
    let items: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    deserialize_vec_with_conditionals(items).map_err(serde::de::Error::custom)
}

pub(super) fn deserialize_flattened_conditional_texts<'de, D>(
    deserializer: D,
) -> Result<Vec<Text>, D::Error>
where
    D: Deserializer<'de>,
{
    let items: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    deserialize_vec_with_conditionals(items).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditions_satisfied_empty_returns_true() {
        assert!(conditions_satisfied(&[]));
    }

    #[test]
    fn conditions_satisfied_no_filter_returns_true() {
        set_enabled_options(None);
        assert!(conditions_satisfied(&[serde_json::json!(901)]));
    }

    #[test]
    fn conditions_satisfied_flat_ids() {
        let mut opts = HashSet::new();
        opts.insert(901);
        opts.insert(911);
        set_enabled_options(Some(opts));

        // All present -> true
        assert!(conditions_satisfied(&[
            serde_json::json!(901),
            serde_json::json!(911),
        ]));
        // Missing 912 -> false
        assert!(!conditions_satisfied(&[
            serde_json::json!(901),
            serde_json::json!(912),
        ]));

        set_enabled_options(None);
    }

    #[test]
    fn conditions_satisfied_nested_or_group() {
        let mut opts = HashSet::new();
        opts.insert(902);
        opts.insert(911);
        set_enabled_options(Some(opts));

        // [[901, 902], 911] -> (901 || 902) && 911 -> true (902 matches)
        assert!(conditions_satisfied(&[
            serde_json::json!([901, 902]),
            serde_json::json!(911),
        ]));

        // [[901, 903], 911] -> (901 || 903) && 911 -> false (neither 901 nor 903)
        assert!(!conditions_satisfied(&[
            serde_json::json!([901, 903]),
            serde_json::json!(911),
        ]));

        set_enabled_options(None);
    }

    #[test]
    fn conditions_satisfied_nested_or_group_alone() {
        let mut opts = HashSet::new();
        opts.insert(902);
        set_enabled_options(Some(opts));

        // [[901, 902]] -> (901 || 902) -> true
        assert!(conditions_satisfied(&[serde_json::json!([901, 902])]));

        // [[903, 904]] -> (903 || 904) -> false
        assert!(!conditions_satisfied(&[serde_json::json!([903, 904])]));

        set_enabled_options(None);
    }

    #[test]
    fn conditions_satisfied_non_integer_element_returns_false() {
        let mut opts = HashSet::new();
        opts.insert(901);
        set_enabled_options(Some(opts));

        // A string element is neither integer nor array -> false
        assert!(!conditions_satisfied(&[serde_json::json!("not_a_number")]));

        set_enabled_options(None);
    }
}
