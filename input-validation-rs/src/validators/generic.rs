//! Generic validators
//!
//! This module provides generic validators that can be used for various types of input.

use crate::errors::{ValidationError, ValidationResult};
use std::collections::HashSet;

/// Validate that a value is not in a denied list
pub fn not_in<T: PartialEq + std::fmt::Debug>(value: &T, denied: &[T]) -> ValidationResult<()> {
    if denied.contains(value) {
        Err(ValidationError::InvalidFormat(format!(
            "Value {:?} is in the denied list",
            value
        )))
    } else {
        Ok(())
    }
}

/// Validate that a value is in an allowed list
pub fn one_of<T: PartialEq + std::fmt::Debug>(value: &T, allowed: &[T]) -> ValidationResult<()> {
    if allowed.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat(format!(
            "Value {:?} is not in the allowed list",
            value
        )))
    }
}

/// Validate that a collection doesn't contain duplicate values
pub fn no_duplicates<T: Eq + std::hash::Hash + std::fmt::Debug>(
    values: &[T],
) -> ValidationResult<()> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for value in values {
        if !seen.insert(value) {
            duplicates.push(value);
        }
    }

    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat(format!(
            "Collection contains duplicate values: {:?}",
            duplicates
        )))
    }
}

/// Validate that a value passes all validators
pub fn all<T, F>(value: &T, validators: &[F]) -> ValidationResult<()>
where
    F: Fn(&T) -> ValidationResult<()>,
{
    let mut errors = Vec::new();

    for validator in validators {
        if let Err(err) = validator(value) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::composite(errors))
    }
}

/// Validate that a value passes at least one validator
pub fn any<T, F>(value: &T, validators: &[F]) -> ValidationResult<()>
where
    F: Fn(&T) -> ValidationResult<()>,
{
    let mut errors = Vec::new();

    for validator in validators {
        match validator(value) {
            Ok(()) => {
                return Ok(());
            }
            Err(err) => {
                errors.push(err);
            }
        }
    }

    Err(ValidationError::composite(errors))
}

/// Validate with a custom validation function
pub fn custom<T, F>(value: &T, validator: F, error_message: &str) -> ValidationResult<()>
where
    F: FnOnce(&T) -> bool,
{
    if validator(value) {
        Ok(())
    } else {
        Err(ValidationError::Generic(error_message.to_string()))
    }
}

/// Validate with different validators based on a condition
pub fn when<T, F, G, H>(value: &T, condition: F, if_true: G, if_false: H) -> ValidationResult<()>
where
    F: FnOnce(&T) -> bool,
    G: FnOnce(&T) -> ValidationResult<()>,
    H: FnOnce(&T) -> ValidationResult<()>,
{
    if condition(value) {
        if_true(value)
    } else {
        if_false(value)
    }
}

