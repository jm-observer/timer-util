use crate::{AsData, Hours, Minuters, MonthDay, MonthDays, Operator, Seconds, WeekDay, WeekDays};
use chrono::{Datelike, NaiveDate};

struct TimeUnit<T: Operator> {
    // 最大值
    max: T::ValTy,
    // 当前的起始值
    index: T::DataTy,
    // 对应的配置
    conf: T,
    // 最后的值
    val: T::ValTy,
}

struct DayUnit {
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

pub trait Computer {
    const MIN: Self::ValTy;
    type ValTy;
    type DataTy;

    /// 下个循环的第一个符合值
    fn update_to_next_ring(&mut self);

    fn is_match(&self) -> bool;
    // 因为结果可能用来赋值，因此用DataTy，可以避免Result。不包含index
    fn next_val(&self) -> Option<Self::DataTy>;
    fn min_val(&self) -> Self::DataTy;
    fn val_mut(&mut self, val: Self::DataTy);
    fn val(&self) -> Self::ValTy;
}

impl Computer for DayUnit {
    const MIN: Self::ValTy = 1;
    type ValTy = u32;
    type DataTy = MonthDay;

    fn update_to_next_ring(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
        let date = NaiveDate::from_ymd(self.year, self.month, 1);
        let weekday: WeekDay = date.weekday().into();

        self.conf = if let Some(ref weekdays) = self.weekdays {
            let week_monthdays = weekdays.to_month_days(weekday);
            if let Some(ref monthdays) = self.monthdays {
                monthdays.merge(monthdays)
            } else {
                week_monthdays
            }
        } else if let Some(ref monthdays) = self.monthdays {
            monthdays.clone()
        } else {
            unreachable!("")
        };
        self.max = date.day();
        self.day = self.min_val();
        self.val = self.day.as_data();
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
        self.day = val;
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
    fn new(index: T::DataTy, conf: T) -> Self {
        let val = index;
        Self {
            max: T::MAX,
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
//
// fn option_min<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
//     if let Some(a) = a {
//         if let Some(b) = b {
//             if b > a {
//                 Some(a)
//             } else {
//                 Some(b)
//             }
//         } else {
//             Some(a)
//         }
//     } else {
//         b
//     }
// }

struct Composition {
    day: DayUnit,
    hour: TimeUnit<Hours>,
    minuter: TimeUnit<Minuters>,
    second: TimeUnit<Seconds>,
}

impl Composition {
    pub fn next(&mut self) {
        if self.day.is_match() {
            if self.match_hour() {
                return;
            }
        }
        self.next_day();
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
            self.day.val_mut(day);
        } else {
            self.day.update_to_next_ring();
        }
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
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
            let mut unit = TimeUnit::new(S30, conf.clone());
            assert!(unit.is_match());
            assert_eq!(unit.next_val(), Some(S55));
            assert_eq!(unit.min_val(), S5);
        }
        {
            let mut unit = TimeUnit::new(S45, conf.clone());
            assert!(!unit.is_match());
            assert_eq!(unit.next_val(), Some(S55));
            assert_eq!(unit.min_val(), S5);
        }
        {
            let mut unit = TimeUnit::new(S55, conf.clone());
            assert!(unit.is_match());
            assert_eq!(unit.next_val(), None);
            assert_eq!(unit.min_val(), S5);
        }
        {
            let mut unit = TimeUnit::new(S57, conf.clone());
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
