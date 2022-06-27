mod compute;
pub mod data;

pub use data::{
    Hour::{self, *},
    Minuter::{self, *},
    MonthDay::{self, *},
    Second::{self, *},
    WeekDay::{self, *},
};

use anyhow::{bail, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use data::{AsData, DateTime};
use log::{debug, trace};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, AddAssign, BitAnd, BitOr, BitOrAssign, Bound, RangeBounds, Shl, Sub};
// use time::format_description::FormatItem;
// use time::macros::format_description;
// use time::{Duration, OffsetDateTime, PrimitiveDateTime};
//
// const TS_DASHES_BLANK_COLONS_DOT_BLANK: &[FormatItem<'static>] =
//     format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

#[derive(Clone)]
pub struct MonthDays(u32);
#[derive(Clone)]
pub struct WeekDays(u8);
#[derive(Clone)]
pub struct Hours(u32);
#[derive(Clone)]
pub struct Minuters(u64);
#[derive(Clone)]
pub struct Seconds(u64);
impl Operator for Hours {
    const MIN: Self::ValTy = 0;
    const MAX: Self::ValTy = 23;
    const ONE: Self::ValTy = 1;
    const ZERO: Self::ValTy = 0;
    const DEFAULT_MAX: Self::ValTy = u32::MAX >> 8;
    type ValTy = u32;
    type DataTy = Hour;

    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _mut_val(&mut self, val: Self::ValTy) {
        self.0 = val
    }
}
impl Operator for Seconds {
    const MIN: Self::ValTy = 0;
    const MAX: Self::ValTy = 59;
    const ONE: Self::ValTy = 1;
    const ZERO: Self::ValTy = 0;
    const DEFAULT_MAX: Self::ValTy = u64::MAX >> 4;
    type ValTy = u64;
    type DataTy = Second;

    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _mut_val(&mut self, val: Self::ValTy) {
        self.0 = val
    }
}
impl Operator for Minuters {
    const MIN: Self::ValTy = 0;
    const MAX: Self::ValTy = 59;
    const ONE: Self::ValTy = 1;
    const ZERO: Self::ValTy = 0;
    const DEFAULT_MAX: Self::ValTy = u64::MAX >> 4;
    type ValTy = u64;
    type DataTy = Minuter;

    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _mut_val(&mut self, val: Self::ValTy) {
        self.0 = val
    }
}

impl Operator for MonthDays {
    const MIN: Self::ValTy = 1;
    const MAX: Self::ValTy = 31;
    const ONE: Self::ValTy = 1;
    const ZERO: Self::ValTy = 0;
    const DEFAULT_MAX: Self::ValTy = u32::MAX << 1;
    type ValTy = u32;
    type DataTy = MonthDay;

    fn _default() -> Self {
        Self(0)
    }

    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _mut_val(&mut self, val: Self::ValTy) {
        self.0 = val
    }
}
impl Operator for WeekDays {
    const MIN: Self::ValTy = 1;
    const MAX: Self::ValTy = 7;
    const ONE: Self::ValTy = 1;
    const ZERO: Self::ValTy = 0;
    const DEFAULT_MAX: Self::ValTy = u8::MAX << 1;
    type ValTy = u8;
    type DataTy = WeekDay;

    fn _default() -> Self {
        Self(0)
    }

    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _mut_val(&mut self, val: Self::ValTy) {
        self.0 = val
    }
}

pub trait Operator: Sized {
    const MIN: Self::ValTy;
    const MAX: Self::ValTy;
    const ONE: Self::ValTy;
    const ZERO: Self::ValTy;
    const DEFAULT_MAX: Self::ValTy;
    type ValTy: BitOr<Output = Self::ValTy>
        + Shl<Output = Self::ValTy>
        + Copy
        + BitOrAssign
        + Add<Output = Self::ValTy>
        + Sub<Output = Self::ValTy>
        + PartialOrd
        + AddAssign
        + BitAnd<Output = Self::ValTy>
        + Display;

