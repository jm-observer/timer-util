use crate::{AsData, MonthDay, MonthDays, Operator, Second, Seconds, WeekDay, WeekDays};

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

pub trait Computer {
    const MIN: Self::ValTy;
    type ValTy;
    type DataTy;

    fn is_match(&self) -> bool;
    // 因为结果可能用来赋值，因此用DataTy，可以避免Result。不包含index
    fn next_val(&self) -> Option<Self::DataTy>;
    fn min_val(&self) -> Self::DataTy;
    fn val_mut(&mut self, val: Self::DataTy);
    fn val(&self) -> Self::ValTy;
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

struct DayUnit {
    // 最大值
    max: u32,
    // 当前的起始值
    index: u32,
    index_week_day: u8,
    // 对应的配置
    month_day_conf: MonthDays,
    week_day_conf: WeekDays,
    // 最后的值
    val: u32,
}

impl Computer for DayUnit {
    const MIN: Self::ValTy = 1;
    type ValTy = u32;
    type DataTy = MonthDay;

    fn is_match(&self) -> bool {
        todo!()
        // self.month_day_conf.contain(self.index.into())
        //     || self.week_day_conf.contain(self.index_week_day.into())
    }

    fn next_val(&self) -> Option<Self::DataTy> {
        todo!()
        // let next_month_day = self.month_day_conf.next(self.index.into()).and_then(|x| {
        //     if x > self.max {
        //         None
        //     } else {
        //         Some(x)
        //     }
        // });
        // let next_week_day = self
        //     .week_day_conf
        //     .next(self.index_week_day.into())
        //     .and_then(|x| Some(self.index + ((x - self.index_week_day) as u32)))
        //     .or_else(|| {
        //         // 日差
        //         let weekday = 7 - self.index_week_day + self.week_day_conf.min_val();
        //         Some(self.index + (weekday as u32))
        //     })
        //     .and_then(|x| {
        //         // 判断是否超过本月末
        //         if x > self.max {
        //             None
        //         } else {
        //             Some(x)
        //         }
        //     });
        // option_min(next_month_day, next_week_day)
    }

    fn min_val(&self) -> Self::DataTy {
        todo!()
    }

    fn val_mut(&mut self, val: Self::DataTy) {
        todo!()
    }

    fn val(&self) -> Self::ValTy {
        todo!()
    }
}

fn option_min<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
    if let Some(a) = a {
        if let Some(b) = b {
            if b > a {
                Some(a)
            } else {
                Some(b)
            }
        } else {
            Some(a)
        }
    } else {
        b
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

    #[test]
    fn test_option_min() {
        assert_eq!(option_min(Some(1), Some(2)), Some(1));
        assert_eq!(option_min(Some(1), None), Some(1));
        assert_eq!(option_min(None, Some(2)), Some(2));
        assert_eq!(option_min(Some(1), Some(1)), Some(1));
        assert_eq!(option_min(Some(2), Some(1)), Some(1));
        assert_eq!(option_min::<u32>(None, None), None);
    }
}