/// Validate that a value is not None
pub fn required<T>(option: &Option<T>) -> ValidationResult<()> {
    if option.is_none() {
        Err(ValidationError::MissingFields(
            "Required value is missing".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Validate an option value if it's Some
pub fn optional<T, F>(option: &Option<T>, validator: F) -> ValidationResult<()>
where
    F: FnOnce(&T) -> ValidationResult<()>,
{
    match option {
        Some(value) => validator(value),
        None => Ok(()),
    }
}

/// Validate that a collection is non-empty
pub fn not_empty<T>(collection: &[T]) -> ValidationResult<()> {
    if collection.is_empty() {
        Err(ValidationError::TooShort(
            "Collection must not be empty".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Validate that a collection doesn't exceed a maximum length
pub fn max_length<T>(collection: &[T], max: usize) -> ValidationResult<()> {
    if collection.len() > max {
        Err(ValidationError::TooLong(format!(
            "Collection length ({}) exceeds maximum length ({})",
            collection.len(),
            max
        )))
    } else {
        Ok(())
    }
}

/// Validate that a collection meets a minimum length requirement
pub fn min_length<T>(collection: &[T], min: usize) -> ValidationResult<()> {
    if collection.len() < min {
        Err(ValidationError::TooShort(format!(
            "Collection length ({}) is less than minimum length ({})",
            collection.len(),
            min
        )))
    } else {
        Ok(())
    }
}

/// Validate each item in a collection
pub fn each<T, F>(collection: &[T], validator: F) -> ValidationResult<()>
where
    F: Fn(&T) -> ValidationResult<()>,
{
    let mut errors = Vec::new();

    for (idx, item) in collection.iter().enumerate() {
        if let Err(err) = validator(item) {
            errors.push(ValidationError::composite_at(
                vec![err],
                format!("At index {}", idx),
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::composite(errors))
    }
}

/// Validate the number of items in a collection that match a predicate
pub fn count_matching<T, F>(
    collection: &[T],
    predicate: F,
    min: Option<usize>,
    max: Option<usize>,
) -> ValidationResult<()>
where
    F: Fn(&T) -> bool,
{
    let matching_count = collection.iter().filter(|item| predicate(item)).count();

    if let Some(min_val) = min {
        if matching_count < min_val {
            return Err(ValidationError::TooShort(format!(
                "Count of matching items ({}) is less than minimum ({})",
                matching_count, min_val
            )));
        }
    }

    if let Some(max_val) = max {
        if matching_count > max_val {
            return Err(ValidationError::TooLong(format!(
                "Count of matching items ({}) exceeds maximum ({})",
                matching_count, max_val
            )));
        }
    }

    Ok(())
}

/// A reference to a validator function for reuse
pub type ValidatorFn<T> = Box<dyn Fn(&T) -> ValidationResult<()>>;

/// Helper for creating composite validators
pub fn create_validator<T, F>(validator: F) -> ValidatorFn<T>
where
    F: Fn(&T) -> ValidationResult<()> + 'static,
{
    Box::new(validator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_of() {
        assert!(one_of(&"apple", &["apple", "banana", "cherry"]).is_ok());
        assert!(one_of(&"orange", &["apple", "banana", "cherry"]).is_err());
    }

    #[test]
    fn test_not_in() {
        assert!(not_in(&"orange", &["apple", "banana", "cherry"]).is_ok());
        assert!(not_in(&"apple", &["apple", "banana", "cherry"]).is_err());
    }

    #[test]
    fn test_no_duplicates() {
        assert!(no_duplicates(&[1, 2, 3, 4]).is_ok());
        assert!(no_duplicates(&[1, 2, 3, 1]).is_err());
    }

    #[test]
    fn test_all() {
        let validators: Vec<ValidatorFn<i32>> = vec![
            create_validator(|&n| {
                if n > 0 {
                    Ok(())
                } else {
                    Err(ValidationError::new("not positive"))
                }
            }),
            create_validator(|&n| {
                if n < 100 {
                    Ok(())
                } else {
                    Err(ValidationError::new("too large"))
                }
            }),
        ];

        assert!(all(&50, &validators).is_ok());
        assert!(all(&0, &validators).is_err());
        assert!(all(&200, &validators).is_err());
    }

    #[test]
    fn test_any() {
        let validators: Vec<ValidatorFn<&str>> = vec![
            create_validator(|s| {
                if s.len() > 10 {
                    Ok(())
                } else {
                    Err(ValidationError::new("too short"))
                }
            }),
            create_validator(|s| {
                if s.contains('x') {
                    Ok(())
                } else {
                    Err(ValidationError::new("no x found"))
                }
            }),
        ];

        assert!(any(&"long string here", &validators).is_ok());
        assert!(any(&"has x here", &validators).is_ok());
        assert!(any(&"short", &validators).is_err());
    }

    #[test]
    fn test_required() {
        let some_val: Option<i32> = Some(5);
        let none_val: Option<i32> = None;

        assert!(required(&some_val).is_ok());
        assert!(required(&none_val).is_err());
    }

    #[test]
    fn test_optional() {
        let some_val: Option<i32> = Some(5);
        let none_val: Option<i32> = None;
        let invalid_val: Option<i32> = Some(-5);

        let validator = |&n: &i32| {
            if n > 0 {
                Ok(())
            } else {
                Err(ValidationError::new("not positive"))
            }
        };

        assert!(optional(&some_val, validator).is_ok());
        assert!(optional(&none_val, validator).is_ok()); // None is always valid
        assert!(optional(&invalid_val, validator).is_err());
    }

    #[test]
    fn test_collection_validators() {
        let empty: Vec<i32> = vec![];
        let items = vec![1, 2, 3, 4, 5];

        assert!(not_empty(&items).is_ok());
        assert!(not_empty(&empty).is_err());

        assert!(max_length(&items, 10).is_ok());
        assert!(max_length(&items, 3).is_err());

        assert!(min_length(&items, 3).is_ok());
        assert!(min_length(&items, 10).is_err());
    }

    #[test]
    fn test_each() {
        let valid_items = vec![2, 4, 6, 8];
        let invalid_items = vec![2, 3, 6, 8];

        let validator = |&n: &i32| {
            if n % 2 == 0 {
                Ok(())
            } else {
                Err(ValidationError::new("not even"))
            }
        };

        assert!(each(&valid_items, validator).is_ok());
        assert!(each(&invalid_items, validator).is_err());
    }

    #[test]
    fn test_custom() {
        let is_even = |&n: &i32| n % 2 == 0;

        assert!(custom(&4, is_even, "not even").is_ok());
        assert!(custom(&3, is_even, "not even").is_err());
    }
}
