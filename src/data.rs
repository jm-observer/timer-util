use crate::error::TimerError;
use crate::traits::{AsBizData, FromData};
use crate::TryFromData;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday as CWeekday};

/// Defines a time-unit enum with automatic trait implementations.
///
/// Generates:
/// - The enum with `#[repr(u64)]`
/// - `FromData<u64>` implementation
/// - `AsBizData<u64>` implementation
/// - `TryFromData<u64>` implementation
macro_rules! define_time_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $variant:ident = $val:expr ),+ $(,)?
        }
        range: $min:expr => $max:expr,
        type_name: $type_str:expr
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, Eq, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(u64)]
        $vis enum $name {
            $( $variant = $val ),+
        }

        impl FromData<u64> for $name {
            fn from_data(val: u64) -> Self {
                match val {
                    $( $val => Self::$variant, )+
                    _ => panic!(
                        concat!(stringify!($name), "::from_data: value {} is out of range"),
                        val
                    ),
                }
            }
        }

        impl AsBizData<u64> for $name {
            fn as_data(self) -> u64 {
                self as u64
            }
        }

        impl TryFromData<u64> for $name {
            fn try_from_data(val: u64) -> crate::error::Result<Self> {
                if !($min..=$max).contains(&val) {
                    Err(TimerError::ValueOutOfRange {
                        type_name: $type_str,
                        value: val,
                        min: $min,
                        max: $max,
                    })
                } else {
                    Ok(Self::from_data(val))
                }
            }
        }
    };
}

define_time_enum! {
    /// Represents a day of the week (1 = Monday, 7 = Sunday).
    pub enum WeekDay {
        W1 = 1, W2 = 2, W3 = 3, W4 = 4, W5 = 5, W6 = 6, W7 = 7,
    }
    range: 1 => 7,
    type_name: "WeekDay"
}

define_time_enum! {
    /// Represents a day of the month (1-31).
    pub enum MonthDay {
        D1 = 1, D2 = 2, D3 = 3, D4 = 4, D5 = 5, D6 = 6, D7 = 7,
        D8 = 8, D9 = 9, D10 = 10, D11 = 11, D12 = 12, D13 = 13,
        D14 = 14, D15 = 15, D16 = 16, D17 = 17, D18 = 18, D19 = 19,
        D20 = 20, D21 = 21, D22 = 22, D23 = 23, D24 = 24, D25 = 25,
        D26 = 26, D27 = 27, D28 = 28, D29 = 29, D30 = 30, D31 = 31,
    }
    range: 1 => 31,
    type_name: "MonthDay"
}

define_time_enum! {
    /// Represents an hour of the day (0-23).
    pub enum Hour {
        H0 = 0, H1 = 1, H2 = 2, H3 = 3, H4 = 4, H5 = 5,
        H6 = 6, H7 = 7, H8 = 8, H9 = 9, H10 = 10, H11 = 11,
        H12 = 12, H13 = 13, H14 = 14, H15 = 15, H16 = 16, H17 = 17,
        H18 = 18, H19 = 19, H20 = 20, H21 = 21, H22 = 22, H23 = 23,
    }
    range: 0 => 23,
    type_name: "Hour"
}

define_time_enum! {
    /// Represents a minute within an hour (0-59).
    pub enum Minute {
        M0 = 0, M1 = 1, M2 = 2, M3 = 3, M4 = 4, M5 = 5,
        M6 = 6, M7 = 7, M8 = 8, M9 = 9, M10 = 10, M11 = 11,
        M12 = 12, M13 = 13, M14 = 14, M15 = 15, M16 = 16, M17 = 17,
        M18 = 18, M19 = 19, M20 = 20, M21 = 21, M22 = 22, M23 = 23,
        M24 = 24, M25 = 25, M26 = 26, M27 = 27, M28 = 28, M29 = 29,
        M30 = 30, M31 = 31, M32 = 32, M33 = 33, M34 = 34, M35 = 35,
        M36 = 36, M37 = 37, M38 = 38, M39 = 39, M40 = 40, M41 = 41,
        M42 = 42, M43 = 43, M44 = 44, M45 = 45, M46 = 46, M47 = 47,
        M48 = 48, M49 = 49, M50 = 50, M51 = 51, M52 = 52, M53 = 53,
        M54 = 54, M55 = 55, M56 = 56, M57 = 57, M58 = 58, M59 = 59,
    }
    range: 0 => 59,
    type_name: "Minute"
}

define_time_enum! {
    /// Represents a second within a minute (0-59).
    pub enum Second {
        S0 = 0, S1 = 1, S2 = 2, S3 = 3, S4 = 4, S5 = 5,
        S6 = 6, S7 = 7, S8 = 8, S9 = 9, S10 = 10, S11 = 11,
        S12 = 12, S13 = 13, S14 = 14, S15 = 15, S16 = 16, S17 = 17,
        S18 = 18, S19 = 19, S20 = 20, S21 = 21, S22 = 22, S23 = 23,
        S24 = 24, S25 = 25, S26 = 26, S27 = 27, S28 = 28, S29 = 29,
        S30 = 30, S31 = 31, S32 = 32, S33 = 33, S34 = 34, S35 = 35,
        S36 = 36, S37 = 37, S38 = 38, S39 = 39, S40 = 40, S41 = 41,
        S42 = 42, S43 = 43, S44 = 44, S45 = 45, S46 = 46, S47 = 47,
        S48 = 48, S49 = 49, S50 = 50, S51 = 51, S52 = 52, S53 = 53,
        S54 = 54, S55 = 55, S56 = 56, S57 = 57, S58 = 58, S59 = 59,
    }
    range: 0 => 59,
    type_name: "Second"
}

