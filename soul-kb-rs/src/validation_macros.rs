//! Validation macros for Soul KB Service

/// Macro for validating input against multiple validation rules
#[macro_export]
macro_rules! validate {
    ($input:expr, $($validator:expr),+) => {{
        let mut result = Ok(());
        $(
            if let Err(e) = $validator.validate(&$input) {
                result = Err(e);
                break;
            }
        )+
        result
    }};
}

/// Macro for validating numeric values against range constraints
#[macro_export]
macro_rules! validate_range {
    ($value:expr, $min:expr, $max:expr) => {{
        if $value < $min || $value > $max {
            Err(ValidationError::OutOfRange(format!(
                "Value {} must be between {} and {}",
                $value, $min, $max
            )))
        } else {
            Ok(())
        }
    }};
}

/// Macro for validating string length
#[macro_export]
macro_rules! validate_length {
    ($value:expr, $max:expr) => {{
        if $value.len() > $max {
            Err(ValidationError::TooLong(format!(
                "Length {} exceeds maximum {}",
                $value.len(),
                $max
            )))
        } else {
            Ok(())
        }
    }};
}