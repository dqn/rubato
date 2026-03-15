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

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysValid(i32);
    impl Validatable for AlwaysValid {
        fn validate(&mut self) -> bool {
            true
        }
    }

    struct ValidIfPositive(i32);
    impl Validatable for ValidIfPositive {
        fn validate(&mut self) -> bool {
            self.0 > 0
        }
    }

    #[test]
    fn remove_invalid_elements_keeps_valid_some_items() {
        let items: Vec<Option<AlwaysValid>> =
            vec![Some(AlwaysValid(1)), None, Some(AlwaysValid(2))];
        let result = remove_invalid_elements(items);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn remove_invalid_elements_removes_none() {
        let items: Vec<Option<AlwaysValid>> = vec![None, None, None];
        let result = remove_invalid_elements(items);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_invalid_elements_removes_failing_validation() {
        let items: Vec<Option<ValidIfPositive>> = vec![
            Some(ValidIfPositive(1)),
            Some(ValidIfPositive(-1)),
            Some(ValidIfPositive(2)),
            Some(ValidIfPositive(0)),
        ];
        let result = remove_invalid_elements(items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 2);
    }

    #[test]
    fn remove_invalid_elements_empty_input() {
        let items: Vec<Option<AlwaysValid>> = vec![];
        let result = remove_invalid_elements(items);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_invalid_elements_vec_keeps_valid() {
        let items = vec![ValidIfPositive(1), ValidIfPositive(-1), ValidIfPositive(3)];
        let result = remove_invalid_elements_vec(items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 3);
    }

    #[test]
    fn remove_invalid_elements_vec_empty_input() {
        let items: Vec<ValidIfPositive> = vec![];
        let result = remove_invalid_elements_vec(items);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_empty_strings_removes_empties() {
        let arr = vec![
            "hello".to_string(),
            "".to_string(),
            "world".to_string(),
            "".to_string(),
        ];
        let result = remove_empty_strings(&arr);
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn remove_empty_strings_all_empty() {
        let arr = vec!["".to_string(), "".to_string()];
        let result = remove_empty_strings(&arr);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_empty_strings_none_empty() {
        let arr = vec!["a".to_string(), "b".to_string()];
        let result = remove_empty_strings(&arr);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn remove_empty_strings_empty_input() {
        let arr: Vec<String> = vec![];
        let result = remove_empty_strings(&arr);
        assert!(result.is_empty());
    }
}
