use crate::error::TimerError;
use std::ops::{Bound, RangeBounds};

/// Trait for time-unit computation within the scheduling engine.
///
/// Implemented by [`DayUnit`](crate::compute::DayUnit) and
/// [`TimeUnit<T>`](crate::compute::TimeUnit) to find the next matching value
/// in a scheduling cycle.
pub trait Computer {
    const MIN: u64;
    type DataTy;

    /// Reset to the first matching value for the next cycle.
    fn update_to_next_ring(&mut self);

    /// Check if the current value matches the configuration.
    fn is_match(&self) -> bool;

    /// Return the next matching value after the current one, or `None` if none exists in this cycle.
    fn next_val(&self) -> Option<Self::DataTy>;

    /// Return the minimum configured value.
    fn min_val(&self) -> Self::DataTy;

    /// Set the current value.
    fn val_mut(&mut self, val: Self::DataTy);

    /// Return the current value as a raw `u64`.
    fn val(&self) -> u64;
}

/// Bitset-based configuration operator for time-unit selection.
///
/// Each implementor stores selected values as bits in a `u64`. For example,
/// `Hours` with bit 9 set means hour 9 is selected.
pub trait ConfigOperator: Sized {
    /// The minimum valid value (e.g., 0 for hours, 1 for month days).
    const MIN: u64;
    /// The maximum valid value (e.g., 23 for hours, 31 for month days).
    const MAX: u64;
    /// The bitset value when all valid positions are selected.
    const DEFAULT_MAX: u64;

    /// The associated data type for this configuration (e.g., `Hour`, `Minute`).
    type DataTy: AsBizData<u64> + Copy + Clone;

    /// Create an empty configuration with no values selected.
    fn _default() -> Self;

    /// Create a configuration with a single value selected.
    #[inline]
    fn default_value(val: Self::DataTy) -> Self {
        let ins = Self::_default();
        ins.add(val)
    }

    /// Create a configuration from a range of values.
    #[inline]
    fn default_range(range: impl RangeBounds<Self::DataTy>) -> crate::error::Result<Self> {
        let ins = Self::_default();
        ins.add_range(range)
    }

    /// Create a configuration with all valid values selected.
    #[inline]
    fn default_all() -> Self {
        let mut ins = Self::_default();
        ins._val_mut(Self::DEFAULT_MAX);
        ins
    }

    /// Create a configuration with all values from `MIN` up to `max` selected.
    #[inline]
    fn default_all_by_max(max: Self::DataTy) -> Self {
        let mut ins = Self::_default();
        let mut val = ins._val();
        let mut index = Self::MIN;
        while index <= max.as_data() {
            val |= 1 << index;
            index += 1;
        }
        ins._val_mut(val);
        ins
    }

    /// Create a configuration from an array of values.
    fn default_array(vals: &[Self::DataTy]) -> Self {
        let ins = Self::_default();
        ins.add_array(vals)
    }

    /// Add multiple values to this configuration.
    fn add_array(mut self, vals: &[Self::DataTy]) -> Self {
        let mut val = self._val();
        for i in vals {
            val |= 1 << i.as_data();
        }
        self._val_mut(val);
        self
    }

    /// Add a single value to this configuration.
    fn add(mut self, index: Self::DataTy) -> Self {
        let index = index.as_data();
        self._val_mut(self._val() | (1 << index));
        self
    }

    /// Add a range of values to this configuration.
    fn add_range(mut self, range: impl RangeBounds<Self::DataTy>) -> crate::error::Result<Self> {
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
            return Err(TimerError::InvalidRange { start: first, end });
        }
        let mut val = self._val();
        while first <= end {
            val |= 1 << first;
            first += 1;
        }
        self._val_mut(val);
        Ok(self)
    }

    /// Return the union of two configurations.
    fn merge(&self, other: &Self) -> Self {
        let mut new = Self::_default();
        new._val_mut(self._val() | other._val());
        new
    }

    /// Return the intersection of two configurations.
    fn intersection(&self, other: &Self) -> Self {
        let mut new = Self::_default();
        new._val_mut(self._val() & other._val());
        new
    }

    /// Return all selected values as a sorted vector.
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

    /// Check if a specific value is selected.
    fn contain(&self, index: Self::DataTy) -> bool {
        let index = index.as_data();
        let val = self._val();
        val & (1 << index) > 0
    }

    /// Return the next selected value after `index`, or `None` if none exists.
    fn next(&self, index: Self::DataTy) -> Option<Self::DataTy>;

    /// Internal: find the next set bit after `index`.
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

    /// Return the minimum selected value.
    fn min_val(&self) -> Self::DataTy;

    /// Internal: find the minimum set bit.
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

    /// Return the raw bitset value.
    fn _val(&self) -> u64;

    /// Set the raw bitset value.
    fn _val_mut(&mut self, val: u64);

    /// Check if no values are selected.
    fn is_zero(&self) -> bool {
        self._val() == 0
    }
}

/// Trait for converting a value type to its raw `u64` representation.
pub trait AsBizData<Ty>: Copy {
    fn as_data(self) -> Ty;
}

/// Trait for converting a raw `u64` value to a typed enum variant.
///
/// Panics if the value is out of valid range. Use [`TryFromData`] for fallible conversion.
pub trait FromData<Ty> {
    fn from_data(val: Ty) -> Self;
}

/// Fallible conversion from a raw value, returning an error for out-of-range values.
pub trait TryFromData<Ty>: FromData<Ty> {
    fn try_from_data(val: Ty) -> crate::error::Result<Self>
    where
        Self: Sized;
}
