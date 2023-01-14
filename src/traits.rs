use anyhow::{bail, Result};
use std::ops::{Bound, RangeBounds};

pub trait Computer {
    const MIN: u64;
    type DataTy;

    /// 下个循环的第一个符合值
    fn update_to_next_ring(&mut self);

    fn is_match(&self) -> bool;
    // 因为结果可能用来赋值，因此用DataTy，可以避免Result。不包含index
    fn next_val(&self) -> Option<Self::DataTy>;
    fn min_val(&self) -> Self::DataTy;
    fn val_mut(&mut self, val: Self::DataTy);
    fn val(&self) -> u64;
}
/// 配置项的操作trait
pub trait ConfigOperator: Sized {
    /// 最小值：比如星期配置，则最小为星期1，即为1
    const MIN: u64;
    /// 最大值：比如星期配置，则最大为星期日，即为7
    const MAX: u64;
    /// 满值：即全选的值，比如星期7天全选，则为二进制1111 1110
    const DEFAULT_MAX: u64;

    type DataTy: AsData<u64> + Copy + Clone;

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
            val |= 1 << index.clone();
            index += 1;
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
            val |= 1 << i.as_data();
        }
        self._val_mut(val);
        self
    }
    fn add(mut self, index: Self::DataTy) -> Self {
        let index = index.as_data();
        self._val_mut(self._val() | (1 << index));
        self
    }
    fn add_range(mut self, range: impl RangeBounds<Self::DataTy>) -> Result<Self> {
        let mut first = match range.start_bound() {
            Bound::Unbounded => Self::MIN,
            Bound::Included(first) => first.as_data(),
            Bound::Excluded(first) => first.as_data() + 1,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => Self::MAX,
            Bound::Included(end) => end.as_data(),
            Bound::Excluded(end) => end.as_data() - 1,
        };
        if first > end {
            bail!("error:{} > {}", first, end);
        }
        let mut val = self._val();
        while first <= end {
            val |= 1 << first;
            first += 1;
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

    fn to_vec(&self) -> Vec<u64> {
        let mut res = Vec::new();
        let val = self._val();
        let mut first = Self::MIN;
        while first <= Self::MAX {
            if (val & (1 << first)) > 0 {
                res.push(first);
            }
            first += 1;
        }
        res
    }
    fn contain(&self, index: Self::DataTy) -> bool {
        let index = index.as_data();
        let val = self._val();
        val & (1 << index) > 0
    }
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy>;
    /// 取下一个持有值，不包括index
    fn _next(&self, index: Self::DataTy) -> Option<u64> {
        let mut first = index.as_data() + 1;
        let val = self._val();
        while first <= Self::MAX {
            if (val & (1 << first)) > 0 {
                return Some(first);
            }
            first += 1;
        }
        None
    }
    fn min_val(&self) -> Self::DataTy;
    /// 取最小的持有值
    fn _min_val(&self) -> u64 {
        let mut first = Self::MIN;
        let val = self._val();
        while first <= Self::MAX {
            if (val & (1 << first)) > 0 {
                return first;
            }
            first += 1;
        }
        unreachable!("it is a bug");
    }
    fn _val(&self) -> u64;
    fn _val_mut(&mut self, val: u64);
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
