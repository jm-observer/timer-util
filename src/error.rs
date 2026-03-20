use std::fmt;

/// Errors that can occur in timer-util.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerError {
    /// The range start is greater than the range end.
    InvalidRange { start: u64, end: u64 },
    /// Unbounded ranges are not supported in `datetimes()`.
    UnboundedRange,
    /// A value is out of the valid range for its type.
    ValueOutOfRange {
        type_name: &'static str,
        value: u64,
        min: u64,
        max: u64,
    },
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange { start, end } => {
                write!(f, "invalid range: start ({}) > end ({})", start, end)
            }
            Self::UnboundedRange => {
                write!(f, "unbounded ranges are not supported")
            }
            Self::ValueOutOfRange {
                type_name,
                value,
                min,
                max,
            } => {
                write!(
                    f,
                    "{} value {} is out of range [{}, {}]",
                    type_name, value, min, max
                )
            }
        }
    }
}

impl std::error::Error for TimerError {}

/// A specialized `Result` type for timer-util.
pub type Result<T> = std::result::Result<T, TimerError>;
