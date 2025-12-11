//! Numeric validators
//!
//! This module provides validators for numeric inputs.

use crate::errors::{ValidationError, ValidationResult};
use std::str::FromStr;

/// Validate that a numeric value is within a range (inclusive)
pub fn between<T>(value: T, min: T, max: T) -> ValidationResult<()>
where
    T: PartialOrd + std::fmt::Debug,
{
    if value < min || value > max {
        Err(ValidationError::OutOfRange(format!(
            "Value {:?} is outside range [{:?}, {:?}]",
            value, min, max
        )))
    } else {
        Ok(())
    }
}

/// Validate that a numeric value is greater than a minimum
pub fn min<T>(value: T, min: T) -> ValidationResult<()>
where
    T: PartialOrd + std::fmt::Debug,
{
    if value < min {
        Err(ValidationError::OutOfRange(format!(
            "Value {:?} is less than minimum {:?}",
            value, min
        )))
    } else {
        Ok(())
    }
}

/// Validate that a numeric value is less than a maximum
pub fn max<T>(value: T, max: T) -> ValidationResult<()>
where
    T: PartialOrd + std::fmt::Debug,
{
    if value > max {
        Err(ValidationError::OutOfRange(format!(
            "Value {:?} exceeds maximum {:?}",
            value, max
        )))
    } else {
        Ok(())
    }
}

/// Validate that a numeric value is positive
pub fn positive<T>(value: T) -> ValidationResult<()>
where
    T: PartialOrd + Default + std::fmt::Debug,
{
    if value <= T::default() {
        Err(ValidationError::OutOfRange(format!(
            "Value {:?} is not positive",
            value
        )))
    } else {
        Ok(())
    }
}

/// Validate that a numeric value is non-negative
pub fn non_negative<T>(value: T) -> ValidationResult<()>
where
    T: PartialOrd + Default + std::fmt::Debug,
{
    if value < T::default() {
        Err(ValidationError::OutOfRange(format!(
            "Value {:?} is negative",
            value
        )))
    } else {
        Ok(())
    }
}

/// Validate that a numeric string parses to an integer
pub fn is_integer(s: &str) -> ValidationResult<()> {
    match i64::from_str(s) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::InvalidFormat(format!(
            "Value '{}' is not a valid integer",
            s
        ))),
    }
}

/// Validate that a numeric string parses to a float
pub fn is_float(s: &str) -> ValidationResult<()> {
    match f64::from_str(s) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::InvalidFormat(format!(
            "Value '{}' is not a valid floating point number",
            s
        ))),
    }
}

/// Validate that a numeric string parses to an unsigned integer
pub fn is_unsigned_integer(s: &str) -> ValidationResult<()> {
    match u64::from_str(s) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::InvalidFormat(format!(
            "Value '{}' is not a valid unsigned integer",
            s
        ))),
    }
}

/// Validate that a value is one of the allowed values
pub fn one_of<T>(value: T, allowed: &[T]) -> ValidationResult<()>
where
    T: PartialEq + std::fmt::Debug,
{
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat(format!(
            "Value {:?} is not one of the allowed values: {:?}",
            value, allowed
        )))
    }
}

/// Validate that a value is a multiple of a given divisor
pub fn multiple_of<T>(value: T, divisor: T) -> ValidationResult<()>
where
    T: std::ops::Rem<Output = T> + Default + PartialEq + Copy + std::fmt::Debug,
{
    if value % divisor == T::default() {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat(format!(
            "Value {:?} is not a multiple of {:?}",
            value, divisor
        )))
    }
}

// String-to-numeric with validation
/// Parse and validate a string as an integer within a range
pub fn parse_integer_in_range(s: &str, min: i64, max: i64) -> ValidationResult<i64> {
    match i64::from_str(s) {
        Ok(value) => {
            between(value, min, max)?;
            Ok(value)
        }
        Err(_) => Err(ValidationError::InvalidFormat(format!(
            "Value '{}' is not a valid integer",
            s
        ))),
    }
}

/// Parse and validate a string as a float within a range
pub fn parse_float_in_range(s: &str, min: f64, max: f64) -> ValidationResult<f64> {
    match f64::from_str(s) {
        Ok(value) => {
            between(value, min, max)?;
            Ok(value)
        }
        Err(_) => Err(ValidationError::InvalidFormat(format!(
            "Value '{}' is not a valid floating point number",
            s
        ))),
    }
}

