#![allow(deprecated)]

use crate::builder::DayConfBuilder;
pub use conf::{Hours, Minutes, MonthDays, Seconds, TimerConf, WeekDays};
pub use data::{
    Hour, Hour::*, Minute, Minute::*, MonthDay, MonthDay::*, Second, Second::*, WeekDay,
    WeekDay::*,
};
pub use error::{TimerError, Result};
pub use iter::TimerIter;
pub use traits::*;

mod builder;
mod compute;
mod conf;
mod data;
pub mod error;
mod iter;
mod traits;

/// Create a timer configuration starting with weekday selection.
///
/// # Example
///
/// ```
/// use timer_util::*;
///
/// let conf = configure_weekday(WeekDays::default_value(W1))
///     .build_with_hours(Hours::default_value(H9))
///     .build_with_minute(Minutes::default_value(M0))
///     .build_with_second(Seconds::default_value(S0));
/// ```
pub fn configure_weekday(week_day: WeekDays) -> builder::DayConfBuilder {
    DayConfBuilder::from(week_day)
}

/// Create a timer configuration starting with month-day selection.
///
/// # Example
///
/// ```
/// use timer_util::*;
///
/// let conf = configure_monthday(MonthDays::default_value(D1))
///     .build_with_hours(Hours::default_all())
///     .build_with_minute(Minutes::default_value(M0))
///     .build_with_second(Seconds::default_value(S0));
/// ```
pub fn configure_monthday(month_day: MonthDays) -> builder::DayConfBuilder {
    DayConfBuilder::from(month_day)
}
