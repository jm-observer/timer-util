use crate::compute::Composition;
use crate::data::{Hour, Minuter, MonthDay, Second, WeekDay};
use crate::traits::{FromData, ConfigOperator};
use anyhow::{bail, Result};
use chrono::{Datelike, Duration, Local, NaiveDateTime, Timelike};
use log::debug;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Bound, RangeBounds, Sub};

/// 定时器配置
#[derive(Debug, Clone)]
pub struct TimerConf {
    pub(crate) days: Days,
    pub(crate) hours: Hours,
    pub(crate) minuters: Minuters,
    pub(crate) seconds: Seconds,
}

impl TimerConf {

    /// 在给定的日期时间范围内，返回符合定时器的所有时间点
    pub fn datetimes(&self, range: impl RangeBounds<NaiveDateTime>) -> Result<Vec<NaiveDateTime>> {
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
    /// 以给定的时间点为起点(不包含该时点)，返回下个符合定时器的时间点
    pub fn next_with_time(&self, now: NaiveDateTime) -> NaiveDateTime {
        let now = now.add(Duration::seconds(1));
        let mut composition = Composition::from(
            now,
            self.days.clone(),
            self.hours.clone(),
            self.minuters.clone(),
            self.seconds.clone(),
        );
        debug!("Composition: {:?}", composition);
        let next = composition.next();
        next
    }
    /// 以当前时间点为起点，返回距离下个符合时间点的时间间隔（s）
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
#[derive(Debug, Clone)]
pub enum Days {
    MonthDays(MonthDays),
    WeekDays(WeekDays),
    MonthAndWeekDays(MonthDays, WeekDays)
}

impl Days {
    pub(crate) fn month_days(&self, week_day: WeekDay) -> MonthDays {
        match self {
            Days::MonthDays(month_days) => { month_days.clone()}
            Days::WeekDays(week_days) => {week_days.to_month_days(week_day)}
            Days::MonthAndWeekDays(month_days, week_days) => {
                month_days.merge(&week_days.to_month_days(week_day))
            }
        }
    }
    pub(crate) fn update_month_days(self, month_days: MonthDays) -> Self {
        match self {
            Days::MonthDays(_) => {Self::MonthDays(month_days)}
            Days::WeekDays(week_days) => {Self::MonthAndWeekDays(month_days, week_days)}
            Days::MonthAndWeekDays(_, week_days) => {Self::MonthAndWeekDays(month_days, week_days)}
        }
    }
    pub(crate) fn update_week_days(self, week_days: WeekDays) -> Self {
        match self {
            Days::MonthDays(month_days) => {Self::MonthAndWeekDays(month_days, week_days)}
            Days::WeekDays(_) => {Self::WeekDays(week_days)}
            Days::MonthAndWeekDays(month_days, _) => {Self::MonthAndWeekDays(month_days, week_days)}
        }
    }
    // pub(crate) fn is_zero(&self) -> bool {
    //     match self {
    //         Days::MonthDays(month_days) => month_days.is_zero(),
    //         Days::WeekDays(week_days) => week_days.is_zero(),
    //         Days::MonthAndWeekDays(month_days, week_days) => month_days.is_zero() || week_days.is_zero()
    //     }
    // }
}


/// 每月的天数配置。如配置（选中）1号、3号……29号
#[derive(Clone)]
pub struct MonthDays(u64);
/// 每星期的天数配置。如配置（选中）周一……周六
#[derive(Clone)]
pub struct WeekDays(u64);
/// 每天的小时（时钟）配置。如配置（选中）0点、3点、9点、……18点
#[derive(Clone)]
pub struct Hours(u64);
/// 每小时的分钟配置。如配置（选中）0分、5分……58分
#[derive(Clone, Eq, PartialEq)]
pub struct Minuters(u64);
/// 每分钟的秒钟配置。如配置（选中）0秒、5秒……58秒
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
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
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
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
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
impl ConfigOperator for Minuters {
    const MIN: u64 = 0;
    const MAX: u64 = 59;
    const DEFAULT_MAX: u64 = u64::MAX >> 4;
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
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
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

impl Minuters {
    pub fn every(interval: u64) -> Self {
        if interval == 0 {
            Self::_default()
        } else {
            let mut val = 0u64;
            let mut minuters = Self::_default();
            while val <= Self::MAX {
                minuters = minuters.add(Minuter::from_data(val));
                val += interval
            }
            minuters
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
        self._next(index)
            .and_then(|x| Some(Self::DataTy::from_data(x)))
    }

    fn _val(&self) -> u64 {
        self.0
    }

    fn _val_mut(&mut self, val: u64) {
        self.0 = val;
    }
}

/// 为啥不是实现Operator
#[allow(dead_code)]
impl WeekDays {
    pub(crate) fn to_month_days(&self, start: WeekDay) -> MonthDays {
        // 因WeekDays起始位置为1,右移去掉冗余的0位
        let week_unit = self.0 >> 1;
        // 按7天，拼出足够长的天数（保证下一步截断后，总天数>= 31）
        // 按起始星期几截断
        // 因MonthDays起始位置为1,再左移1位
        let days = (week_unit | week_unit << 7 | week_unit << 14 | week_unit << 21 | week_unit << 28 | week_unit << 35) >> (start as u64 - 1) << 1;

        let mut month_days = MonthDays::_default();
        month_days._val_mut(days);
        month_days
        //
        //
        // let mut next = Some(WeekArray::init(start));
        // let conf_week_days = self.to_vec();
        //
        // let mut monthdays = MonthDays::_default();
        // while let Some(ref weekday) = next {
        //     for x in &conf_week_days {
        //         if let Some(day) = weekday.day(*x as usize) {
        //             monthdays = monthdays.add(MonthDay::from_data(day));
        //         }
        //     }
        //     next = weekday.next();
        // }
        // monthdays
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
    use super::{Hours, Minuters, MonthDays, ConfigOperator, Seconds, WeekDays};
    use crate::conf::TimerConf;
    #[allow(unused_imports)]
    use crate::data::{DateTime, Hour::*, Minuter::*, MonthDay::*, Second::*, WeekDay::*};
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
        // custom_utils::logger::logger_stdout_debug();
        let conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
            .conf_month_days(
                MonthDays::default_range(D5..D10)?
                    .add_range(D15..D20)?
                    .add_range(D25..D30)?,
            )
            .build_with_hours(Hours::default_array(&[H5, H10, H15, H20]))
            .build_with_minuter(Minuters::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_value(S0));

        let mut start = datetime(2022, 7, 4, 20, 15, 0);
        let end = datetime(2033, 8, 15, 12, 30, 45);

        // let datetimes = conf.datetimes(start.clone()..end)?;
        let datetimes = conf.datetimes(start.clone()..end)?;
        start = start.sub(Duration::seconds(1));
        let mut next = end;
        for datetime in datetimes {
            next = conf.next_with_time(start.clone());
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
        // let datetimes = conf.datetimes(start.clone()..end.clone())?;
        let datetimes = conf.datetimes(start.clone()..end.clone())?;
        start = start.sub(Duration::seconds(1));
        let mut next = start.clone();
        for datetime in datetimes {
            next = conf.next_with_time(start.clone());
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
        // custom_utils::logger::logger_stdout_debug();
        let conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
            .conf_month_days(
                MonthDays::default_range(D5..D10)?
                    .add_range(D15..D20)?
                    .add_range(D25..D30)?,
            )
            .build_with_hours(Hours::default_array(&[H5, H10, H15, H20]))
            .build_with_minuter(Minuters::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_value(S0));

        let start = datetime(2022, 7, 4, 22, 17, 10);
        let end = datetime(2022, 7, 5, 12, 30, 45);

        // let datetimes = conf.datetimes(start.clone()..end)?;
        let datetimes = conf.datetimes(start.clone()..end)?;
        debug!("{:?}", datetimes);
        Ok(())
    }

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
        // custom_utils::logger::logger_stdout_debug();
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
        // let datetimes =
        //     conf.datetimes(datetime(2020, 5, 15, 10, 30, 17)..=datetime(2020, 5, 15, 15, 30, 30))?;
        let datetimes =
            conf.datetimes(datetime(2020, 5, 15, 10, 30, 17)..=datetime(2020, 5, 15, 15, 30, 30))?;

        assert_eq!(datetimes.as_slice(), &some_datetimes[..]);

        Ok(())
    }

    #[test]
    fn test() -> Result<()> {
        // custom_utils::logger::logger_stdout_debug();
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
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
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
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
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
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
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
            let dist: DateTime = conf.next_with_time(dt0.into()).into();
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
            assert_eq!(
                conf.next_with_time(times[index].clone()),
                times[index + 1].clone()
            );
            index += 1;
            if index == len {
                break;
            }
        }
    }
    #[test]
    fn test_every() {
        use Minuter::*;
        let minuters = Minuters::every(11);
        assert_eq!(minuters, Minuters::default_array(&[M0, M11, M22, M33, M44, M55]));
        let minuters = Minuters::every(0);
        assert_eq!(minuters, Minuters::_default());
        let minuters = Minuters::every(30);
        assert_eq!(minuters, Minuters::default_array(&[M0, M30]));
        let minuters = Minuters::every(31);
        assert_eq!(minuters, Minuters::default_array(&[M0, M31]));
        let minuters = Minuters::every(60);
        assert_eq!(minuters, Minuters::default_array(&[M0]));

        let minuters = Minuters::every(500);
        assert_eq!(minuters, Minuters::default_array(&[M0]));
    }
}