/// Validates a port number (1-65535)
pub fn is_port(value: u16) -> ValidationResult<()> {
    if value == 0 {
        Err(ValidationError::OutOfRange(
            "Port number cannot be 0".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Parse and validate a string as a port number
pub fn parse_port(s: &str) -> ValidationResult<u16> {
    match u16::from_str(s) {
        Ok(value) => {
            is_port(value)?;
            Ok(value)
        }
        Err(_) => Err(ValidationError::InvalidFormat(format!(
            "Value '{}' is not a valid port number",
            s
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_between() {
        assert!(between(5, 0, 10).is_ok());
        assert!(between(0, 0, 10).is_ok()); // Min boundary
        assert!(between(10, 0, 10).is_ok()); // Max boundary
        assert!(between(-1, 0, 10).is_err());
        assert!(between(11, 0, 10).is_err());
    }

    #[test]
    fn test_min_max() {
        assert!(min(5, 0).is_ok());
        assert!(min(0, 0).is_ok()); // Boundary
        assert!(min(-1, 0).is_err());

        assert!(max(5, 10).is_ok());
        assert!(max(10, 10).is_ok()); // Boundary
        assert!(max(11, 10).is_err());
    }

    #[test]
    fn test_positive_non_negative() {
        assert!(positive(5).is_ok());
        assert!(positive(0).is_err());
        assert!(positive(-1).is_err());

        assert!(non_negative(5).is_ok());
        assert!(non_negative(0).is_ok()); // Boundary
        assert!(non_negative(-1).is_err());
    }

    #[test]
    fn test_is_integer() {
        assert!(is_integer("123").is_ok());
        assert!(is_integer("-123").is_ok());
        assert!(is_integer("123.45").is_err());
        assert!(is_integer("abc").is_err());
    }

    #[test]
    fn test_is_float() {
        assert!(is_float("123.45").is_ok());
        assert!(is_float("-123.45").is_ok());
        assert!(is_float("123").is_ok()); // Integers are valid floats
        assert!(is_float("abc").is_err());
    }

    #[test]
    fn test_is_unsigned_integer() {
        assert!(is_unsigned_integer("123").is_ok());
        assert!(is_unsigned_integer("-123").is_err());
        assert!(is_unsigned_integer("123.45").is_err());
        assert!(is_unsigned_integer("abc").is_err());
    }

    #[test]
    fn test_one_of() {
        assert!(one_of(5, &[1, 3, 5, 7, 9]).is_ok());
        assert!(one_of(2, &[1, 3, 5, 7, 9]).is_err());
    }

    #[test]
    fn test_multiple_of() {
        assert!(multiple_of(10, 2).is_ok());
        assert!(multiple_of(10, 3).is_err());
    }

    #[test]
    fn test_parse_integer_in_range() {
        assert_eq!(parse_integer_in_range("5", 0, 10).unwrap(), 5);
        assert!(parse_integer_in_range("-1", 0, 10).is_err());
        assert!(parse_integer_in_range("11", 0, 10).is_err());
        assert!(parse_integer_in_range("abc", 0, 10).is_err());
    }

    #[test]
    fn test_parse_float_in_range() {
        assert_eq!(parse_float_in_range("5.5", 0.0, 10.0).unwrap(), 5.5);
        assert!(parse_float_in_range("-1.0", 0.0, 10.0).is_err());
        assert!(parse_float_in_range("11.0", 0.0, 10.0).is_err());
        assert!(parse_float_in_range("abc", 0.0, 10.0).is_err());
    }

    #[test]
    fn test_is_port() {
        assert!(is_port(1).is_ok());
        assert!(is_port(8080).is_ok());
        assert!(is_port(65535).is_ok()); // Max valid port
        assert!(is_port(0).is_err()); // Reserved
    }

    #[test]
    fn test_parse_port() {
        assert_eq!(parse_port("8080").unwrap(), 8080);
        assert!(parse_port("0").is_err());
        assert!(parse_port("70000").is_err()); // Out of u16 range
        assert!(parse_port("abc").is_err());
    }
}
