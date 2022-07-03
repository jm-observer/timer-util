use crate::conf::{Hours, Minuters, MonthDays, Seconds, WeekDays};
use crate::data::{Hour, Minuter, MonthDay, Second, WeekDay};
use crate::traits::{AsData, Computer, FromData, Operator};
use anyhow::bail;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use log::debug;

#[derive(Debug)]
pub struct TimeUnit<T: Operator> {
    // 最大值
    // max: T::ValTy,
    // 当前的起始值
    index: T::DataTy,
    // 对应的配置
    conf: T,
    // 最后的值
    val: T::ValTy,
}
#[derive(Debug)]
pub struct DayUnit {
    // 当前的起始值
    year: i32,
    // 最后的值
    month: u32,
    monthdays: Option<MonthDays>,
    weekdays: Option<WeekDays>,
    day: MonthDay,
    max: u32,
    conf: MonthDays,
    val: u32,
}

impl Computer for DayUnit {
    const MIN: Self::ValTy = 1;
    type ValTy = u32;
    type DataTy = MonthDay;

    fn update_to_next_ring(&mut self) {
        loop {
            if self.month == 12 {
                self.month = 1;
                self.year += 1;
            } else {
                self.month += 1;
            }
            let date = NaiveDate::from_ymd(self.year, self.month, 1);
            let next_month = next_month(self.year, self.month);
            let weekday: WeekDay = date.weekday().into();
            self.conf = merge_days_conf(self.monthdays.clone(), self.weekdays.clone(), weekday);
            self.max = next_month.pred().day();
            self.day = self.conf.min_val();
            if self.day.as_data() <= self.max {
                self.val = self.day.as_data();
                break;
            }
        }
    }

    fn is_match(&self) -> bool {
        self.conf.contain(self.day)
    }

    fn next_val(&self) -> Option<Self::DataTy> {
        if let Some(next) = self.conf.next(self.day) {
            if next.as_data() > self.max {
                None
            } else {
                Some(next)
            }
        } else {
            None
        }
    }

    fn min_val(&self) -> Self::DataTy {
        self.conf.min_val()
    }

    fn val_mut(&mut self, val: Self::DataTy) {
        self.val = val.as_data();
    }

    fn val(&self) -> Self::ValTy {
        self.day.as_data()
    }
}

// impl<T: Operator> TimeUnit<T> {
//     pub fn is_match(&self) -> bool {
//         self.conf.contain(self.index)
//     }
//     pub fn next_val(&self) -> Option<T::ValTy> {
//         self.conf.next(self.index)
//     }
//     pub fn min_val(&self) -> T::ValTy {
//         self.conf.min_val()
//     }
// }

pub fn merge_days_conf(
    monthdays: Option<MonthDays>,
    weekdays: Option<WeekDays>,
    weekday: WeekDay,
) -> MonthDays {
    let conf = if let Some(ref weekdays) = weekdays {
        let week_monthdays = weekdays.to_month_days(weekday);
        if let Some(ref monthdays) = monthdays {
            monthdays.merge(&week_monthdays)
        } else {
            week_monthdays
        }
    } else if let Some(ref monthdays) = monthdays {
        monthdays.clone()
    } else {
        unreachable!("")
    };
    conf
}

impl<T: Operator> Computer for TimeUnit<T> {
    const MIN: Self::ValTy = <T as Operator>::MIN;
    type ValTy = <T as Operator>::ValTy;
    type DataTy = T::DataTy;

    fn update_to_next_ring(&mut self) {
        self.index = self.conf.min_val();
        self.val = self.index.as_data();
    }

    fn is_match(&self) -> bool {
        self.conf().contain(self.index())
    }
    fn next_val(&self) -> Option<Self::DataTy> {
        self.conf().next(self.index())
    }
    fn min_val(&self) -> Self::DataTy {
        self.conf().min_val()
    }
    fn val_mut(&mut self, val: T::DataTy) {
        self.val = val.as_data();
    }

    fn val(&self) -> Self::ValTy {
        self.val
    }
}

impl<T: Operator> TimeUnit<T> {
    pub fn new(index: T::DataTy, conf: T) -> Self {
        let val = index;
        Self {
            // max: T::MAX,
            index,
            conf,
            val: val.as_data(),
        }
    }

    fn conf(&self) -> &T {
        &self.conf
    }

    fn index(&self) -> T::DataTy {
        self.index
    }
}

impl DayUnit {
    pub fn new(
        year: i32,
        month: u32,
        monthdays: Option<MonthDays>,
        weekdays: Option<WeekDays>,
        day: MonthDay,
        first_week_day: WeekDay,
        max: u32,
    ) -> Self {
        let conf = merge_days_conf(monthdays.clone(), weekdays.clone(), first_week_day);
        Self {
            year,
            month,
            monthdays,
            weekdays,
            day: day.clone(),
            max,
            conf,
            val: day.as_data(),
        }
    }
}
#[derive(Debug)]
pub struct Composition {
    day: DayUnit,
    hour: TimeUnit<Hours>,
    minuter: TimeUnit<Minuters>,
    second: TimeUnit<Seconds>,
}

