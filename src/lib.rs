mod compute;
pub mod data;

pub use data::{
    Hour::{self, *},
    Minuter::{self, *},
    MonthDay::{self, *},
    Second::{self, *},
    WeekDay::{self, *},
};

use crate::compute::{next_month, Composition, DayUnit, TimeUnit};
use crate::data::FromData;
use anyhow::{bail, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use data::AsData;
use log::debug;
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
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
    }
    fn _val_mut(&mut self, val: Self::ValTy) {
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
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
    }
    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _val_mut(&mut self, val: Self::ValTy) {
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
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
    }
    fn _default() -> Self {
        Self(0)
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _val_mut(&mut self, val: Self::ValTy) {
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
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
    }
    fn _default() -> Self {
        Self(0)
    }
    fn min_val(&self) -> Self::DataTy {
        Self::DataTy::from_data(self._min_val())
    }
    fn _val(&self) -> Self::ValTy {
        self.0
    }
    fn _val_mut(&mut self, val: Self::ValTy) {
        self.0 = val
    }
}

pub trait Operator: Sized {
    /// 最小值：比如星期配置，则最小为星期1，即为1
    const MIN: Self::ValTy;
    /// 最大值：比如星期配置，则最大为星期日，即为7
    const MAX: Self::ValTy;
    /// 单位值：好像全为1
    const ONE: Self::ValTy;
    /// 0值：即全不选的值，比如星期7天都不选，则为二进制0000 0000
    const ZERO: Self::ValTy;
    /// 满值：即全选的值，比如星期7天全选，则为二进制1111 1110
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

