/// Validatable trait - equivalent of Java Validatable interface
pub trait Validatable {
    fn validate(&mut self) -> bool;
}

/// Remove invalid elements from a Vec.
/// Elements that are None or fail validation are removed.
pub fn remove_invalid_elements<T: Validatable>(items: Vec<Option<T>>) -> Vec<T> {
    let mut result = Vec::new();
    for item in items {
        if let Some(mut val) = item
            && val.validate()
        {
            result.push(val);
        }
    }
    result
}

/// Remove invalid elements from a Vec of non-optional Validatable items.
pub fn remove_invalid_elements_vec<T: Validatable>(mut items: Vec<T>) -> Vec<T> {
    items.retain_mut(|item| item.validate());
    items
}

/// Remove empty strings from a Vec<String>
pub fn remove_empty_strings(arr: &[String]) -> Vec<String> {
    arr.iter().filter(|s| !s.is_empty()).cloned().collect()
}
