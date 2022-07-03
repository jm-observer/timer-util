use crate::compute::{merge_days_conf, WeekArray, YearMonth, A};
use crate::compute::{next_month, Composition, DayUnit, TimeUnit};
use crate::data::{Hour, Minuter, MonthDay, Second, WeekDay};
use crate::traits::{AsData, FromData, Operator};
use anyhow::{bail, Result};
use chrono::format::Numeric::YearMod100;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use log::debug;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Bound, RangeBounds, Sub};

#[derive(Debug, Clone)]
pub struct TimerConf {
    pub(crate) month_days: Option<MonthDays>,
    pub(crate) week_days: Option<WeekDays>,
    pub(crate) hours: Hours,
    pub(crate) minuters: Minuters,
    pub(crate) seconds: Seconds,
}

impl TimerConf {
    fn datetimes(&self, range: impl RangeBounds<NaiveDateTime>) -> Result<Vec<NaiveDateTime>> {
        // 转成 a..=b
        let mut start = match range.start_bound() {
            Bound::Unbounded => bail!("不支持该模式"),
            Bound::Included(first) => first.sub(Duration::seconds(1)),
            Bound::Excluded(first) => first.clone(),
        };
        let end = match range.end_bound() {
            Bound::Unbounded => bail!("不支持该模式"),
            Bound::Included(end) => end.clone(),
            Bound::Excluded(end) => end.sub(Duration::seconds(1)),
        };
        if start >= end {
            bail!("起始-结束日期配置错误")
        }
        let mut start_year_month = YearMonth::new(start.year(), start.month());
        let start_tmp = start_year_month.clone();
        let end_year_month = YearMonth::new(end.year(), end.month());
        let hours = self.hours.to_vec();
        let mins: Vec<u32> = self
            .minuters
            .to_vec()
            .into_iter()
            .map(|x| x as u32)
            .collect::<Vec<u32>>();
        let seconds: Vec<u32> = self
            .seconds
            .to_vec()
            .into_iter()
            .map(|x| x as u32)
            .collect::<Vec<u32>>();
        let mut datetime = Vec::default();
        while start_year_month <= end_year_month {
            let days = self.datetime_by_month(&start_year_month).to_vec();
            let mut a = A::new(
                days.as_slice(),
                hours.as_slice(),
                mins.as_slice(),
                seconds.as_slice(),
            );
            debug!("{:?}", a);
            if start_year_month == start_tmp {
                if !a.filter_bigger(start.day(), start.hour(), start.minute(), start.second()) {
                    start_year_month.add_month();
                    continue;
                }
            }
            debug!("{:?}", a);
            if start_year_month == end_year_month {
                if !a.filter_small(end.day(), end.hour(), end.minute(), end.second()) {
                    start_year_month.add_month();
                    continue;
                }
            }
            debug!("{:?}", a);
            datetime.append(&mut a.generate_datetime(&start_year_month));
            start_year_month.add_month();
        }
        Ok(datetime)
    }
    fn datetime_by_month(&self, year_month: &YearMonth) -> MonthDays {
        let first_week_day: WeekDay = NaiveDate::from_ymd(year_month.year, year_month.month, 1)
            .weekday()
            .into();
        merge_days_conf(
            self.month_days.clone(),
            self.week_days.clone(),
            first_week_day,
        )
    }
    pub fn _next_2(&self, now: NaiveDateTime) -> NaiveDateTime {
        let now = now.add(Duration::seconds(1));
        let mut composition = Composition::from(
            now,
            self.month_days.clone(),
            self.week_days.clone(),
            self.hours.clone(),
            self.minuters.clone(),
            self.seconds.clone(),
        );
        debug!("Composition: {:?}", composition);
        let next = composition.next();
        next
    }
    pub fn next(&self) -> u64 {
        let now_local = Local::now().naive_local();
        let next_local = self._next_2(now_local);
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

#[allow(dead_code)]
impl WeekDays {
    const DEFAULT_MAX: u8 = u8::MAX << 1;
    const ONE: u8 = 1;
    const ZERO: u8 = 0;
    const MIN: u8 = 1;
    const MAX: u8 = 7;

    pub(crate) fn to_month_days(&self, start: WeekDay) -> MonthDays {
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
    pub fn default_value(val: WeekDay) -> Self {
        let ins = Self::_default();
        ins.add(val)
    }
    #[inline]
    pub fn default_range(range: impl RangeBounds<WeekDay>) -> Result<Self> {
        let ins = Self::_default();
        ins.add_range(range)
    }
    #[inline]
    pub fn default_all() -> Self {
        let mut ins = Self::_default();
        ins._val_mut(Self::DEFAULT_MAX);
        ins
    }
    pub fn default_array(vals: &[WeekDay]) -> Self {
        let ins = Self::_default();
        ins.add_array(vals)
    }
    pub fn add_array(mut self, vals: &[WeekDay]) -> Self {
        let mut val = self._val();
        for i in vals {
            val |= Self::ONE << i.as_data();
        }
        self._val_mut(val);
        self
    }
    pub fn add(mut self, index: WeekDay) -> Self {
        let index = index.as_data();
        self._val_mut(self._val() | (Self::ONE << index));
        self
    }
    pub fn add_range(mut self, range: impl RangeBounds<WeekDay>) -> Result<Self> {
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
//
// #[derive(Debug)]
// pub struct NextTime {
//     hours: u64,
//     minuters: u64,
//     seconds: u64,
// }
// impl NextTime {
//     fn init(mut times: u64) -> Self {
//         let seconds = times % 60;
//         times = times / 60;
//         let minuters = times % 60;
//         times = times / 60;
//         let hours = times % 60;
//         Self {
//             seconds,
//             minuters,
//             hours,
//         }
//     }
// }

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

#[cfg(test)]
mod test {
    use super::{Hours, Minuters, MonthDays, Operator, Seconds, WeekDays};
    use crate::compute::datetime;
    use crate::conf::TimerConf;
    use crate::data::{DateTime, Hour::*, Minuter::*, MonthDay::*, Second::*, WeekDay::*};
    use crate::*;
    use anyhow::Result;
    use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
    use log::debug;

    /// 测试WeekDays生成当月的月日期
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
        // debug!("{:?}", month_days.to_vec());
    }
    #[test]
    fn test_datetimes() -> Result<()> {
        custom_utils::logger::logger_stdout_debug();
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
            .build_with_minuter(Minuters::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_array(&[S15, S30, S45]));
        debug!("2020-5-15 10:30:17");
        let datetimes =
            conf.datetimes(datetime(2020, 5, 15, 10, 30, 17)..=datetime(2020, 5, 15, 15, 30, 30))?;

        assert_eq!(datetimes.as_slice(), &some_datetimes[..]);

        Ok(())
    }

    #[test]
    fn test() -> Result<()> {
        custom_utils::logger::logger_stdout_debug();
        let conf = configure_weekday(WeekDays::default_array(&[W5, W3]))
            .conf_month_days(MonthDays::default_array(&[D5, D15, D24]))
            .build_with_hours(Hours::default_array(&[H5, H10, H15]))
            .build_with_minuter(Minuters::default_array(&[M15, M30, M45]))
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
        let conf = configure_weekday(WeekDays::default_array(&[W5, W3]))
            .conf_month_days(MonthDays::default_array(&[D5, D15, D31]))
            .build_with_hours(Hours::default_array(&[H5, H10, H15]))
            .build_with_minuter(Minuters::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_array(&[S15, S30, S45]));
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
        let conf = configure_monthday(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minuter(Minuters::default_array(&[M30]))
            .build_with_second(Seconds::default_array(&[S0]));
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
        custom_utils::logger::logger_stdout_debug();
        let conf = configure_monthday(MonthDays::default_value(D31))
            .build_with_hours(Hours::default_array(&[H12]))
            .build_with_minuter(Minuters::default_array(&[M30]))
            .build_with_second(Seconds::default_array(&[S0]));
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

    fn compare(conf: &TimerConf, times: &[NaiveDateTime]) {
        let len = times.len() - 1;
        let mut index = 0;
        loop {
            assert_eq!(conf._next_2(times[index].clone()), times[index + 1].clone());
            index += 1;
            if index == len {
                break;
            }
        }
    }
}