    type DataTy: AsData<Self::ValTy> + Copy + Clone;

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
        ins._val_mut(Self::DEFAULT_MAX);
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
        self._val_mut(val);
        self
    }
    fn add(mut self, index: Self::DataTy) -> Self {
        let index = index.as_data();
        self._val_mut(self._val() | (Self::ONE << index));
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
        self._val_mut(val);
        Ok(self)
    }

    fn merge(&self, other: &Self) -> Self {
        let mut new = Self::_default();
        new._val_mut(self._val() | other._val());
        new
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
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy>;
    /// 取下一个持有值，不包括index
    fn _next(&self, index: Self::DataTy) -> Option<Self::ValTy> {
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
    fn min_val(&self) -> Self::DataTy;
    /// 取最小的持有值
    fn _min_val(&self) -> Self::ValTy {
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
    fn _val_mut(&mut self, val: Self::ValTy);
}

// impl Operator for WeekDays {
//     const MIN: Self::ValTy = 1;
//     const MAX: Self::ValTy = 7;
//     const ONE: Self::ValTy = 1;
//     const ZERO: Self::ValTy = 0;
//     const DEFAULT_MAX: Self::ValTy = u8::MAX << 1;
//     type ValTy = u8;
//     type DataTy = WeekDay;
//
//     fn _default() -> Self {
//         Self(0)
//     }
//
//     fn next(&self, index: Self::DataTy) -> Option<Self::DataTy> {
//         self._next(index)
//             .and_then(|x| Some(Self::DataTy::from_data(x)))
//     }
//
//     fn min_val(&self) -> Self::DataTy {
//         Self::DataTy::from_data(self._min_val())
//     }
//
//     fn _val(&self) -> Self::ValTy {
//         self.0
//     }
//     fn _val_mut(&mut self, val: Self::ValTy) {
//         self.0 = val
//     }
// }
#[allow(dead_code)]
impl WeekDays {
    const DEFAULT_MAX: u8 = u8::MAX << 1;
    const ONE: u8 = 1;
    const ZERO: u8 = 0;
    const MIN: u8 = 1;
    const MAX: u8 = 7;

    fn to_month_days(&self, start: WeekDay) -> MonthDays {
        let mut next = Some(WeekArray::init(start));
        let conf_week_days = self.to_vec();

        let mut monthdays = MonthDays::_default();
        while let Some(ref weekday) = next {
            for x in &conf_week_days {
                if let Some(day) = weekday.day(*x) {
                    monthdays = monthdays.add(MonthDay::from_data(day));
                }
            }
            next = weekday.next();
        }
        monthdays
    }

    fn _default() -> Self {
        Self(0)
    }
    #[inline]
    fn default_value(val: WeekDay) -> Self {
        let ins = Self::_default();
        ins.add(val)
    }
    #[inline]
    fn default_range(range: impl RangeBounds<WeekDay>) -> Result<Self> {
        let ins = Self::_default();
        ins.add_range(range)
    }
    #[inline]
    fn default_all() -> Self {
        let mut ins = Self::_default();
        ins._val_mut(Self::DEFAULT_MAX);
        ins
    }
    fn default_array(vals: &[WeekDay]) -> Self {
        let ins = Self::_default();
        ins.add_array(vals)
    }
    fn add_array(mut self, vals: &[WeekDay]) -> Self {
        let mut val = self._val();
        for i in vals {
            val |= Self::ONE << i.as_data();
        }
        self._val_mut(val);
        self
    }
    fn add(mut self, index: WeekDay) -> Self {
        let index = index.as_data();
        self._val_mut(self._val() | (Self::ONE << index));
        self
    }
    fn add_range(mut self, range: impl RangeBounds<WeekDay>) -> Result<Self> {
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
        self._val_mut(val);
        Ok(self)
    }
    fn to_vec(&self) -> Vec<usize> {
        let mut res = Vec::new();
        let val = self._val();
        let mut first = Self::MIN;
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                res.push((first - 1) as usize);
            }
            first += Self::ONE;
        }
        res
    }
    fn _val_mut(&mut self, val: u8) {
        self.0 = val;
    }
    fn _val(&self) -> u8 {
        self.0
    }
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
    pub fn _next_2(&self, now: NaiveDateTime) -> NaiveDateTime {
        let now = now.add(Duration::seconds(1));
        let year = now.year();
        let month = now.month();
        let day = MonthDay::from_data(now.day());

        let first_week_day: WeekDay = NaiveDate::from_ymd(year, month, 1).weekday().into();
        let max = next_month(year, month).pred().day();

        let day_unit = DayUnit::new(
            year,
            month,
            self.month_days.clone(),
            self.week_days.clone(),
            day,
            first_week_day,
            max,
        );
        let hour: TimeUnit<Hours> = TimeUnit::new(Hour::from_data(now.hour()), self.hours.clone());
        let minuter = TimeUnit::new(
            Minuter::from_data(now.minute() as u64),
            self.minuters.clone(),
        );
        let second = TimeUnit::new(Second::from_data(now.second() as u64), self.seconds.clone());
        let mut composition = Composition::new(day_unit, hour, minuter, second);
        debug!("Composition: {:?}", composition);
        let next = composition.next();
        next
        // (next.timestamp() - now.timestamp()) as u64
    }
    pub fn next(&self) -> u64 {
        let now_local = Local::now().naive_local();
        let next_local = self._next_2(now_local);
        let times = (next_local.timestamp() - now_local.timestamp()) as u64;
        let next_time = NextTime::init(times);
        debug!(
            "now: {}, next: {}, next time is after {:?} = {}s",
            now_local, next_local, next_time, times
        );
        times
    }
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
            write!(f, "week day's array index: {:?}.", self.to_vec())
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
struct WeekArray {
    days: [i8; 7],
    // max: i8,
}
impl WeekArray {
    fn init(start: WeekDay) -> Self {
        let mut init_week = [1i8; 7];
        if start.as_data() >= 2 {
            let mut index = (start.as_data() - 2) as usize;
            let mut diff = 1;
            loop {
                init_week[index] -= diff;
                diff += 1;
                if index == 0 {
                    break;
                }
                index -= 1;
            }
        }
        let mut index = (start.as_data()) as usize;
        let mut diff = 1;
        while index < 7 {
            init_week[index] += diff;
            diff += 1;
            index += 1;
        }
        Self {
            days: init_week,
            // max,
        }
    }

    fn day(&self, index: usize) -> Option<u32> {
        let day = self.days[index];
        if day > 0 && day <= 31 {
            Some(day as u32)
        } else {
            None
        }
    }

    fn next(&self) -> Option<Self> {
        let mut days = self.days;
        for i in days.iter_mut() {
            *i = *i + 7;
        }
        if days[0] > 31 {
            None
        } else {
            Some(Self { days })
        }
    }
}
#[cfg(test)]
mod test {
    use super::DayHourMinuterSecondConf;
    use super::{Hours, Minuters, MonthDays, Operator, Seconds, WeekDays};
    use crate::data::{DateTime, Hour, Minuter, MonthDay, Second, WeekDay};
    use crate::*;
    use anyhow::Result;
    use chrono::{Datelike, NaiveDate};

    /// 测试WeekDays生成当月的月日期
    #[test]
    fn test_to_month_days() {
        custom_utils::logger::logger_stdout_debug();
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
        // debug!("{:?}", month_days.to_vec());
    }

    /// 测试WeekArray的init和next方法
    #[test]
    fn test_init_first_week() {
        assert_eq!(
            WeekArray::init(W3),
            WeekArray {
                days: [-1, 0, 1, 2, 3, 4, 5],
            }
        );
        assert_eq!(
            WeekArray::init(W1),
            WeekArray {
                days: [1, 2, 3, 4, 5, 6, 7],
            }
        );
        assert_eq!(
            WeekArray::init(W7),
            WeekArray {
                days: [-5, -4, -3, -2, -1, 0, 1],
            }
        );
        {
            let next = WeekArray::init(W7).next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [2, 3, 4, 5, 6, 7, 8]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [9, 10, 11, 12, 13, 14, 15]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [16, 17, 18, 19, 20, 21, 22]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [23, 24, 25, 26, 27, 28, 29]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [30, 31, 32, 33, 34, 35, 36]);

            let next = next.next();
            assert!(next.is_none());
        }
        {
            let next = WeekArray::init(W3).next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [6, 7, 8, 9, 10, 11, 12]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [13, 14, 15, 16, 17, 18, 19]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [20, 21, 22, 23, 24, 25, 26]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [27, 28, 29, 30, 31, 32, 33]);

            let next = next.next();
            assert!(next.is_none());
        }

        {
            let next = WeekArray::init(W1).next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [8, 9, 10, 11, 12, 13, 14]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [15, 16, 17, 18, 19, 20, 21]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [22, 23, 24, 25, 26, 27, 28]);

            let next = next.next();
            assert!(next.is_some());
            let next = next.unwrap();
            assert_eq!(next.days, [29, 30, 31, 32, 33, 34, 35]);

            let next = next.next();
            assert!(next.is_none());
        }
    }

    #[test]
    fn test() -> Result<()> {
        custom_utils::logger::logger_stdout_debug();
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
            month_day: D15,
            week_day: W7,
            hour: H10,
            minuter: M30,
            second: S30,
        };

        {
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = S45;

            assert!(dist == dt0_dist);
            dt0_dist.second = S31;
            assert!(dist != dt0_dist);
        }
        //
        {
            dt0.second = S45;
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = S15;
            dt0_dist.minuter = M45;
            assert!(dist == dt0_dist);
        }
        {
            dt0.second = S45;
            dt0.minuter = M45;
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = S15;
            dt0_dist.minuter = M15;
            dt0_dist.hour = H15;
            assert!(dist == dt0_dist);
        }
        {
            let dt0 = DateTime {
                date: NaiveDate::from_ymd_opt(2022, 5, 15).unwrap(),
                month_day: D15,
                week_day: W7,
                hour: H15,
                minuter: M45,
                second: S45,
            };
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.second = S15;
            dt0_dist.minuter = M15;
            dt0_dist.hour = H5;
            dt0_dist.week_day = W3;
            dt0_dist.month_day = D18;
            dt0_dist.date = NaiveDate::from_ymd_opt(2022, 5, 18).unwrap();
            debug!("{:?}", dist);
            debug!("{:?}", dt0_dist);
            assert_eq!(dist, dt0_dist);
        }
        // -------------------------------
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 5, 20).unwrap(),
            month_day: D20,
            week_day: W5,
            hour: H15,
            minuter: M45,
            second: S45,
        };
        {
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.week_day = W2;
            dt0_dist.month_day = D24;
            dt0_dist.second = S15;
            dt0_dist.minuter = M15;
            dt0_dist.hour = H5;
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
        debug!("{:?}", conf);
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 4, 29).unwrap(),
            month_day: D29,
            week_day: W5,
            hour: H15,
            minuter: M45,
            second: S45,
        };
        {
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dt0.clone();
            dt0_dist.week_day = W3;
            dt0_dist.month_day = D4;
            dt0_dist.second = S15;
            dt0_dist.minuter = M15;
            dt0_dist.hour = H5;
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
            month_day: D31,
            week_day: W5,
            hour: H12,
            minuter: M30,
            second: S30,
        };
        {
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dist.clone();
            dt0_dist.second = S0;
            dt0_dist.minuter = M30;
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
        // custom_utils::logger::logger_stdout_debug();
        let conf = DayHourMinuterSecondConf::default_month_days(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minuter(Minuters::default_array(&[Minuter::M30]))
            .build_with_second(Seconds::default_array(&[Second::S0]));
        let dt0 = DateTime {
            date: NaiveDate::from_ymd_opt(2022, 1, 31).unwrap(),
            month_day: D31,
            week_day: W5,
            hour: H12,
            minuter: M30,
            second: S30,
        };
        {
            let dist: DateTime = conf._next_2(dt0.into()).into();
            let mut dt0_dist = dist.clone();
            dt0_dist.second = S0;
            dt0_dist.minuter = M30;
            dt0_dist.hour = H12;
            dt0_dist.week_day = W4;
            dt0_dist.month_day = D31;
            assert!(dist == dt0_dist, "{:?}", dist);
            assert!(dist.date.year() == 2022, "{:?}", dist.date);
            assert!(dist.date.month() == 3, "{:?}", dist.date);
        }
        Ok(())
    }
}