    type DataTy: AsData<Self::ValTy>;

    fn _default() -> Self;
    #[inline]
    fn default_value(val: Self::DataTy) -> Self {
        let ins = Self::_default();
        ins.add(val)
    }
    #[inline]
    fn default_range(range: impl RangeBounds<Self::DataTy>) -> Result<Self> {
        let ins = Self::_default();
        ins.add_range(range)
    }
    #[inline]
    fn default_all() -> Self {
        let mut ins = Self::_default();
        ins._mut_val(Self::DEFAULT_MAX);
        ins
    }
    fn default_array(vals: &[Self::DataTy]) -> Self {
        let ins = Self::_default();
        ins.add_array(vals)
    }
    fn add_array(mut self, vals: &[Self::DataTy]) -> Self {
        let mut val = self._val();
        for i in vals {
            val |= Self::ONE << i.as_data();
        }
        self._mut_val(val);
        self
    }
    fn add(mut self, index: Self::DataTy) -> Self {
        let index = index.as_data();
        self._mut_val(self._val() | (Self::ONE << index));
        self
    }
    fn add_range(mut self, range: impl RangeBounds<Self::DataTy>) -> Result<Self> {
        let mut first = match range.start_bound() {
            Bound::Unbounded => Self::MIN,
            Bound::Included(first) => first.as_data(),
            Bound::Excluded(first) => first.as_data() + Self::ONE,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => Self::MAX,
            Bound::Included(end) => end.as_data(),
            Bound::Excluded(end) => end.as_data() - Self::ONE,
        };
        if first > end {
            bail!("error:{} > {}", first, end);
        }
        let mut val = self._val();
        while first <= end {
            val |= Self::ONE << first;
            first += Self::ONE;
        }
        self._mut_val(val);
        Ok(self)
    }

    fn to_vec(&self) -> Vec<Self::ValTy> {
        let mut res = Vec::new();
        let val = self._val();
        let mut first = Self::MIN;
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                res.push(first);
            }
            first += Self::ONE;
        }
        res
    }
    fn contain(&self, index: Self::DataTy) -> bool {
        let index = index.as_data();
        let val = self._val();
        val & (Self::ONE << index) > Self::ZERO
    }
    /// 取下一个持有值
    fn next(&self, index: Self::DataTy) -> Option<Self::ValTy> {
        let mut first = index.as_data() + Self::ONE;
        let val = self._val();
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                return Some(first);
            }
            first += Self::ONE;
        }
        None
    }
    /// 取最小的持有值
    fn min_val(&self) -> Self::ValTy {
        let mut first = Self::MIN;
        let val = self._val();
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                return first;
            }
            first += Self::ONE;
        }
        unreachable!("it is a bug");
    }
    fn _val(&self) -> Self::ValTy;
    fn _mut_val(&mut self, val: Self::ValTy);
}

