use crate::conf::{Days, Hours, Minutes, MonthDays, Seconds};
use crate::data::{Hour, Minute, MonthDay, Second, WeekDay};
use crate::traits::{AsBizData, Computer, ConfigOperator, FromData};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use log::debug;

#[derive(Debug)]
pub struct TimeUnit<T: ConfigOperator> {
    index: T::DataTy,
    conf: T,
    val: u64,
}
#[derive(Debug)]
pub struct DayUnit {
    year: i32,
    month: u32,
    days: Days,
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
        self.conf
            .next(self.day)
            .filter(|next| next.as_data() <= self.max as u64)
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
            day,
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
    minute: TimeUnit<Minutes>,
    second: TimeUnit<Seconds>,
}

impl Composition {
    pub fn from(
        now: NaiveDateTime,
        days: Days,
        hours: Hours,
        min: Minutes,
        seconds: Seconds,
    ) -> Self {
        let year = now.year();
        let month = now.month();
        let day = MonthDay::from_data(now.day() as u64);
        let first_week_day: WeekDay = NaiveDate::from_ymd(year, month, 1).weekday().into();
        let max = next_month(year, month).pred().day();
        let day_unit = DayUnit::new(year, month, days, day, first_week_day, max);
        let hour: TimeUnit<Hours> = TimeUnit::new(Hour::from_data(now.hour() as u64), hours);
        let minute = TimeUnit::new(Minute::from_data(now.minute() as u64), min);
        let second = TimeUnit::new(Second::from_data(now.second() as u64), seconds);
        Composition::new(day_unit, hour, minute, second)
    }
    pub fn new(
        day: DayUnit,
        hour: TimeUnit<Hours>,
        minute: TimeUnit<Minutes>,
        second: TimeUnit<Seconds>,
    ) -> Self {
        Self {
            day,
            hour,
            minute,
            second,
        }
    }

    pub fn next(&mut self) -> NaiveDateTime {
        loop {
            if self.day.is_match() && self.match_hour() {
                break;
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
                self.minute.val as u32,
                self.second.val as u32,
            ),
        )
    }
    fn match_hour(&mut self) -> bool {
        if self.hour.is_match() && self.match_minute() {
            return true;
        }
        if let Some(hour) = self.hour.next_val() {
            self.hour.val_mut(hour);
            self.minute_update_to_next_ring();
            true
        } else {
            false
        }
    }
    fn match_minute(&mut self) -> bool {
        if self.minute.is_match() && self.match_second() {
            return true;
        }
        if let Some(minute) = self.minute.next_val() {
            self.minute.val_mut(minute);
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
        if let Some(second) = self.second.next_val() {
            self.second.val_mut(second);
            true
        } else {
            false
        }
    }

    fn next_day(&mut self) {
        if let Some(day) = self.day.next_val() {
            debug!("day_unit: {:?}, next_day: {:?}", self.day, day);
            self.day.val_mut(day);
            self.day.day = day;
        } else {
            self.day.update_to_next_ring();
        }
        debug!("day_unit: {:?}", self.day);
        self.hour_update_to_next_ring();
    }
    fn hour_update_to_next_ring(&mut self) {
        self.hour.update_to_next_ring();
        self.minute_update_to_next_ring();
    }
    fn minute_update_to_next_ring(&mut self) {
        self.minute.update_to_next_ring();
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
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

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

    #[test]
    fn test_time_unit_hour() {
        let conf = Hours::default_array(&[H0, H8, H16]);
        {
            let unit = TimeUnit::new(H0, conf.clone());
            assert!(unit.is_match());
            assert_eq!(unit.next_val(), Some(H8));
        }
        {
            let unit = TimeUnit::new(H10, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), Some(H16));
        }
        {
            let unit = TimeUnit::new(H20, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), None);
        }
    }

    #[test]
    fn test_time_unit_minute() {
        let conf = Minutes::default_array(&[M0, M15, M30, M45]);
        {
            let unit = TimeUnit::new(M0, conf.clone());
            assert!(unit.is_match());
            assert_eq!(unit.next_val(), Some(M15));
        }
        {
            let unit = TimeUnit::new(M10, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), Some(M15));
        }
        {
            let unit = TimeUnit::new(M50, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), None);
        }
    }

    #[test]
    fn test_leap_year_feb29() {
        let conf = configure_monthday(MonthDays::default_value(D29))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 2, 28).unwrap(),
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        );
        let next = conf.next_with_time(start);
        assert_eq!(
            next,
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        );
    }

    #[test]
    fn test_non_leap_year_skip_feb29() {
        let conf = configure_monthday(MonthDays::default_value(D29))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2023, 1, 30).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let next = conf.next_with_time(start);
        // 2023 is not a leap year, Feb has no 29th, should skip to March 29
        assert_eq!(
            next,
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2023, 3, 29).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        );
    }
}
