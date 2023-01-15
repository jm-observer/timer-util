use crate::conf::{Days, Hours, Minuters, MonthDays, Seconds, TimerConf, WeekDays};

pub struct DayConfBuilder {
    pub(crate) days: Days,
}
impl DayConfBuilder {
    pub(crate) fn default_month_days(month_days: MonthDays) -> DayConfBuilder {
        DayConfBuilder {
            days: Days::MonthDays(month_days),
        }
    }
    pub(crate) fn default_week_days(week_days: WeekDays) -> DayConfBuilder {
        DayConfBuilder {
            days: Days::WeekDays(week_days),
        }
    }
    pub fn conf_month_days(self, month_days: MonthDays) -> Self {
        DayConfBuilder {
            days: self.days.update_month_days(month_days),
        }
    }
    pub fn conf_week_days(self, week_days: WeekDays) -> Self {
        DayConfBuilder {
            days: self.days.update_week_days(week_days),
        }
    }
    pub fn build_with_hours(self, hours: Hours) -> DayHourConfBuilder {
        DayHourConfBuilder {
            days: self.days,
            hours,
        }
    }
}
pub struct DayHourConfBuilder {
    days: Days,
    hours: Hours,
}
impl DayHourConfBuilder {
    /// config minuter
    pub fn build_with_minuter(self, minuters: Minuters) -> DayHourMinuterConfBuilder {
        DayHourMinuterConfBuilder {
            days: self.days,
            hours: self.hours,
            minuters,
        }
    }
}
pub struct DayHourMinuterConfBuilder {
    days: Days,
    hours: Hours,
    minuters: Minuters,
}
impl DayHourMinuterConfBuilder {
    pub fn build_with_second(self, seconds: Seconds) -> TimerConf {
        // if seconds.is_zero() {
        //     bail!("second must be selected")
        // } else if self.minuters.is_zero() {
        //     bail!("minuter must be selected")
        // } else if self.hours.is_zero() {
        //     bail!("hour must be selected")
        // } else if self.days.is_zero() {
        //     bail!("day must be selected")
        // }
        TimerConf {
            days: self.days,
            hours: self.hours,
            minuters: self.minuters,
            seconds,
        }
    }
}

impl From<MonthDays> for DayConfBuilder {
    fn from(builder: MonthDays) -> Self {
        DayConfBuilder::default_month_days(builder)
    }
}
impl From<WeekDays> for DayConfBuilder {
    fn from(builder: WeekDays) -> Self {
        DayConfBuilder::default_week_days(builder)
    }
}
