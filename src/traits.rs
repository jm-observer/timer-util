use anyhow::{bail, Result};
use std::fmt::Display;
use std::ops::{Add, AddAssign, BitAnd, BitOr, BitOrAssign, Bound, RangeBounds, Shl, Sub};

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

pub trait Operator: Sized {
    /// 最小值：比如星期配置，则最小为星期1，即为1
    const MIN: Self::ValTy;
    /// 最大值：比如星期配置，则最大为星期日，即为7
    const MAX: Self::ValTy;
    /// 单位值：好像全为1
    const ONE: Self::ValTy;
    /// 0值：即全不选的值，比如星期7天都不选，则为二进制0000 0000
    const ZERO: Self::ValTy;
    /// 满值：即全选的值，比如星期7天全选，则为二进制1111 1110
    const DEFAULT_MAX: Self::ValTy;
    type ValTy: BitOr<Output = Self::ValTy>
        + Shl<Output = Self::ValTy>
        + Copy
        + BitOrAssign
        + Add<Output = Self::ValTy>
        + Sub<Output = Self::ValTy>
        + PartialOrd
        + AddAssign
        + BitAnd<Output = Self::ValTy>
        + Display;

    type DataTy: AsData<Self::ValTy> + Copy + Clone;

    fn _default() -> Self;
    #[inline]
    fn default_value(val: Self::DataTy) -> Self {
        let ins = Self::_default();
        ins.add(val)
    }
    #[inline]
    fn default_range(range: impl RangeBounds<Self::DataTy>) -> Result<Self> {
        let ins = Self::_default();
        ins.add_range(range)
    }
    #[inline]
    fn default_all() -> Self {
        let mut ins = Self::_default();
        ins._val_mut(Self::DEFAULT_MAX);
        ins
    }
    #[inline]
    fn default_all_by_max(max: Self::DataTy) -> Self {
        let mut ins = Self::_default();
        let mut val = ins._val();
        let mut index = Self::MIN;
        while index <= max.as_data() {
            val |= Self::ONE << index.clone();
            index += Self::ONE;
        }
        ins._val_mut(val);
        ins
    }
    fn default_array(vals: &[Self::DataTy]) -> Self {
        let ins = Self::_default();
        ins.add_array(vals)
    }
    fn add_array(mut self, vals: &[Self::DataTy]) -> Self {
        let mut val = self._val();
        for i in vals {
            val |= Self::ONE << i.as_data();
        }
        self._val_mut(val);
        self
    }
    fn add(mut self, index: Self::DataTy) -> Self {
        let index = index.as_data();
        self._val_mut(self._val() | (Self::ONE << index));
        self
    }
    fn add_range(mut self, range: impl RangeBounds<Self::DataTy>) -> Result<Self> {
        let mut first = match range.start_bound() {
            Bound::Unbounded => Self::MIN,
            Bound::Included(first) => first.as_data(),
            Bound::Excluded(first) => first.as_data() + Self::ONE,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => Self::MAX,
            Bound::Included(end) => end.as_data(),
            Bound::Excluded(end) => end.as_data() - Self::ONE,
        };
        if first > end {
            bail!("error:{} > {}", first, end);
        }
        let mut val = self._val();
        while first <= end {
            val |= Self::ONE << first;
            first += Self::ONE;
        }
        self._val_mut(val);
        Ok(self)
    }

    fn merge(&self, other: &Self) -> Self {
        let mut new = Self::_default();
        new._val_mut(self._val() | other._val());
        new
    }
    fn intersection(&self, other: &Self) -> Self {
        let mut new = Self::_default();
        new._val_mut(self._val() & other._val());
        new
    }

    fn to_vec(&self) -> Vec<Self::ValTy> {
        let mut res = Vec::new();
        let val = self._val();
        let mut first = Self::MIN;
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                res.push(first);
            }
            first += Self::ONE;
        }
        res
    }
    fn contain(&self, index: Self::DataTy) -> bool {
        let index = index.as_data();
        let val = self._val();
        val & (Self::ONE << index) > Self::ZERO
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy>;
    /// 取下一个持有值，不包括index
    fn _next(&self, index: Self::DataTy) -> Option<Self::ValTy> {
        let mut first = index.as_data() + Self::ONE;
        let val = self._val();
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                return Some(first);
            }
            first += Self::ONE;
        }
        None
    }
    fn min_val(&self) -> Self::DataTy;
    /// 取最小的持有值
    fn _min_val(&self) -> Self::ValTy {
        let mut first = Self::MIN;
        let val = self._val();
        while first <= Self::MAX {
            if (val & (Self::ONE << first)) > Self::ZERO {
                return first;
            }
            first += Self::ONE;
        }
        unreachable!("it is a bug");
    }
    fn _val(&self) -> Self::ValTy;
    fn _val_mut(&mut self, val: Self::ValTy);
}

pub trait AsData<Ty>: Copy {
    fn as_data(self) -> Ty;
}

pub trait FromData<Ty> {
    fn from_data(val: Ty) -> Self;
}

pub trait TryFromData<Ty>: FromData<Ty> {
    fn try_from_data(val: Ty) -> anyhow::Result<Self>
    where
        Self: Sized;
}
