use crate::builder::DayConfBuilder;
pub use conf::{Hours, Minuters, MonthDays, Seconds, TimerConf, WeekDays};
pub use data::{
    Hour, Hour::*, Minuter, Minuter::*, MonthDay, MonthDay::*, Second, Second::*, WeekDay,
    WeekDay::*,
};
pub use traits::Operator;

mod builder;
mod compute;
mod conf;
mod data;
mod traits;

pub fn configure_weekday(week_day: WeekDays) -> builder::DayConfBuilder {
    DayConfBuilder::from(week_day)
}
pub fn configure_monthday(month_day: MonthDays) -> builder::DayConfBuilder {
    DayConfBuilder::from(month_day)
}
