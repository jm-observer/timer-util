use crate::compute::Composition;
use crate::data::{Hour, Minute, MonthDay, Second, WeekDay};
use crate::traits::{ConfigOperator, FromData};
use anyhow::{bail, Result};
use chrono::{Datelike, Duration, Local, NaiveDateTime, Timelike};
use log::debug;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Bound, RangeBounds, Sub};

/// Timer configuration that defines a schedule based on days, hours, minutes, and seconds.
///
/// Use [`configure_weekday()`](crate::configure_weekday) or
/// [`configure_monthday()`](crate::configure_monthday) to create a configuration
/// through the builder API.
///
/// # Example
///
/// ```
/// use timer_util::*;
///
/// // Schedule for every Monday at 9:30:00
/// let conf = configure_weekday(WeekDays::default_value(W1))
///     .build_with_hours(Hours::default_value(H9))
///     .build_with_minute(Minutes::default_value(M30))
///     .build_with_second(Seconds::default_value(S0));
///
/// // Get the next scheduled time after a given point
/// let next = conf.next_with_time(
///     chrono::NaiveDateTime::new(
///         chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
///         chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
///     )
/// );
/// ```
#[derive(Debug, Clone)]
pub struct TimerConf {
    pub(crate) days: Days,
    pub(crate) hours: Hours,
    pub(crate) minutes: Minutes,
    pub(crate) seconds: Seconds,
}

impl TimerConf {
    /// Return all scheduled time points within the given date-time range.
    ///
    /// Both bounded (`start..end`, `start..=end`) and half-open ranges are supported,
    /// but unbounded ranges will return an error.
    pub fn datetimes(
        &self,
        range: impl RangeBounds<NaiveDateTime>,
    ) -> Result<Vec<NaiveDateTime>> {
        let mut start = match range.start_bound() {
            Bound::Unbounded => bail!("unbounded start is not supported"),
            Bound::Included(first) => first.sub(Duration::seconds(1)),
            Bound::Excluded(first) => *first,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => bail!("unbounded end is not supported"),
            Bound::Included(end) => *end,
            Bound::Excluded(end) => end.sub(Duration::seconds(1)),
        };
        if start >= end {
            bail!("start must be before end")
        }
        let mut date_times = Vec::new();
        while start <= end {
            let next = self.next_with_time(start);
            if next <= end {
                date_times.push(next);
                start = next;
            } else {
                break;
            }
        }
        Ok(date_times)
    }

    /// Return the next scheduled time point after `now` (exclusive).
    pub fn next_with_time(&self, now: NaiveDateTime) -> NaiveDateTime {
        let now = now.add(Duration::seconds(1));
        let mut composition = Composition::from(
            now,
            self.days.clone(),
            self.hours.clone(),
            self.minutes.clone(),
            self.seconds.clone(),
        );
        debug!("Composition: {:?}", composition);
        composition.next()
    }

    /// Return the number of seconds until the next scheduled time point from now.
    pub fn next(&self) -> u64 {
        let now_local = Local::now().naive_local();
        let next_local = self.next_with_time(now_local);
        let times = (next_local.timestamp() - now_local.timestamp()) as u64;
        debug!(
            "now : {}-{:02}-{:02} {:02}:{:02}:{:02}",
            now_local.year(),
            now_local.month(),
            now_local.day(),
            now_local.hour(),
            now_local.minute(),
            now_local.second()
        );
        debug!(
            "next: {}-{:02}-{:02} {:02}:{:02}:{:02}",
            next_local.year(),
            next_local.month(),
            next_local.day(),
            next_local.hour(),
            next_local.minute(),
            next_local.second()
        );
        times
    }
}

/// Day selection mode: by month days, week days, or both (union).
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Days {
    MonthDays(MonthDays),
    WeekDays(WeekDays),
    MonthAndWeekDays(MonthDays, WeekDays),
}