/// Internal date-time representation combining date and individual time components.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DateTime {
    pub(crate) date: NaiveDate,
    pub(crate) month_day: MonthDay,
    pub(crate) week_day: WeekDay,
    pub(crate) hour: Hour,
    pub(crate) minute: Minute,
    pub(crate) second: Second,
}

impl DateTime {
    #[allow(dead_code)]
    pub(crate) fn default() -> Self {
        let now = Local::now().naive_local();
        now.into()
    }
}

impl From<NaiveDateTime> for DateTime {
    fn from(tmp: NaiveDateTime) -> Self {
        let date = tmp.date();
        let time = tmp.time();

        let month_day = MonthDay::from_data(date.day() as u64);
        let week_day: WeekDay = date.weekday().into();
        let hour = Hour::from_data(time.hour() as u64);
        let minute = Minute::from_data(time.minute() as u64);
        let second = Second::from_data(time.second() as u64);
        Self {
            date,
            month_day,
            week_day,
            hour,
            minute,
            second,
        }
    }
}

impl From<DateTime> for NaiveDateTime {
    fn from(tmp: DateTime) -> Self {
        NaiveDateTime::new(
            tmp.date,
            NaiveTime::from_hms(
                tmp.hour.as_data() as u32,
                tmp.minute.as_data() as u32,
                tmp.second.as_data() as u32,
            ),
        )
    }
}

impl<T, A: AsBizData<T>> AsBizData<T> for &A {
    fn as_data(self) -> T {
        (*self).as_data()
    }
}

impl From<CWeekday> for WeekDay {
    fn from(day: CWeekday) -> Self {
        match day {
            CWeekday::Mon => Self::W1,
            CWeekday::Tue => Self::W2,
            CWeekday::Wed => Self::W3,
            CWeekday::Thu => Self::W4,
            CWeekday::Fri => Self::W5,
            CWeekday::Sat => Self::W6,
            CWeekday::Sun => Self::W7,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::Hour::*;
    use crate::data::Minute::*;
    use crate::data::Second::*;
    use crate::data::WeekDay::*;

    #[test]
    fn test_hour_from_data() {
        assert_eq!(Hour::from_data(0), H0);
        assert_eq!(Hour::from_data(23), H23);
    }

    #[test]
    #[should_panic]
    fn test_hour_from_data_out_of_range() {
        Hour::from_data(24);
    }

    #[test]
    #[should_panic]
    fn test_month_day_from_data_zero() {
        MonthDay::from_data(0);
    }

    #[test]
    #[should_panic]
    fn test_week_day_from_data_zero() {
        WeekDay::from_data(0);
    }

    #[test]
    #[should_panic]
    fn test_week_day_from_data_overflow() {
        WeekDay::from_data(8);
    }

    #[test]
    fn test_try_from_data_valid() {
        assert!(MonthDay::try_from_data(1).is_ok());
        assert!(MonthDay::try_from_data(31).is_ok());
    }

    #[test]
    fn test_try_from_data_invalid() {
        assert!(MonthDay::try_from_data(0).is_err());
        assert!(MonthDay::try_from_data(32).is_err());
        assert!(WeekDay::try_from_data(0).is_err());
        assert!(WeekDay::try_from_data(8).is_err());
        assert!(Hour::try_from_data(24).is_err());
        assert!(Minute::try_from_data(60).is_err());
        assert!(Second::try_from_data(60).is_err());
    }

    #[test]
    fn test_as_biz_data_roundtrip() {
        for i in 0..24 {
            assert_eq!(Hour::from_data(i).as_data(), i);
        }
        for i in 1..=31 {
            assert_eq!(MonthDay::from_data(i).as_data(), i);
        }
        for i in 1..=7 {
            assert_eq!(WeekDay::from_data(i).as_data(), i);
        }
        for i in 0..60 {
            assert_eq!(Minute::from_data(i).as_data(), i);
            assert_eq!(Second::from_data(i).as_data(), i);
        }
    }

    #[test]
    fn test_datetime_conversion_roundtrip() {
        let ndt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        );
        let dt: DateTime = ndt.into();
        let back: NaiveDateTime = dt.into();
        assert_eq!(ndt, back);
    }

    #[test]
    fn test_weekday_from_chrono() {
        use chrono::Weekday;
        assert_eq!(WeekDay::from(Weekday::Mon), W1);
        assert_eq!(WeekDay::from(Weekday::Tue), W2);
        assert_eq!(WeekDay::from(Weekday::Wed), W3);
        assert_eq!(WeekDay::from(Weekday::Thu), W4);
        assert_eq!(WeekDay::from(Weekday::Fri), W5);
        assert_eq!(WeekDay::from(Weekday::Sat), W6);
        assert_eq!(WeekDay::from(Weekday::Sun), W7);
    }

    #[test]
    fn test_minute_from_data_boundaries() {
        assert_eq!(Minute::from_data(0), M0);
        assert_eq!(Minute::from_data(59), M59);
    }

    #[test]
    #[should_panic]
    fn test_minute_from_data_out_of_range() {
        Minute::from_data(60);
    }

    #[test]
    fn test_second_from_data_boundaries() {
        assert_eq!(Second::from_data(0), S0);
        assert_eq!(Second::from_data(59), S59);
    }

    #[test]
    #[should_panic]
    fn test_second_from_data_out_of_range() {
        Second::from_data(60);
    }
}