pub struct DayConfBuilder {
    month_days: Option<MonthDays>,
    week_days: Option<WeekDays>,
}
impl DayConfBuilder {
    pub fn conf_month_days(self, month_days: MonthDays) -> Self {
        DayConfBuilder {
            month_days: Some(month_days),
            week_days: self.week_days,
        }
    }
    pub fn conf_week_days(self, week_days: WeekDays) -> Self {
        DayConfBuilder {
            month_days: self.month_days,
            week_days: Some(week_days),
        }
    }
    pub fn build_with_hours(self, hours: Hours) -> DayHourConfBuilder {
        DayHourConfBuilder {
            month_days: self.month_days,
            week_days: self.week_days,
            hours,
        }
    }
}
pub struct DayHourConfBuilder {
    month_days: Option<MonthDays>,
    week_days: Option<WeekDays>,
    hours: Hours,
}
impl DayHourConfBuilder {
    /// config minuter
    pub fn build_with_minuter(self, minuters: Minuters) -> DayHourMinuterConfBuilder {
        DayHourMinuterConfBuilder {
            month_days: self.month_days,
            week_days: self.week_days,
            hours: self.hours,
            minuters,
        }
    }
}
pub struct DayHourMinuterConfBuilder {
    month_days: Option<MonthDays>,
    week_days: Option<WeekDays>,
    hours: Hours,
    minuters: Minuters,
}
impl DayHourMinuterConfBuilder {
    pub fn build_with_second(self, seconds: Seconds) -> DayHourMinuterSecondConf {
        DayHourMinuterSecondConf {
            month_days: self.month_days,
            week_days: self.week_days,
            hours: self.hours,
            minuters: self.minuters,
            seconds,
        }
    }
}
#[derive(Debug, Clone)]
pub struct DayHourMinuterSecondConf {
    pub(crate) month_days: Option<MonthDays>,
    pub(crate) week_days: Option<WeekDays>,
    pub(crate) hours: Hours,
    pub(crate) minuters: Minuters,
    pub(crate) seconds: Seconds,
}

impl DayHourMinuterSecondConf {
    pub fn default_month_days(month_days: MonthDays) -> DayConfBuilder {
        DayConfBuilder {
            month_days: Some(month_days),
            week_days: None,
        }
    }
    pub fn default_week_days(week_days: WeekDays) -> DayConfBuilder {
        DayConfBuilder {
            month_days: None,
            week_days: Some(week_days),
        }
    }
    pub fn next(&self) -> Result<u64> {
        let now_local = Local::now().naive_local();
        let datetime = now_local.clone().into();
        // let offset = now_local.clone().offset();
        let next_local = self._next(datetime)?;
        let times = (next_local.timestamp() - now_local.timestamp()) as u64;
        let next_time = NextTime::init(times);
        debug!(
            "now: {}, next: {}, next time is after {:?} = {}s",
            now_local, next_local, next_time, times
        );
        Ok(times)
    }
    fn _next(&self, datetime: DateTime) -> Result<NaiveDateTime> {
        let day_self = self
            .month_days
            .as_ref()
            .map_or(false, |x| x.contain(datetime.month_day))
            || self
                .week_days
                .as_ref()
                .map_or(false, |x| x.contain(datetime.week_day));

        let hour_self = self.hours.contain(datetime.hour);
        let minuter_self = self.minuters.contain(datetime.minuter);

        let (mut day_possible, mut hour_possible, mut minuter_possible, mut second_possible) =
            if day_self {
                if hour_self {
                    if minuter_self {
                        (
                            Possible::Oneself,
                            Possible::Oneself,
                            Possible::Oneself,
                            Possible::Next,
                        )
                    } else {
                        (
                            Possible::Oneself,
                            Possible::Oneself,
                            Possible::Next,
                            Possible::Min,
                        )
                    }
                } else {
                    (
                        Possible::Oneself,
                        Possible::Next,
                        Possible::Min,
                        Possible::Min,
                    )
                }
            } else {
                (Possible::Next, Possible::Min, Possible::Min, Possible::Min)
            };
        let (second, second_recount) = get_val(second_possible, &self.seconds, datetime.second);
        if second_recount {
            second_possible = Possible::Min;
            minuter_possible = Possible::Next;
        }
        let (minuter, minuter_recount) =
            get_val(minuter_possible, &self.minuters, datetime.minuter);
        if minuter_recount {
            minuter_possible = Possible::Min;
            hour_possible = Possible::Next;
        }
        let (hour, hour_recount) = get_val(hour_possible, &self.hours, datetime.hour);
        if hour_recount {
            hour_possible = Possible::Min;
            day_possible = Possible::Next;
        }
        trace!(
            "{:?} {:?} {:?} {:?}",
            day_possible,
            hour_possible,
            minuter_possible,
            second_possible
        );
        let day_week_possible = day_possible;
        let time_next = NaiveTime::from_hms(hour, minuter as u32, second as u32);
        //
        let date_month = if let Some(month_days) = &self.month_days {
            // 计算月日期的下个日期
            let (mut month_day, month_day_recount) =
                get_val(day_possible, month_days, datetime.month_day);
            let year = datetime.date.year();
            let month = datetime.date.month();
            trace!(
                "{:?} {:?} {:?} {:?}",
                month_day,
                month_day_recount,
                day_possible,
                datetime.month_day
            );
            if !month_day_recount {
                // 这个月的日期：
                if let Some(date) = NaiveDate::from_ymd_opt(year, month, month_day) {
                    Some(date)
                } else {
                    day_possible = Possible::Min;
                    month_day = month_days.min_val();
                    // 下个月：月数+1，年也许也要加+1
                    Some(add_month(year, month, month_day)?)
                }
            } else {
                // 下个月：月数+1，年也许也要加+1
                Some(add_month(year, month, month_day)?)
            }
        } else {
            None
        };
        let date_week = if let Some(week_days) = &self.week_days {
            let (week_day, week_day_recount) =
                get_val(day_week_possible, week_days, datetime.week_day);
            trace!(
                "week: {:?} {:?} {:?} {:?}",
                week_day,
                day_week_possible,
                datetime.week_day,
                week_day_recount
            );
            if week_day_recount {
                let mut date = datetime.date.clone();
                date += Duration::days((week_day + 7 - datetime.week_day.as_data()) as i64);
                Some(date)
            } else {
                let mut date = datetime.date.clone();
                date += Duration::days((week_day - datetime.week_day.as_data()) as i64);
                Some(date)
            }
        } else {
            None
        };
        trace!("{:?} {:?}", date_month, date_week);
        let date = if let Some(date_month) = date_month {
            if let Some(date_week) = date_week {
                if date_month > date_week {
                    date_week
                } else {
                    date_month
                }
            } else {
                date_month
            }
        } else {
            date_week.unwrap()
        };
        Ok(NaiveDateTime::new(date, time_next))
    }
}
///
/// 依据Possible，获取对应的值
/// return( 获取的值,是否重新开始)
fn get_val<D: Operator>(possible: Possible, d: &D, oneself: D::DataTy) -> (D::ValTy, bool) {
    let mut re_count = false;
    let data = match possible {
        Possible::Min => d.min_val(),
        Possible::Oneself => oneself.as_data(),
        Possible::Next => {
            if let Some(data) = d.next(oneself) {
                data
            } else {
                re_count = true;
                d.min_val()
            }
        }
    };
    (data, re_count)
}