impl Days {
    pub(crate) fn month_days(&self, week_day: WeekDay) -> MonthDays {
        match self {
            Days::MonthDays(month_days) => month_days.clone(),
            Days::WeekDays(week_days) => week_days.to_month_days(week_day),
            Days::MonthAndWeekDays(month_days, week_days) => {
                month_days.merge(&week_days.to_month_days(week_day))
            }
        }
    }
    pub(crate) fn update_month_days(self, month_days: MonthDays) -> Self {
        match self {
            Days::MonthDays(_) => Self::MonthDays(month_days),
            Days::WeekDays(week_days) => Self::MonthAndWeekDays(month_days, week_days),
            Days::MonthAndWeekDays(_, week_days) => Self::MonthAndWeekDays(month_days, week_days),
        }
    }
    pub(crate) fn update_week_days(self, week_days: WeekDays) -> Self {
        match self {
            Days::MonthDays(month_days) => Self::MonthAndWeekDays(month_days, week_days),
            Days::WeekDays(_) => Self::WeekDays(week_days),
            Days::MonthAndWeekDays(month_days, _) => Self::MonthAndWeekDays(month_days, week_days),
        }
    }
}

/// Configuration for which days of the month are selected (1-31).
///
/// Uses a bitset internally where bit N being set means day N is selected.
#[derive(Clone)]
pub struct MonthDays(u64);

/// Configuration for which days of the week are selected (1=Mon, 7=Sun).
///
/// Uses a bitset internally where bit N being set means weekday N is selected.
#[derive(Clone)]
pub struct WeekDays(u64);

/// Configuration for which hours of the day are selected (0-23).
///
/// Uses a bitset internally where bit N being set means hour N is selected.
#[derive(Clone)]
pub struct Hours(u64);

/// Configuration for which minutes of the hour are selected (0-59).
///
/// Uses a bitset internally where bit N being set means minute N is selected.
#[derive(Clone, Eq, PartialEq)]
pub struct Minutes(u64);

/// Configuration for which seconds of the minute are selected (0-59).
///
/// Uses a bitset internally where bit N being set means second N is selected.
#[derive(Clone)]
pub struct Seconds(u64);

impl ConfigOperator for Hours {
    const MIN: u64 = 0;
    const MAX: u64 = 23;
    const DEFAULT_MAX: u64 = (u32::MAX >> 8) as u64;
    type DataTy = Hour;

    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> u64 {
        self.0
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index).map(Self::DataTy::from_data)
    }
    fn _val_mut(&mut self, val: u64) {
        self.0 = val
    }
}
impl ConfigOperator for Seconds {
    const MIN: u64 = 0;
    const MAX: u64 = 59;
    const DEFAULT_MAX: u64 = u64::MAX >> 4;
    type DataTy = Second;
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index).map(Self::DataTy::from_data)
    }
    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> u64 {
        self.0
    }
    fn _val_mut(&mut self, val: u64) {
        self.0 = val
    }
}
impl ConfigOperator for Minutes {
    const MIN: u64 = 0;
    const MAX: u64 = 59;
    const DEFAULT_MAX: u64 = u64::MAX >> 4;
    type DataTy = Minute;
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index).map(Self::DataTy::from_data)
    }
    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> u64 {
        self.0
    }
    fn _val_mut(&mut self, val: u64) {
        self.0 = val
    }
}

impl ConfigOperator for MonthDays {
    const MIN: u64 = 1;
    const MAX: u64 = 31;
    const DEFAULT_MAX: u64 = (u32::MAX << 1) as u64;
    type DataTy = MonthDay;
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index).map(Self::DataTy::from_data)
    }
    fn _default() -> Self {
        Self(0)
    }
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn _val(&self) -> u64 {
        self.0
    }
    fn _val_mut(&mut self, val: u64) {
        self.0 = val
    }
}

impl Minutes {
    /// Create a minute configuration that triggers at regular intervals.
    ///
    /// For example, `Minutes::every(15)` selects minutes 0, 15, 30, 45.
    /// Returns an empty configuration if `interval` is 0.
    pub fn every(interval: u64) -> Self {
        if interval == 0 {
            Self::_default()
        } else {
            let mut val = 0u64;
            let mut minutes = Self::_default();
            while val <= Self::MAX {
                minutes = minutes.add(Minute::from_data(val));
                val += interval
            }
            minutes
        }
    }
}

impl ConfigOperator for WeekDays {
    const DEFAULT_MAX: u64 = (u8::MAX << 1) as u64;
    const MIN: u64 = 1;
    const MAX: u64 = 7;

    type DataTy = WeekDay;

    fn _default() -> Self {
        Self(0)
    }

    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index).map(Self::DataTy::from_data)
    }

    fn _val(&self) -> u64 {
        self.0
    }

    fn _val_mut(&mut self, val: u64) {
        self.0 = val;
    }
}

