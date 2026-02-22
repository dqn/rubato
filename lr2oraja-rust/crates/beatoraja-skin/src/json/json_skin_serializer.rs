// Mechanical translation of JsonSkinSerializer.java
// JSON skin serializer with conditional branching and includes

use std::collections::HashSet;
use std::path::Path;

use serde_json::Value;

/// Corresponds to JsonSkinSerializer
///
/// In Java, this class sets up custom serializers for LibGDX Json that handle:
/// - Conditional branches (if/value)
/// - File includes
/// - Lua script property loading
///
/// In Rust, serde handles the deserialization directly via #[serde(default)].
/// The conditional branching and include logic needs to be handled as a
/// pre-processing step on the JSON before feeding it to serde.
pub struct JsonSkinSerializer {
    pub options: HashSet<i32>,
}

impl Default for JsonSkinSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonSkinSerializer {
    pub fn new() -> Self {
        Self {
            options: HashSet::new(),
        }
    }

    pub fn with_options(options: HashSet<i32>) -> Self {
        Self { options }
    }

    /// Test option conditions.
    /// Corresponds to Serializer.testOption in Java.
    ///
    /// - 901 -> 901 enabled
    /// - [901, 911] -> 901 enabled && 911 enabled
    /// - [[901, 902], 911] -> (901 || 902) && 911
    /// - -901 -> 901 disabled
    pub fn test_option(&self, ops: &Value) -> bool {
        if ops.is_null() {
            return true;
        }
        if let Some(n) = ops.as_i64() {
            return self.test_number(n as i32);
        }
        if let Some(arr) = ops.as_array() {
            let mut enabled = true;
            for item in arr {
                if let Some(n) = item.as_i64() {
                    enabled = self.test_number(n as i32);
                } else if let Some(sub_arr) = item.as_array() {
                    let mut enabled_sub = false;
                    for sub_item in sub_arr {
                        if let Some(n) = sub_item.as_i64()
                            && self.test_number(n as i32)
                        {
                            enabled_sub = true;
                            break;
                        }
                    }
                    enabled = enabled_sub;
                } else {
                    enabled = false;
                }
                if !enabled {
                    break;
                }
            }
            return enabled;
        }
        false
    }

    /// Set up serializers on a JSON loader.
    /// In Java: registers custom LibGDX Json serializers for conditional/include handling.
    /// In Rust: serde handles deserialization directly; conditional/include preprocessing
    /// is done via preprocess_object/preprocess_array methods.
    pub fn set_serializers(&self, _loader: &mut super::json_skin_loader::JSONSkinLoader) {
        // No-op in Rust: serde-based deserialization doesn't use LibGDX-style serializer registration.
        // The preprocess_object and preprocess_array methods handle the same logic.
    }

    /// Test a single option number.
    /// Corresponds to Java Serializer.testNumber(int op).
    pub fn test_number(&self, op: i32) -> bool {
        if op >= 0 {
            self.options.contains(&op)
        } else {
            !self.options.contains(&(-op))
        }
    }

    /// Pre-process a JSON value, resolving conditional branches and includes.
    /// This corresponds to ObjectSerializer.read in Java.
    pub fn preprocess_object(&self, value: &Value, base_path: &Path) -> Option<Value> {
        if let Some(arr) = value.as_array() {
            // Conditional branch: take first clause satisfying its conditions
            for item in arr {
                if let Some(obj) = item.as_object()
                    && let Some(if_val) = obj.get("if")
                    && self.test_option(if_val)
                {
                    return obj.get("value").cloned();
                }
            }
            return None;
        }

        if let Some(obj) = value.as_object()
            && let Some(include) = obj.get("include")
            && let Some(include_path) = include.as_str()
        {
            let file_path = base_path
                .parent()
                .map(|p| p.join(include_path))
                .unwrap_or_else(|| std::path::PathBuf::from(include_path));
            if file_path.exists()
                && let Ok(content) = std::fs::read_to_string(&file_path)
                && let Ok(parsed) = serde_json::from_str::<Value>(&content)
            {
                return Some(parsed);
            }
            return None;
        }

        Some(value.clone())
    }

    /// Pre-process a JSON array value, resolving conditional items and includes.
    /// This corresponds to ArraySerializer.read in Java.
    pub fn preprocess_array(&self, value: &Value, base_path: &Path) -> Vec<Value> {
        let mut items = Vec::new();

        if let Some(arr) = value.as_array() {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if obj.contains_key("if")
                        && (obj.contains_key("value") || obj.contains_key("values"))
                    {
                        // Conditional item(s)
                        if let Some(if_val) = obj.get("if")
                            && self.test_option(if_val)
                        {
                            if let Some(val) = obj.get("value") {
                                items.push(val.clone());
                            }
                            if let Some(vals) = obj.get("values")
                                && let Some(vals_arr) = vals.as_array()
                            {
                                for v in vals_arr {
                                    items.push(v.clone());
                                }
                            }
                        }
                    } else if obj.contains_key("include") {
                        // Array include (inside)
                        let included = self.include_array(obj, base_path);
                        items.extend(included);
                    } else {
                        // Single item
                        items.push(item.clone());
                    }
                } else {
                    items.push(item.clone());
                }
            }
        } else if let Some(obj) = value.as_object() {
            if obj.contains_key("include") {
                // Array include (outside)
                let included = self.include_array(obj, base_path);
                items.extend(included);
            } else {
                // Regard as single item
                items.push(value.clone());
            }
        }

        items
    }

    fn include_array(&self, obj: &serde_json::Map<String, Value>, base_path: &Path) -> Vec<Value> {
        let mut items = Vec::new();
        if let Some(include) = obj.get("include")
            && let Some(include_path) = include.as_str()
        {
            let file_path = base_path
                .parent()
                .map(|p| p.join(include_path))
                .unwrap_or_else(|| std::path::PathBuf::from(include_path));
            if file_path.exists()
                && let Ok(content) = std::fs::read_to_string(&file_path)
                && let Ok(parsed) = serde_json::from_str::<Value>(&content)
                && let Some(arr) = parsed.as_array()
            {
                items.extend(arr.iter().cloned());
            }
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_number() {
        let mut serializer = JsonSkinSerializer::new();
        serializer.options.insert(901);

        assert!(serializer.test_option(&serde_json::json!(901)));
        assert!(!serializer.test_option(&serde_json::json!(902)));
        assert!(!serializer.test_option(&serde_json::json!(-901)));
        assert!(serializer.test_option(&serde_json::json!(-902)));
    }

    #[test]
    fn test_option_array() {
        let mut serializer = JsonSkinSerializer::new();
        serializer.options.insert(901);
        serializer.options.insert(911);

        // [901, 911] -> 901 && 911
        assert!(serializer.test_option(&serde_json::json!([901, 911])));
        // [901, 912] -> 901 && !912
        assert!(!serializer.test_option(&serde_json::json!([901, 912])));
    }

    #[test]
    fn test_option_nested_array() {
        let mut serializer = JsonSkinSerializer::new();
        serializer.options.insert(902);
        serializer.options.insert(911);

        // [[901, 902], 911] -> (901 || 902) && 911
        assert!(serializer.test_option(&serde_json::json!([[901, 902], 911])));
    }

    #[test]
    fn test_option_null() {
        let serializer = JsonSkinSerializer::new();
        assert!(serializer.test_option(&serde_json::json!(null)));
    }
}
