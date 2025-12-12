//! Validation macros for Soul KB Service

/// Macro for validating input against multiple validation rules
#[macro_export]
macro_rules! validate {
    ($input:expr, $($validator:expr),+ $(,)?) => {{
        // Validators are expected to be functions/closures returning `ValidationResult<()>`.
        // They should accept the same input type as `$input`.
        $(
            $validator($input)?;
        )+
        Ok(())
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