#[allow(dead_code)]
impl WeekDays {
    pub(crate) fn to_month_days(&self, start: WeekDay) -> MonthDays {
        let week_unit = self.0 >> 1;
        let days = (week_unit
            | week_unit << 7
            | week_unit << 14
            | week_unit << 21
            | week_unit << 28
            | week_unit << 35)
            >> (start as u64 - 1)
            << 1;

        let mut month_days = MonthDays::_default();
        month_days._val_mut(days);
        month_days
    }
}

impl Debug for Seconds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == u64::MAX >> 4 {
            write!(f, "all seconds.")
        } else {
            write!(f, "seconds: {:?}.", self.to_vec())
        }
    }
}
impl Debug for Minutes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == u64::MAX >> 4 {
            write!(f, "all minutes.")
        } else {
            write!(f, "minutes: {:?}.", self.to_vec())
        }
    }
}
impl Debug for Hours {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == (u32::MAX >> 8) as u64 {
            write!(f, "all hours.")
        } else {
            write!(f, "hours: {:?}.", self.to_vec())
        }
    }
}
impl Debug for MonthDays {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == (u32::MAX << 1) as u64 {
            write!(f, "all month days.")
        } else {
            write!(f, "month days: {:?}.", self.to_vec())
        }
    }
}
impl Debug for WeekDays {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == (u8::MAX << 1) as u64 {
            write!(f, "all week days.")
        } else {
            write!(f, "week day's array index: {:?}.", self.to_vec())
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ConfigOperator, Hours, Minutes, MonthDays, Seconds, WeekDays};
    use crate::conf::TimerConf;
    #[allow(unused_imports)]
    use crate::data::{DateTime, Hour::*, Minute::*, MonthDay::*, Second::*, WeekDay::*};
    use crate::*;
    use anyhow::Result;
    use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
    use log::debug;
    use std::ops::Sub;

