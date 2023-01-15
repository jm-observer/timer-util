use crate::conf::{Days, Hours, Minuters, MonthDays, Seconds};
use crate::data::{Hour, Minuter, MonthDay, Second, WeekDay};
use crate::traits::{AsBizData, Computer, FromData, ConfigOperator};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use log::debug;

#[derive(Debug)]
pub struct TimeUnit<T: ConfigOperator> {
    // 最大值
    // max: T::ValTy,
    // 当前的起始值
    index: T::DataTy,
    // 对应的配置
    conf: T,
    // 最后的值
    val: u64,
}
#[derive(Debug)]
pub struct DayUnit {
    // 当前的起始值
    year: i32,
    // 最后的值
    month: u32,
    days: Days,
    // monthdays: Option<MonthDays>,
    // weekdays: Option<WeekDays>,
    day: MonthDay,
    max: u32,
    conf: MonthDays,
    val: u32,
}

impl Computer for DayUnit {
    const MIN: u64 = 1;
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
            self.conf = self.days.month_days(weekday);
            self.max = next_month.pred().day();
            self.day = self.conf.min_val();
            if self.day.as_data() as u32 <= self.max {
                self.val = self.day.as_data() as u32;
                break;
            }
        }
    }

    fn is_match(&self) -> bool {
        self.conf.contain(self.day)
    }

    fn next_val(&self) -> Option<Self::DataTy> {
        if let Some(next) = self.conf.next(self.day) {
            if next.as_data() > self.max as u64 {
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
        self.val = val.as_data() as u32;
    }

    fn val(&self) -> u64 {
        self.day.as_data()
    }
}

impl<T: ConfigOperator> Computer for TimeUnit<T> {
    const MIN: u64 = <T as ConfigOperator>::MIN;
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

    fn val(&self) -> u64 {
        self.val
    }
}

impl<T: ConfigOperator> TimeUnit<T> {
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
        days: Days,
        day: MonthDay,
        first_week_day: WeekDay,
        max: u32,
    ) -> Self {
        let conf = days.month_days(first_week_day);
        Self {
            year,
            month,
            days,
            day: day.clone(),
            max,
            conf,
            val: day.as_data() as u32,
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
        days: Days,
        hours: Hours,
        min: Minuters,
        seconds: Seconds,
    ) -> Self {
        let year = now.year();
        let month = now.month();
        let day = MonthDay::from_data(now.day() as u64);
        let first_week_day: WeekDay = NaiveDate::from_ymd(year, month, 1).weekday().into();
        let max = next_month(year, month).pred().day();
        let day_unit = DayUnit::new(year, month, days, day, first_week_day, max);
        let hour: TimeUnit<Hours> = TimeUnit::new(Hour::from_data(now.hour() as u64), hours);
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
                self.hour.val as u32,
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
            self.day.val_mut(day.clone());
            self.day.day = day;
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
#[cfg(test)]
mod test {
    use super::*;
    use crate::traits::Computer;
    use crate::*;

    // #[test]
    // fn test_init_first_week() {
    //     assert_eq!(
    //         WeekArray::init(W3),
    //         WeekArray {
    //             days: [-1, 0, 1, 2, 3, 4, 5],
    //         }
    //     );
    //     assert_eq!(
    //         WeekArray::init(W1),
    //         WeekArray {
    //             days: [1, 2, 3, 4, 5, 6, 7],
    //         }
    //     );
    //     assert_eq!(
    //         WeekArray::init(W7),
    //         WeekArray {
    //             days: [-5, -4, -3, -2, -1, 0, 1],
    //         }
    //     );
    //     {
    //         let next = WeekArray::init(W7).next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [2, 3, 4, 5, 6, 7, 8]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [9, 10, 11, 12, 13, 14, 15]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [16, 17, 18, 19, 20, 21, 22]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [23, 24, 25, 26, 27, 28, 29]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [30, 31, 32, 33, 34, 35, 36]);
    //
    //         let next = next.next();
    //         assert!(next.is_none());
    //     }
    //     {
    //         let next = WeekArray::init(W3).next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [6, 7, 8, 9, 10, 11, 12]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [13, 14, 15, 16, 17, 18, 19]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [20, 21, 22, 23, 24, 25, 26]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [27, 28, 29, 30, 31, 32, 33]);
    //
    //         let next = next.next();
    //         assert!(next.is_none());
    //     }
    //
    //     {
    //         let next = WeekArray::init(W1).next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [8, 9, 10, 11, 12, 13, 14]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [15, 16, 17, 18, 19, 20, 21]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [22, 23, 24, 25, 26, 27, 28]);
    //
    //         let next = next.next();
    //         assert!(next.is_some());
    //         let next = next.unwrap();
    //         assert_eq!(next.days, [29, 30, 31, 32, 33, 34, 35]);
    //
    //         let next = next.next();
    //         assert!(next.is_none());
    //     }
    // }

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

}
