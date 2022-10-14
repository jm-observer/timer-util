use crate::traits::{AsData, FromData};
use crate::TryFromData;
use anyhow::{bail, Result};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday as CWeekday};

// use time::{OffsetDateTime, Weekday};
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DateTime {
    pub(crate) date: NaiveDate,
    pub(crate) month_day: MonthDay,
    pub(crate) week_day: WeekDay,
    pub(crate) hour: Hour,
    pub(crate) minuter: Minuter,
    pub(crate) second: Second,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WeekDay {
    W1 = 1,
    W2,
    W3,
    W4,
    W5,
    W6,
    W7,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MonthDay {
    D1 = 1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
    D11,
    D12,
    D13,
    D14,
    D15,
    D16,
    D17,
    D18,
    D19,
    D20,
    D21,
    D22,
    D23,
    D24,
    D25,
    D26,
    D27,
    D28,
    D29,
    D30,
    D31,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Hour {
    H0 = 0,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    H7,
    H8,
    H9,
    H10,
    H11,
    H12,
    H13,
    H14,
    H15,
    H16,
    H17,
    H18,
    H19,
    H20,
    H21,
    H22,
    H23,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum Minuter {
    M0 = 0,
    M1,
    M2,
    M3,
    M4,
    M5,
    M6,
    M7,
    M8,
    M9,
    M10,
    M11,
    M12,
    M13,
    M14,
    M15,
    M16,
    M17,
    M18,
    M19,
    M20,
    M21,
    M22,
    M23,
    M24,
    M25,
    M26,
    M27,
    M28,
    M29,
    M30,
    M31,
    M32,
    M33,
    M34,
    M35,
    M36,
    M37,
    M38,
    M39,
    M40,
    M41,
    M42,
    M43,
    M44,
    M45,
    M46,
    M47,
    M48,
    M49,
    M50,
    M51,
    M52,
    M53,
    M54,
    M55,
    M56,
    M57,
    M58,
    M59,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum Second {
    S0 = 0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    S12,
    S13,
    S14,
    S15,
    S16,
    S17,
    S18,
    S19,
    S20,
    S21,
    S22,
    S23,
    S24,
    S25,
    S26,
    S27,
    S28,
    S29,
    S30,
    S31,
    S32,
    S33,
    S34,
    S35,
    S36,
    S37,
    S38,
    S39,
    S40,
    S41,
    S42,
    S43,
    S44,
    S45,
    S46,
    S47,
    S48,
    S49,
    S50,
    S51,
    S52,
    S53,
    S54,
    S55,
    S56,
    S57,
    S58,
    S59,
}

//
// #[derive(Copy, Clone, Eq, PartialEq, Debug)]
// pub(crate) struct InnerWeekDay(pub(crate) u8);
// #[derive(Copy, Clone, Eq, PartialEq, Debug)]
// pub(crate) struct InnerMonthDay(pub(crate) u32);
// #[derive(Copy, Clone, Eq, PartialEq, Debug)]
// pub(crate) struct InnerHour(pub(crate) u32);
// #[derive(Copy, Clone, Eq, PartialEq, Debug)]
// pub(crate) struct InnerMinuter(pub(crate) u64);
// #[derive(Copy, Clone, Eq, PartialEq, Debug)]
// pub(crate) struct InnerSecond(pub(crate) u64);

impl DateTime {
    #[allow(dead_code)]
    pub(crate) fn default() -> Result<Self> {
        let now = Local::now().naive_local();
        Ok(now.into())
    }
}

impl From<NaiveDateTime> for DateTime {
    fn from(tmp: NaiveDateTime) -> Self {
        let date = tmp.date();
        let time = tmp.time();

        let month_day = MonthDay::from_data(date.day());
        let week_day: WeekDay = date.weekday().into();
        let hour = Hour::from_data(time.hour());
        let minuter = Minuter::from_data(time.minute() as u64);
        let second = Second::from_data(time.second() as u64);
        Self {
            date,
            month_day,
            week_day,
            hour,
            minuter,
            second,
        }
    }
}
impl From<DateTime> for NaiveDateTime {
    fn from(tmp: DateTime) -> Self {
        NaiveDateTime::new(
            tmp.date,
            NaiveTime::from_hms(
                tmp.hour.as_data(),
                tmp.minuter.as_data() as u32,
                tmp.second.as_data() as u32,
            ),
        )
    }
}

impl AsData<u8> for WeekDay {
    fn as_data(self) -> u8 {
        self as u8
    }
}
impl AsData<u32> for MonthDay {
    fn as_data(self) -> u32 {
        self as u32
    }
}
impl AsData<u32> for Hour {
    fn as_data(self) -> u32 {
        self as u32
    }
}
impl AsData<u64> for Minuter {
    fn as_data(self) -> u64 {
        self as u64
    }
}
impl AsData<u64> for Second {
    fn as_data(self) -> u64 {
        self as u64
    }
}

impl<T, A: AsData<T>> AsData<T> for &A {
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

impl FromData<u32> for MonthDay {
    fn from_data(val: u32) -> Self {
        assert!(val < 32 && val != 0);
        match val {
            1 => Self::D1,
            2 => Self::D2,
            3 => Self::D3,
            4 => Self::D4,
            5 => Self::D5,
            6 => Self::D6,
            7 => Self::D7,
            8 => Self::D8,
            9 => Self::D9,
            10 => Self::D10,
            11 => Self::D11,
            12 => Self::D12,
            13 => Self::D13,
            14 => Self::D14,
            15 => Self::D15,
            16 => Self::D16,
            17 => Self::D17,
            18 => Self::D18,
            19 => Self::D19,
            20 => Self::D20,
            21 => Self::D21,
            22 => Self::D22,
            23 => Self::D23,
            24 => Self::D24,
            25 => Self::D25,
            26 => Self::D26,
            27 => Self::D27,
            28 => Self::D28,
            29 => Self::D29,
            30 => Self::D30,
            31 => Self::D31,
            _ => unreachable!("bug!"),
        }
    }
}
impl FromData<u8> for WeekDay {
    fn from_data(val: u8) -> Self {
        assert!(val < 8 && val != 0);
        match val {
            1 => Self::W1,
            2 => Self::W2,
            3 => Self::W3,
            4 => Self::W4,
            5 => Self::W5,
            6 => Self::W6,
            7 => Self::W7,
            _ => unreachable!("bug!"),
        }
    }
}

impl FromData<u32> for Hour {
    fn from_data(val: u32) -> Self {
        assert!(val < 24);
        match val {
            0 => Self::H0,
            1 => Self::H1,
            2 => Self::H2,
            3 => Self::H3,
            4 => Self::H4,
            5 => Self::H5,
            6 => Self::H6,
            7 => Self::H7,
            8 => Self::H8,
            9 => Self::H9,
            10 => Self::H10,
            11 => Self::H11,
            12 => Self::H12,
            13 => Self::H13,
            14 => Self::H14,
            15 => Self::H15,
            16 => Self::H16,
            17 => Self::H17,
            18 => Self::H18,
            19 => Self::H19,
            20 => Self::H20,
            21 => Self::H21,
            22 => Self::H22,
            23 => Self::H23,
            _ => unreachable!("bug!"),
        }
    }
}

impl FromData<u64> for Minuter {
    fn from_data(val: u64) -> Self {
        assert!(val < 60);
        match val {
            0 => Self::M0,
            1 => Self::M1,
            2 => Self::M2,
            3 => Self::M3,
            4 => Self::M4,
            5 => Self::M5,
            6 => Self::M6,
            7 => Self::M7,
            8 => Self::M8,
            9 => Self::M9,
            10 => Self::M10,
            11 => Self::M11,
            12 => Self::M12,
            13 => Self::M13,
            14 => Self::M14,
            15 => Self::M15,
            16 => Self::M16,
            17 => Self::M17,
            18 => Self::M18,
            19 => Self::M19,
            20 => Self::M20,
            21 => Self::M21,
            22 => Self::M22,
            23 => Self::M23,
            24 => Self::M24,
            25 => Self::M25,
            26 => Self::M26,
            27 => Self::M27,
            28 => Self::M28,
            29 => Self::M29,
            30 => Self::M30,
            31 => Self::M31,
            32 => Self::M32,
            33 => Self::M33,
            34 => Self::M34,
            35 => Self::M35,
            36 => Self::M36,
            37 => Self::M37,
            38 => Self::M38,
            39 => Self::M39,
            40 => Self::M40,
            41 => Self::M41,
            42 => Self::M42,
            43 => Self::M43,
            44 => Self::M44,
            45 => Self::M45,
            46 => Self::M46,
            47 => Self::M47,
            48 => Self::M48,
            49 => Self::M49,
            50 => Self::M50,
            51 => Self::M51,
            52 => Self::M52,
            53 => Self::M53,
            54 => Self::M54,
            55 => Self::M55,
            56 => Self::M56,
            57 => Self::M57,
            58 => Self::M58,
            59 => Self::M59,
            _ => unreachable!("bug!"),
        }
    }
}

impl FromData<u64> for Second {
    fn from_data(val: u64) -> Self {
        assert!(val < 60);
        match val {
            0 => Self::S0,
            1 => Self::S1,
            2 => Self::S2,
            3 => Self::S3,
            4 => Self::S4,
            5 => Self::S5,
            6 => Self::S6,
            7 => Self::S7,
            8 => Self::S8,
            9 => Self::S9,
            10 => Self::S10,
            11 => Self::S11,
            12 => Self::S12,
            13 => Self::S13,
            14 => Self::S14,
            15 => Self::S15,
            16 => Self::S16,
            17 => Self::S17,
            18 => Self::S18,
            19 => Self::S19,
            20 => Self::S20,
            21 => Self::S21,
            22 => Self::S22,
            23 => Self::S23,
            24 => Self::S24,
            25 => Self::S25,
            26 => Self::S26,
            27 => Self::S27,
            28 => Self::S28,
            29 => Self::S29,
            30 => Self::S30,
            31 => Self::S31,
            32 => Self::S32,
            33 => Self::S33,
            34 => Self::S34,
            35 => Self::S35,
            36 => Self::S36,
            37 => Self::S37,
            38 => Self::S38,
            39 => Self::S39,
            40 => Self::S40,
            41 => Self::S41,
            42 => Self::S42,
            43 => Self::S43,
            44 => Self::S44,
            45 => Self::S45,
            46 => Self::S46,
            47 => Self::S47,
            48 => Self::S48,
            49 => Self::S49,
            50 => Self::S50,
            51 => Self::S51,
            52 => Self::S52,
            53 => Self::S53,
            54 => Self::S54,
            55 => Self::S55,
            56 => Self::S56,
            57 => Self::S57,
            58 => Self::S58,
            59 => Self::S59,
            _ => unreachable!("bug!"),
        }
    }
}

impl TryFromData<u32> for MonthDay {
    fn try_from_data(val: u32) -> Result<Self> {
        if val == 0 || val > 31 {
            bail!("month day should not be 0 or > 31");
        }
        Ok(MonthDay::from_data(val))
    }
}
impl TryFromData<u8> for WeekDay {
    fn try_from_data(val: u8) -> Result<Self> {
        if val >= 8 || val == 0 {
            bail!("week day should not be 0 or >= 60");
        }
        Ok(WeekDay::from_data(val))
    }
}
impl TryFromData<u32> for Hour {
    fn try_from_data(val: u32) -> Result<Self> {
        if val >= 24 {
            bail!("week day should not >= 24");
        }
        Ok(Hour::from_data(val))
    }
}
impl TryFromData<u64> for Minuter {
    fn try_from_data(val: u64) -> Result<Self> {
        if val >= 60 {
            bail!("minuter should not >= 60");
        }
        Ok(Minuter::from_data(val))
    }
}
impl TryFromData<u64> for Second {
    fn try_from_data(val: u64) -> Result<Self> {
        if val >= 60 {
            bail!("second should not >= 60");
        }
        Ok(Second::from_data(val))
    }
}