impl Composition {
    pub fn from(
        now: NaiveDateTime,
        month_days: Option<MonthDays>,
        week_days: Option<WeekDays>,
        hours: Hours,
        min: Minuters,
        seconds: Seconds,
    ) -> Self {
        let year = now.year();
        let month = now.month();
        let day = MonthDay::from_data(now.day());
        let first_week_day: WeekDay = NaiveDate::from_ymd(year, month, 1).weekday().into();
        let max = next_month(year, month).pred().day();
        let day_unit = DayUnit::new(year, month, month_days, week_days, day, first_week_day, max);
        let hour: TimeUnit<Hours> = TimeUnit::new(Hour::from_data(now.hour()), hours);
        let minuter = TimeUnit::new(Minuter::from_data(now.minute() as u64), min);
        let second = TimeUnit::new(Second::from_data(now.second() as u64), seconds);
        Composition::new(day_unit, hour, minuter, second)
    }
    pub fn new(
        day: DayUnit,
        hour: TimeUnit<Hours>,
        minuter: TimeUnit<Minuters>,
        second: TimeUnit<Seconds>,
    ) -> Self {
        Self {
            day,
            hour,
            minuter,
            second,
        }
    }

    pub fn next(&mut self) -> NaiveDateTime {
        loop {
            if self.day.is_match() {
                if self.match_hour() {
                    break;
                }
            }
            self.next_day();
        }
        self.to_datetime()
    }
    fn to_datetime(&self) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd(self.day.year, self.day.month, self.day.val),
            NaiveTime::from_hms(
                self.hour.val,
                self.minuter.val as u32,
                self.second.val as u32,
            ),
        )
    }
    fn match_hour(&mut self) -> bool {
        if self.hour.is_match() {
            if self.match_minuter() {
                return true;
            }
        }
        if let Some(hour) = self.hour.next_val() {
            self.hour.val_mut(hour);
            self.minuter_update_to_next_ring();
            true
        } else {
            false
        }
    }
    fn match_minuter(&mut self) -> bool {
        if self.minuter.is_match() {
            if self.match_second() {
                return true;
            }
        }
        if let Some(minuter) = self.minuter.next_val() {
            self.minuter.val_mut(minuter);
            self.second_update_to_next_ring();
            true
        } else {
            false
        }
    }
    fn match_second(&mut self) -> bool {
        if self.second.is_match() {
            return true;
        }
        if let Some(hour) = self.second.next_val() {
            self.second.val_mut(hour);
            true
        } else {
            false
        }
    }

    fn next_day(&mut self) {
        if let Some(day) = self.day.next_val() {
            debug!("day_unit: {:?}, next_day: {:?}", self.day, day);
            self.day.val_mut(day);
        } else {
            self.day.update_to_next_ring();
        }
        debug!("day_unit: {:?}", self.day);
        self.hour_update_to_next_ring();
    }
    fn hour_update_to_next_ring(&mut self) {
        self.hour.update_to_next_ring();
        self.minuter_update_to_next_ring();
    }
    fn minuter_update_to_next_ring(&mut self) {
        self.minuter.update_to_next_ring();
        self.second_update_to_next_ring();
    }
    fn second_update_to_next_ring(&mut self) {
        self.second.update_to_next_ring();
    }
}

pub fn next_month(mut year: i32, mut month: u32) -> NaiveDate {
    if month == 12 {
        month = 1;
        year += 1;
    } else {
        month += 1;
    }
    NaiveDate::from_ymd(year, month, 1)
}

