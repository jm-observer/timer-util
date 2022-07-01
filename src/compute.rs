use crate::conf::{Hours, Minuters, MonthDays, Seconds, WeekDays};
use crate::data::{MonthDay, WeekDay};
use crate::traits::{AsData, Computer, Operator};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
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

fn merge_days_conf(
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::Second::*;
    use crate::traits::Computer;

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
