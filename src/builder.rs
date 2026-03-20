use crate::conf::{Days, Hours, Minutes, MonthDays, Seconds, TimerConf, WeekDays};

/// Builder for configuring which days the timer should trigger on.
///
/// Created by [`configure_weekday()`](crate::configure_weekday) or
/// [`configure_monthday()`](crate::configure_monthday).
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

    /// Add month-day constraints (union with existing day configuration).
    pub fn conf_month_days(self, month_days: MonthDays) -> Self {
        DayConfBuilder {
            days: self.days.update_month_days(month_days),
        }
    }

    /// Add week-day constraints (union with existing day configuration).
    pub fn conf_week_days(self, week_days: WeekDays) -> Self {
        DayConfBuilder {
            days: self.days.update_week_days(week_days),
        }
    }

    /// Set the hour configuration and advance to the next builder stage.
    pub fn build_with_hours(self, hours: Hours) -> DayHourConfBuilder {
        DayHourConfBuilder {
            days: self.days,
            hours,
        }
    }
}

/// Builder with day and hour configuration set.
pub struct DayHourConfBuilder {
    days: Days,
    hours: Hours,
}
impl DayHourConfBuilder {
    /// Set the minute configuration and advance to the next builder stage.
    pub fn build_with_minute(self, minutes: Minutes) -> DayHourMinuteConfBuilder {
        DayHourMinuteConfBuilder {
            days: self.days,
            hours: self.hours,
            minutes,
        }
    }
}

/// Builder with day, hour, and minute configuration set.
pub struct DayHourMinuteConfBuilder {
    days: Days,
    hours: Hours,
    minutes: Minutes,
}
impl DayHourMinuteConfBuilder {
    /// Set the second configuration and produce the final [`TimerConf`].
    pub fn build_with_second(self, seconds: Seconds) -> TimerConf {
        TimerConf {
            days: self.days,
            hours: self.hours,
            minutes: self.minutes,
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

#[cfg(test)]
mod test {
    use crate::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn test_builder_basic_weekday() {
        let conf = configure_weekday(WeekDays::default_value(W1))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let next = conf.next_with_time(start);
        // 2024-01-01 is Monday, next Monday is 2024-01-08
        assert_eq!(
            next,
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 1, 8).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        );
    }

    #[test]
    fn test_builder_combined_days() {
        let conf = configure_weekday(WeekDays::default_value(W1))
            .conf_month_days(MonthDays::default_value(D15))
            .build_with_hours(Hours::default_value(H12))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let next = conf.next_with_time(start);
        // Should find next matching day (union of weekday Monday and monthday 15)
        assert!(next > start);
    }

    #[test]
    fn test_builder_from_monthday() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_all())
            .build_with_minute(Minutes::every(15))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let next = conf.next_with_time(start);
        // With every(15), minutes are 0,15,30,45. Next after 0:00:00 on the 1st should be 0:15:00
        assert_eq!(
            next,
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 15, 0).unwrap(),
            )
        );
    }

    #[test]
    fn test_builder_conf_week_days() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .conf_week_days(WeekDays::default_value(W5))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let next = conf.next_with_time(start);
        assert!(next > start);
    }
}