#[derive(Eq, PartialEq, Debug)]
pub struct WeekArray {
    pub(crate) days: [i8; 7],
}
impl WeekArray {
    pub(crate) fn init(start: WeekDay) -> Self {
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

    pub(crate) fn day(&self, index: usize) -> Option<u32> {
        let day = self.days[index];
        if day > 0 && day <= 31 {
            Some(day as u32)
        } else {
            None
        }
    }

    pub(crate) fn next(&self) -> Option<Self> {
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

pub struct A<'a> {
    days: &'a [u32],
    hours: &'a [u32],
    minuters: &'a [u32],
    seconds: &'a [u32],
}

impl<'a> A<'a> {
    pub fn start(
        &self,
        day: u32,
        hour: u32,
        minuter: u32,
        second: u32,
    ) -> Vec<(u32, u32, u32, u32)> {
        let mut res = Vec::default();
        let mut day_index = 0;
        let mut hour_index = 0;
        let mut minuter_index = 0;
        let mut second_index = 0;
        let mut foud = false;
        while day_index < self.days.len() {
            if self.days[day_index] > day {
                foud = true;
                break;
            } else if self.days[day_index] == day {
                while hour_index < self.hours.len() {
                    if self.hours[hour_index] > hour {
                        foud = true;
                        break;
                    } else if self.hours[hour_index] == hour {
                        while minuter_index < self.minuters.len() {
                            if self.minuters[minuter_index] > minuter {
                                foud = true;
                                break;
                            } else if self.minuters[minuter_index] == hour {
                                while second_index < self.seconds.len() {
                                    if self.seconds[minuter_index] > second {
                                        foud = true;
                                        break;
                                    }
                                    second_index += 1;
                                }
                            }
                            minuter_index += 1;
                        }
                    }
                    hour_index += 1;
                }
            }
            day_index += 1;
        }
        if foud {
            let days = &self.days[day_index..];
            let hours = &self.hours[hour_index..];
            let minuters = &self.minuters[minuter_index..];
            let seconds = &self.seconds[second_index..];
            for day in days {
                for hour in hours {
                    for minuter in minuters {
                        for second in seconds {
                            res.push((*day, *hour, *minuter, *second));
                        }
                    }
                }
            }
        }
        res
    }
}

/// 找出<=给定值的索引
fn get_smaller_index(
    day: u32,
    hour: u32,
    minuter: u32,
    second: u32,
    days: &[u32],
    hours: &[u32],
    minuters: &[u32],
    seconds: &[u32],
) -> Option<(usize, usize, usize, usize)> {
    let mut day_index = days.len() - 1;
    let mut hour_index = hours.len() - 1;
    let mut minuter_index = minuters.len() - 1;
    let mut second_index = seconds.len() - 1;
    let mut foud = false;
    while day_index > 0 {
        if days[day_index] < day {
            foud = true;
            break;
        } else if days[day_index] == day {
            while hour_index > 0 {
                if hours[hour_index] < hour {
                    foud = true;
                    break;
                } else if hours[hour_index] == hour {
                    while minuter_index > 0 {
                        if minuters[minuter_index] < minuter {
                            foud = true;
                            break;
                        } else if minuters[minuter_index] == hour {
                            while second_index > 0 {
                                if seconds[minuter_index] <= second {
                                    foud = true;
                                    break;
                                }
                                second_index -= 1;
                            }
                        }
                        minuter_index -= 1;
                    }
                }
                hour_index -= 1;
            }
        }
        day_index -= 1;
    }
    if foud {
        Some((day_index, hour_index, minuter_index, second_index))
    } else {
        None
    }
}

fn get_bigger_index(
    day: u32,
    hour: u32,
    minuter: u32,
    second: u32,
    days: &[u32],
    hours: &[u32],
    minuters: &[u32],
    seconds: &[u32],
) -> Option<(usize, usize, usize, usize)> {
    let mut day_index = 0;
    let mut hour_index = 0;
    let mut minuter_index = 0;
    let mut second_index = 0;
    let mut foud = false;
    while day_index < days.len() {
        if days[day_index] > day {
            foud = true;
            break;
        } else if days[day_index] == day {
            while hour_index < hours.len() {
                if hours[hour_index] > hour {
                    foud = true;
                    break;
                } else if hours[hour_index] == hour {
                    while minuter_index < minuters.len() {
                        if minuters[minuter_index] > minuter {
                            foud = true;
                            break;
                        } else if minuters[minuter_index] == hour {
                            while second_index < seconds.len() {
                                if seconds[minuter_index] > second {
                                    foud = true;
                                    break;
                                }
                                second_index += 1;
                            }
                        }
                        minuter_index += 1;
                    }
                }
                hour_index += 1;
            }
        }
        day_index += 1;
    }
    if foud {
        Some((day_index, hour_index, minuter_index, second_index))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // use crate::data::Second::*;
    use crate::traits::Computer;
    use crate::*;

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
    fn test_time_unit_second() {
        let conf = Seconds::default_array(&[S5, S30, S55]);
        {
            let mut unit = TimeUnit::new(S10, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), Some(S30));
            assert_eq!(unit.min_val(), S5);
            unit.val_mut(S30);
            assert_eq!(unit.val, 30);
        }
        {
            let unit = TimeUnit::new(S30, conf.clone());
            assert!(unit.is_match());
            assert_eq!(unit.next_val(), Some(S55));
            assert_eq!(unit.min_val(), S5);
        }
        {
            let unit = TimeUnit::new(S45, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), Some(S55));
            assert_eq!(unit.min_val(), S5);
        }
        {
            let unit = TimeUnit::new(S55, conf.clone());
            assert!(unit.is_match());
            assert_eq!(unit.next_val(), None);
            assert_eq!(unit.min_val(), S5);
        }
        {
            let unit = TimeUnit::new(S57, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), None);
            assert_eq!(unit.min_val(), S5);
        }
    }

    // #[test]
    // fn test_option_min() {
    //     assert_eq!(option_min(Some(1), Some(2)), Some(1));
    //     assert_eq!(option_min(Some(1), None), Some(1));
    //     assert_eq!(option_min(None, Some(2)), Some(2));
    //     assert_eq!(option_min(Some(1), Some(1)), Some(1));
    //     assert_eq!(option_min(Some(2), Some(1)), Some(1));
    //     assert_eq!(option_min::<u32>(None, None), None);
    // }
}
