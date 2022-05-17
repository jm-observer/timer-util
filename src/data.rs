use time::{OffsetDateTime, Weekday};
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct DateTime {
    pub(crate) date: time::Date,
    pub(crate) month_day: InnerMonthDay,
    pub(crate) week_day: InnerWeekDay,
    pub(crate) hour: InnerHour,
    pub(crate) minuter: InnerMinuter,
    pub(crate) second: InnerSecond,
}

#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct InnerWeekDay(pub(crate) u8);
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct InnerMonthDay(pub(crate) u32);
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct InnerHour(pub(crate) u32);
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct InnerMinuter(pub(crate) u64);
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct InnerSecond(pub(crate) u64);

impl DateTime {
    #[allow(dead_code)]
    pub(crate) fn default() -> anyhow::Result<Self> {
        Ok(OffsetDateTime::now_local()?.into())
    }
}
impl From<OffsetDateTime> for DateTime {
    fn from(tmp: OffsetDateTime) -> Self {
        let month_day = InnerMonthDay(tmp.clone().date().day() as u32);
        let week_day: InnerWeekDay = tmp.clone().date().weekday().into();
        let date = tmp.clone().date();
        let time = tmp.time();
        let hour = InnerHour(time.clone().hour() as u32);
        let minuter = InnerMinuter(time.clone().minute() as u64);
        let second = InnerSecond(time.second() as u64);
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

pub trait AsData<Ty>: Copy {
    fn as_data(self) -> Ty;
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

impl AsData<u8> for InnerWeekDay {
    fn as_data(self) -> u8 {
        self.0
    }
}
impl AsData<u32> for InnerMonthDay {
    fn as_data(self) -> u32 {
        self.0
    }
}
impl AsData<u32> for InnerHour {
    fn as_data(self) -> u32 {
        self.0
    }
}
impl AsData<u64> for InnerMinuter {
    fn as_data(self) -> u64 {
        self.0
    }
}
impl AsData<u64> for InnerSecond {
    fn as_data(self) -> u64 {
        self.0
    }
}

impl<T, A: AsData<T>> AsData<T> for &A {
    fn as_data(self) -> T {
        (*self).as_data()
    }
}

impl From<time::Weekday> for InnerWeekDay {
    fn from(day: Weekday) -> Self {
        match day {
            Weekday::Monday => InnerWeekDay(1),
            Weekday::Tuesday => InnerWeekDay(2),
            Weekday::Wednesday => InnerWeekDay(3),
            Weekday::Thursday => InnerWeekDay(4),
            Weekday::Friday => InnerWeekDay(5),
            Weekday::Saturday => InnerWeekDay(6),
            Weekday::Sunday => InnerWeekDay(7),
        }
    }
}
impl From<time::Weekday> for WeekDay {
    fn from(day: Weekday) -> Self {
        match day {
            Weekday::Monday => Self::W1,
            Weekday::Tuesday => Self::W2,
            Weekday::Wednesday => Self::W3,
            Weekday::Thursday => Self::W4,
            Weekday::Friday => Self::W5,
            Weekday::Saturday => Self::W6,
            Weekday::Sunday => Self::W7,
        }
    }
}