    pub(crate) fn datetime(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        second: u32,
    ) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd(year, month, day),
            NaiveTime::from_hms(hour, min, second),
        )
    }

    #[test]
    fn test_auto() -> anyhow::Result<()> {
        let conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
            .conf_month_days(
                MonthDays::default_range(D5..D10)?
                    .add_range(D15..D20)?
                    .add_range(D25..D30)?,
            )
            .build_with_hours(Hours::default_array(&[H5, H10, H15, H20]))
            .build_with_minute(Minutes::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_value(S0));

        let mut start = datetime(2022, 7, 4, 20, 15, 0);
        let end = datetime(2033, 8, 15, 12, 30, 45);

        let datetimes = conf.datetimes(start..end)?;
        start = start.sub(Duration::seconds(1));
        let mut next = end;
        for datetime in datetimes {
            next = conf.next_with_time(start);
            assert_eq!(datetime, next, "{:?} - {:?}", start, next);
            start = datetime;
        }
        assert_eq!(
            datetime(2033, 8, 15, 10, 45, 0),
            next,
            "{:?} - {:?}",
            end,
            next
        );

        let mut start = datetime(2022, 7, 4, 20, 15, 0);
        let end = datetime(2033, 8, 15, 15, 30, 0);
        let datetimes = conf.datetimes(start..end)?;
        start = start.sub(Duration::seconds(1));
        let mut next = start;
        for datetime in datetimes {
            next = conf.next_with_time(start);
            assert_eq!(datetime, next, "{:?} - {:?}", start, next);
            start = datetime;
        }
        assert_eq!(
            datetime(2033, 8, 15, 15, 15, 0),
            next,
            "{:?} - {:?}",
            end,
            next
        );
        Ok(())
    }
    #[test]
    fn test_auto_pre() -> anyhow::Result<()> {
        let conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
            .conf_month_days(
                MonthDays::default_range(D5..D10)?
                    .add_range(D15..D20)?
                    .add_range(D25..D30)?,
            )
            .build_with_hours(Hours::default_array(&[H5, H10, H15, H20]))
            .build_with_minute(Minutes::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_value(S0));

        let start = datetime(2022, 7, 4, 22, 17, 10);
        let end = datetime(2022, 7, 5, 12, 30, 45);

        let datetimes = conf.datetimes(start..end)?;
        debug!("{:?}", datetimes);
        Ok(())
    }

    #[test]
    fn test_to_month_days() {
        let month_days0 = WeekDays::default_array(&[W1, W3, W5, W7]).to_month_days(W3);
        assert_eq!(
            month_days0.to_vec(),
            vec![1, 3, 5, 6, 8, 10, 12, 13, 15, 17, 19, 20, 22, 24, 26, 27, 29, 31]
        );

        let month_days1 = WeekDays::default_array(&[W1, W3, W5]).to_month_days(W1);
        assert_eq!(
            month_days1.to_vec(),
            vec![1, 3, 5, 8, 10, 12, 15, 17, 19, 22, 24, 26, 29, 31]
        );
        let month_days2 = month_days0.merge(&month_days1);
        assert_eq!(
            month_days2.to_vec(),
            vec![1, 3, 5, 6, 8, 10, 12, 13, 15, 17, 19, 20, 22, 24, 26, 27, 29, 31]
        );
    }
    #[test]
    fn test_datetimes() -> Result<()> {
        let some_datetimes = [
            datetime(2020, 5, 15, 10, 30, 30),
            datetime(2020, 5, 15, 10, 30, 45),
            datetime(2020, 5, 15, 10, 45, 15),
            datetime(2020, 5, 15, 10, 45, 30),
            datetime(2020, 5, 15, 10, 45, 45),
            datetime(2020, 5, 15, 15, 15, 15),
            datetime(2020, 5, 15, 15, 15, 30),
            datetime(2020, 5, 15, 15, 15, 45),
            datetime(2020, 5, 15, 15, 30, 15),
            datetime(2020, 5, 15, 15, 30, 30),
        ];
        let conf = configure_weekday(WeekDays::default_array(&[W5, W3]))
            .conf_month_days(MonthDays::default_array(&[D5, D15, D24]))
            .build_with_hours(Hours::default_array(&[H5, H10, H15]))
            .build_with_minute(Minutes::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_array(&[S15, S30, S45]));
        debug!("2020-5-15 10:30:17");
        let datetimes =
            conf.datetimes(datetime(2020, 5, 15, 10, 30, 17)..=datetime(2020, 5, 15, 15, 30, 30))?;

        assert_eq!(datetimes.as_slice(), &some_datetimes[..]);

        Ok(())
    }

    #[test]
    fn test() -> Result<()> {
        let conf = configure_weekday(WeekDays::default_array(&[W5, W3]))
            .conf_month_days(MonthDays::default_array(&[D5, D15, D24]))
            .build_with_hours(Hours::default_array(&[H5, H10, H15]))
            .build_with_minute(Minutes::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_array(&[S15, S30, S45]));

        compare(
            &conf,
            &[
                datetime(2020, 5, 15, 10, 30, 30),
                datetime(2020, 5, 15, 10, 30, 45),
                datetime(2020, 5, 15, 10, 45, 15),
                datetime(2020, 5, 15, 10, 45, 30),
                datetime(2020, 5, 15, 10, 45, 45),
                datetime(2020, 5, 15, 15, 15, 15),
                datetime(2020, 5, 15, 15, 15, 30),
                datetime(2020, 5, 15, 15, 15, 45),
                datetime(2020, 5, 15, 15, 30, 15),
                datetime(2020, 5, 15, 15, 30, 30),
            ],
        );
        // -------------------------------
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 5, 20).unwrap(),
            month_day: D20,
            week_day: W5,
            hour: H15,
            minute: M45,
            second: S45,
        };
        {
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
            let mut dt0_dist = dt0;
            dt0_dist.week_day = W2;
            dt0_dist.month_day = D24;
            dt0_dist.second = S15;
            dt0_dist.minute = M15;
            dt0_dist.hour = H5;
            dt0_dist.date = NaiveDate::from_ymd_opt(2022, 5, 24).unwrap();
            assert_eq!(dist, dt0_dist);
        }
        // -------------------------------
        let conf = configure_weekday(WeekDays::default_array(&[W5, W3]))
            .conf_month_days(MonthDays::default_array(&[D5, D15, D31]))
            .build_with_hours(Hours::default_array(&[H5, H10, H15]))
            .build_with_minute(Minutes::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_array(&[S15, S30, S45]));
        debug!("{:?}", conf);
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 4, 29).unwrap(),
            month_day: D29,
            week_day: W5,
            hour: H15,
            minute: M45,
            second: S45,
        };
        {
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
            let mut dt0_dist = dt0;
            dt0_dist.week_day = W3;
            dt0_dist.month_day = D4;
            dt0_dist.second = S15;
            dt0_dist.minute = M15;
            dt0_dist.hour = H5;
            dt0_dist.date = NaiveDate::from_ymd_opt(2022, 5, 4).unwrap();
            assert_eq!(dist, dt0_dist);
        }
        Ok(())
    }

    #[test]
    fn test_year() -> Result<()> {
        let conf = configure_monthday(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minute(Minutes::default_array(&[M30]))
            .build_with_second(Seconds::default_array(&[S0]));
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2021, 12, 31).unwrap(),
            month_day: D31,
            week_day: W5,
            hour: H12,
            minute: M30,
            second: S30,
        };
        {
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
            let mut dt0_dist = dist;
            dt0_dist.second = S0;
            dt0_dist.minute = M30;
            dt0_dist.hour = H12;
            dt0_dist.week_day = W1;
            dt0_dist.month_day = D31;
            assert!(dist == dt0_dist, "{:?}", dist);
            assert!(dist.date.year() == 2022, "{:?}", dist.date);
            assert!(dist.date.month() == 1, "{:?}", dist.date);
        }
        Ok(())
    }
    #[test]
    fn test_month() -> Result<()> {
        let conf = configure_monthday(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minute(Minutes::default_array(&[M30]))
            .build_with_second(Seconds::default_array(&[S0]));
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 1, 31).unwrap(),
            month_day: D31,
            week_day: W5,
            hour: H12,
            minute: M30,
            second: S30,
        };
        {
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
            let mut dt0_dist = dist;
            dt0_dist.second = S0;
            dt0_dist.minute = M30;
            dt0_dist.hour = H12;
            dt0_dist.week_day = W4;
            dt0_dist.month_day = D31;
            assert!(dist == dt0_dist, "{:?}", dist);
            assert!(dist.date.year() == 2022, "{:?}", dist.date);
            assert!(dist.date.month() == 3, "{:?}", dist.date);
        }
        Ok(())
    }

    fn compare(conf: &TimerConf, times: &[NaiveDateTime]) {
        let len = times.len() - 1;
        let mut index = 0;
        loop {
            assert_eq!(conf.next_with_time(times[index]), times[index + 1]);
            index += 1;
            if index == len {
                break;
            }
        }
    }
    #[test]
    fn test_every() {
        use Minute::*;
        let minutes = Minutes::every(11);
        assert_eq!(
            minutes,
            Minutes::default_array(&[M0, M11, M22, M33, M44, M55])
        );
        let minutes = Minutes::every(0);
        assert_eq!(minutes, Minutes::_default());
        let minutes = Minutes::every(30);
        assert_eq!(minutes, Minutes::default_array(&[M0, M30]));
        let minutes = Minutes::every(31);
        assert_eq!(minutes, Minutes::default_array(&[M0, M31]));
        let minutes = Minutes::every(60);
        assert_eq!(minutes, Minutes::default_array(&[M0]));

        let minutes = Minutes::every(500);
        assert_eq!(minutes, Minutes::default_array(&[M0]));
    }

    #[test]
    fn test_config_operator_basic() {
        // default_all
        let hours = Hours::default_all();
        assert_eq!(hours.to_vec().len(), 24);

        // intersection
        let h1 = Hours::default_array(&[H0, H6, H12, H18]);
        let h2 = Hours::default_array(&[H6, H12]);
        let inter = h1.intersection(&h2);
        assert_eq!(inter.to_vec(), vec![6, 12]);

        // contain
        assert!(h1.contain(H0));
        assert!(!h1.contain(H1));

        // is_zero
        let empty = Hours::_default();
        assert!(empty.is_zero());
        assert!(!h1.is_zero());
    }

    #[test]
    fn test_datetimes_invalid_range() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let t1 = datetime(2024, 6, 1, 0, 0, 0);
        let t2 = datetime(2024, 1, 1, 0, 0, 0);
        assert!(conf.datetimes(t1..t2).is_err());
    }

    #[test]
    fn test_default_all_by_max() {
        let hours = Hours::default_all_by_max(H12);
        assert_eq!(
            hours.to_vec(),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        );
    }
}