fn add_month(mut year: i32, mut month: u32, day: u32) -> Result<NaiveDate> {
    let mut add_month_times = 0;
    loop {
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            return Ok(date);
        }
        add_month_times += 1;
        if add_month_times > 12 {
            bail!("todo")
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Possible {
    Min,
    Oneself,
    Next,
}
#[derive(Debug)]
pub struct NextTime {
    hours: u64,
    minuters: u64,
    seconds: u64,
}
impl NextTime {
    fn init(mut times: u64) -> Self {
        let seconds = times % 60;
        times = times / 60;
        let minuters = times % 60;
        times = times / 60;
        let hours = times % 60;
        Self {
            seconds,
            minuters,
            hours,
        }
    }
}

impl From<MonthDays> for DayConfBuilder {
    fn from(builder: MonthDays) -> Self {
        DayHourMinuterSecondConf::default_month_days(builder)
    }
}
impl From<WeekDays> for DayConfBuilder {
    fn from(builder: WeekDays) -> Self {
        DayHourMinuterSecondConf::default_week_days(builder)
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
impl Debug for Minuters {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == u64::MAX >> 4 {
            write!(f, "all minuters.")
        } else {
            write!(f, "minuters: {:?}.", self.to_vec())
        }
    }
}
impl Debug for Hours {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == u32::MAX >> 8 {
            write!(f, "all hours.")
        } else {
            write!(f, "hours: {:?}.", self.to_vec())
        }
    }
}
impl Debug for MonthDays {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == u32::MAX << 1 {
            write!(f, "all month days.")
        } else {
            write!(f, "month days: {:?}.", self.to_vec())
        }
    }
}
impl Debug for WeekDays {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == u8::MAX << 1 {
            write!(f, "all week days.")
        } else {
            write!(f, "week days: {:?}.", self.to_vec())
        }
    }
}
#[cfg(test)]
mod test {
    use super::{get_val, DayHourMinuterSecondConf, Possible};
    use super::{Hours, Minuters, MonthDays, Operator, Seconds, WeekDays};
    use crate::data::{DateTime, Hour, Minuter, MonthDay, Second, WeekDay};
    use crate::{D31, H12};
    use anyhow::Result;
    use chrono::{Datelike, NaiveDate};
    use log::LevelFilter;

    #[test]
    fn test_get_val() -> Result<()> {
        let some_seconds = Seconds::default_range(Second::S10..Second::S30)?
            .add_range(Second::S40..=Second::S50)?;
        assert_eq!(
            (10, false),
            get_val(Possible::Min, &some_seconds, Second::S40)
        );
        assert_eq!(
            (40, false),
            get_val(Possible::Oneself, &some_seconds, Second::S40)
        );
        assert_eq!(
            (41, false),
            get_val(Possible::Next, &some_seconds, Second::S40)
        );
        assert_eq!(
            (40, false),
            get_val(Possible::Next, &some_seconds, Second::S29)
        );
        assert_eq!(
            (10, true),
            get_val(Possible::Next, &some_seconds, Second::S50)
        );

        Ok(())
    }

    #[test]
    fn test() -> Result<()> {
        // custom_utils::logger::logger_stdout_debug();
        let conf = DayHourMinuterSecondConf::default_week_days(WeekDays::default_array(&[
            WeekDay::W5,
            WeekDay::W3,
        ]))
        .conf_month_days(MonthDays::default_array(&[
            MonthDay::D5,
            MonthDay::D15,
            MonthDay::D24,
        ]))
        .build_with_hours(Hours::default_array(&[Hour::H5, Hour::H10, Hour::H15]))
        .build_with_minuter(Minuters::default_array(&[
            Minuter::M15,
            Minuter::M30,
            Minuter::M45,
        ]))
        .build_with_second(Seconds::default_array(&[
            Second::S15,
            Second::S30,
            Second::S45,
        ]));

        let mut dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 5, 15).unwrap(),
            month_day: 15.into(),
            week_day: 7.into(),
            hour: 10.into(),
            minuter: 30.into(),
            second: 30.into(),
        };

        {
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = 45.into();
            assert!(dist == dt0_dist);
            dt0_dist.second = 31.into();
            assert!(dist != dt0_dist);
        }
        //
        {
            dt0.second = 45.into();
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = 15.into();
            dt0_dist.minuter = 45.into();
            assert!(dist == dt0_dist);
        }
        {
            dt0.second = 45.into();
            dt0.minuter = 45.into();
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = 15.into();
            dt0_dist.minuter = 15.into();
            dt0_dist.hour = 15.into();
            assert!(dist == dt0_dist);
        }
        {
            dt0.second = 45.into();
            dt0.minuter = 45.into();
            dt0.hour = 15.into();
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = 15.into();
            dt0_dist.minuter = 15.into();
            dt0_dist.hour = 5.into();
            dt0_dist.week_day = 3.into();
            dt0_dist.month_day = 18.into();
            dt0_dist.date = NaiveDate::from_ymd_opt(2022, 5, 18).unwrap();
            assert_eq!(dist, dt0_dist);
        }
        // -------------------------------
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 5, 20).unwrap(),
            month_day: 20.into(),
            week_day: 5.into(),
            hour: 15.into(),
            minuter: 45.into(),
            second: 45.into(),
        };
        {
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.week_day = 2.into();
            dt0_dist.month_day = 24.into();
            dt0_dist.second = 15.into();
            dt0_dist.minuter = 15.into();
            dt0_dist.hour = 5.into();
            dt0_dist.date = NaiveDate::from_ymd_opt(2022, 5, 24).unwrap();
            assert_eq!(dist, dt0_dist);
        }
        // -------------------------------
        let conf = DayHourMinuterSecondConf::default_week_days(WeekDays::default_array(&[
            WeekDay::W5,
            WeekDay::W3,
        ]))
        .conf_month_days(MonthDays::default_array(&[
            MonthDay::D5,
            MonthDay::D15,
            MonthDay::D31,
        ]))
        .build_with_hours(Hours::default_array(&[Hour::H5, Hour::H10, Hour::H15]))
        .build_with_minuter(Minuters::default_array(&[
            Minuter::M15,
            Minuter::M30,
            Minuter::M45,
        ]))
        .build_with_second(Seconds::default_array(&[
            Second::S15,
            Second::S30,
            Second::S45,
        ]));
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 4, 29).unwrap(),
            month_day: 29.into(),
            week_day: 5.into(),
            hour: 15.into(),
            minuter: 45.into(),
            second: 45.into(),
        };
        {
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.week_day = 3.into();
            dt0_dist.month_day = 4.into();
            dt0_dist.second = 15.into();
            dt0_dist.minuter = 15.into();
            dt0_dist.hour = 5.into();
            dt0_dist.date = NaiveDate::from_ymd_opt(2022, 5, 4).unwrap();
            assert_eq!(dist, dt0_dist);
        }
        Ok(())
    }

    #[test]
    fn test_year() -> Result<()> {
        // custom_utils::logger::logger_stdout_debug();
        let conf = DayHourMinuterSecondConf::default_month_days(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minuter(Minuters::default_array(&[Minuter::M30]))
            .build_with_second(Seconds::default_array(&[Second::S0]));
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2021, 12, 31).unwrap(),
            month_day: 31.into(),
            week_day: 5.into(),
            hour: 12.into(),
            minuter: 30.into(),
            second: 30.into(),
        };
        {
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dist.clone();
            dt0_dist.second = 0.into();
            dt0_dist.minuter = 30.into();
            dt0_dist.hour = 12.into();
            dt0_dist.week_day = 1.into();
            dt0_dist.month_day = 31.into();
            assert!(dist == dt0_dist, "{:?}", dist);
            assert!(dist.date.year() == 2022, "{:?}", dist.date);
            assert!(dist.date.month() == 1, "{:?}", dist.date);
        }
        Ok(())
    }
    #[test]
    fn test_month() -> Result<()> {
        // custom_utils::logger::logger_stdout_debug();
        let conf = DayHourMinuterSecondConf::default_month_days(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minuter(Minuters::default_array(&[Minuter::M30]))
            .build_with_second(Seconds::default_array(&[Second::S0]));
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 1, 31).unwrap(),
            month_day: 31.into(),
            week_day: 5.into(),
            hour: 12.into(),
            minuter: 30.into(),
            second: 30.into(),
        };
        {
            let dist: DateTime = conf._next(dt0)?.into();
            let mut dt0_dist = dist.clone();
            dt0_dist.second = 0.into();
            dt0_dist.minuter = 30.into();
            dt0_dist.hour = 12.into();
            dt0_dist.week_day = 4.into();
            dt0_dist.month_day = 31.into();
            assert!(dist == dt0_dist, "{:?}", dist);
            assert!(dist.date.year() == 2022, "{:?}", dist.date);
            assert!(dist.date.month() == 3, "{:?}", dist.date);
        }
        Ok(())
    }
}
