# 目标 2：架构与功能增强

> 自定义错误类型、Serde 支持、迭代器接口、过程宏消除重复

## 仓库概览

参见 [goal-1.md](./goal-1.md) 的仓库概览部分。本目标基于目标 1 完成后的代码（已修复拼写、版本 >= 0.4.0）。

---

## 任务 2.1：自定义错误类型

### 背景

当前库使用 `anyhow::Result` 作为公开 API 的返回类型。`anyhow` 适合应用层，但作为库应暴露具体的错误类型，让调用方可以 `match` 处理。

### 当前 anyhow 使用点

| 文件 | 位置 | 错误场景 |
|------|------|----------|
| `conf.rs:22-48` | `TimerConf::datetimes()` | `Bound::Unbounded`（不支持无界范围）、`start >= end`（范围错误） |
| `traits.rs:75-96` | `ConfigOperator::add_range()` | `first > end`（范围起止错误） |
| `data.rs:538-576` | `TryFromData` 各实现 | 值超出有效范围 |

此外 `data.rs` 中的 `FromData` 实现使用 `assert!` + `unreachable!` 宏直接 panic，也需要考虑是否改为返回 Result。

### 设计方案

新建 `src/error.rs`：

```rust
use std::fmt;

/// Errors that can occur in timer-util.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerError {
    /// The range start is greater than the range end.
    InvalidRange {
        start: u64,
        end: u64,
    },
    /// Unbounded ranges are not supported in `datetimes()`.
    UnboundedRange,
    /// A value is out of the valid range for its type.
    ValueOutOfRange {
        type_name: &'static str,
        value: u64,
        min: u64,
        max: u64,
    },
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange { start, end } => {
                write!(f, "invalid range: start ({}) > end ({})", start, end)
            }
            Self::UnboundedRange => {
                write!(f, "unbounded ranges are not supported")
            }
            Self::ValueOutOfRange { type_name, value, min, max } => {
                write!(f, "{} value {} is out of range [{}, {}]", type_name, value, min, max)
            }
        }
    }
}

impl std::error::Error for TimerError {}

/// A specialized `Result` type for timer-util.
pub type Result<T> = std::result::Result<T, TimerError>;
```

### 变更清单

| 文件 | 变更 |
|------|------|
| `src/error.rs` | 新建，定义 `TimerError` 枚举 |
| `src/lib.rs` | 添加 `mod error;`，公开导出 `TimerError`, `Result` |
| `src/conf.rs` | `datetimes()` 返回值从 `anyhow::Result` 改为 `crate::error::Result` |
| `src/traits.rs` | `add_range()` / `default_range()` 返回值改为 `crate::error::Result` |
| `src/data.rs` | `TryFromData` 返回值改为 `crate::error::Result` |
| `Cargo.toml` | 移除 `anyhow` 依赖（如果 panic 路径也改为 Result 的话） |

### 关于 `FromData` 中的 panic

`FromData` trait 的 `from_data()` 方法当前使用 `assert!` 做校验，越界直接 panic。有两个选择：

1. **保持 panic**：`FromData` 是内部 trait（`pub(crate)` 使用），调用方已经保证输入合法，panic 作为 bug 的断言合理
2. **改为 TryFrom**：将 `FromData` 合并到 `TryFromData`，统一用 Result

**建议方案 1**：保持 `FromData` 的 panic（内部使用），仅修改公开 API 的错误类型。

### 测试数据

```rust
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_invalid_range_error() {
        let result = Hours::default_range(H10..H5);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            TimerError::InvalidRange { start: 10, end: 4 }
        );
    }

    #[test]
    fn test_unbounded_range_error() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let t = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        assert!(matches!(conf.datetimes(t..), Err(TimerError::UnboundedRange)));
        assert!(matches!(conf.datetimes(..t), Err(TimerError::UnboundedRange)));
    }

    #[test]
    fn test_value_out_of_range() {
        assert!(matches!(
            MonthDay::try_from_data(0),
            Err(TimerError::ValueOutOfRange { type_name: "MonthDay", .. })
        ));
        assert!(matches!(
            Hour::try_from_data(24),
            Err(TimerError::ValueOutOfRange { type_name: "Hour", .. })
        ));
    }
}
```

### 验证方式

```bash
cargo build
cargo test
# 确保 anyhow 不再出现在公开 API 签名中
cargo doc --open  # 检查文档中的返回值类型
```

---

## 任务 2.2：Serde 支持

### 背景

添加 `serde` feature flag，允许 `TimerConf` 和相关配置类型的序列化/反序列化，方便从配置文件加载调度规则。

### 设计方案

使用 Cargo feature gate：

```toml
# Cargo.toml
[features]
default = []
serde = ["dep:serde"]

[dependencies]
serde = { version = "1", features = ["derive"], optional = true }
```

为核心类型添加条件编译的 Serde 实现：

```rust
// conf.rs
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hours(u64);
```

### 需要添加 Serde derive 的类型

| 类型 | 文件 | 备注 |
|------|------|------|
| `Hours` | conf.rs | u64 包装，直接 derive |
| `Minutes` | conf.rs | u64 包装，直接 derive |
| `Seconds` | conf.rs | u64 包装，直接 derive |
| `MonthDays` | conf.rs | u64 包装，直接 derive |
| `WeekDays` | conf.rs | u64 包装，直接 derive |
| `Days` | conf.rs | 枚举，包含上述类型 |
| `TimerConf` | conf.rs | 组合以上类型 |
| `Hour` | data.rs | repr(u64) 枚举 |
| `Minute` | data.rs | repr(u64) 枚举 |
| `Second` | data.rs | repr(u64) 枚举 |
| `MonthDay` | data.rs | repr(u64) 枚举 |
| `WeekDay` | data.rs | repr(u64) 枚举 |

### 序列化格式示例

```json
{
  "days": {
    "WeekDays": 42
  },
  "hours": 16777216,
  "minutes": 1073741824,
  "seconds": 1
}
```

注意：直接序列化 u64 位集合对人类不友好。可考虑自定义序列化来暴露数组形式：

```rust
// 可选的人类可读序列化
#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{Serializer, Deserializer};

    // 将 Hours(u64) 序列化为 [0, 8, 16] 这样的数组
    impl serde::Serialize for Hours {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.to_vec().serialize(serializer)
        }
    }
    // 从数组反序列化回来
    impl<'de> serde::Deserialize<'de> for Hours {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let vec: Vec<u64> = Vec::deserialize(deserializer)?;
            // 重建位集合
            let mut hours = Hours::_default();
            for v in vec {
                if v > 23 {
                    return Err(serde::de::Error::custom(format!("hour {} out of range", v)));
                }
                hours._val_mut(hours._val() | (1 << v));
            }
            Ok(hours)
        }
    }
}
```

**建议**：先使用简单的 derive 方式，后续版本再考虑人类可读格式。

### 测试数据

```rust
#[cfg(test)]
#[cfg(feature = "serde")]
mod serde_test {
    use super::*;

    #[test]
    fn test_hours_serde_roundtrip() {
        let hours = Hours::default_array(&[H0, H8, H16]);
        let json = serde_json::to_string(&hours).unwrap();
        let back: Hours = serde_json::from_str(&json).unwrap();
        assert_eq!(hours.to_vec(), back.to_vec());
    }

    #[test]
    fn test_timer_conf_serde_roundtrip() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_value(H9))
            .build_with_minute(Minutes::default_value(M30))
            .build_with_second(Seconds::default_value(S0));
        let json = serde_json::to_string(&conf).unwrap();
        let back: TimerConf = serde_json::from_str(&json).unwrap();
        // 验证行为一致
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        assert_eq!(conf.next_with_time(start), back.next_with_time(start));
    }

    #[test]
    fn test_weekday_enum_serde() {
        let day = WeekDay::W3;
        let json = serde_json::to_string(&day).unwrap();
        let back: WeekDay = serde_json::from_str(&json).unwrap();
        assert_eq!(day, back);
    }
}
```

### 验证方式

```bash
# 不启用 feature 时应正常编译
cargo build
cargo test

# 启用 serde feature
cargo build --features serde
cargo test --features serde

# dev-dependencies 需要添加
# serde_json = "1"  （仅用于测试）
```

---

## 任务 2.3：迭代器接口

### 背景

当前 `TimerConf::datetimes()` 返回 `Vec<NaiveDateTime>`，对于大范围查询会分配大量内存。改为返回迭代器可以按需生成，且更符合 Rust 惯用法。

### 设计方案

新建 `src/iter.rs`：

```rust
use chrono::NaiveDateTime;
use crate::conf::TimerConf;

/// An iterator that yields scheduled `NaiveDateTime` values from a `TimerConf`.
///
/// Created by [`TimerConf::iter_from()`] or [`TimerConf::iter_range()`].
pub struct TimerIter<'a> {
    conf: &'a TimerConf,
    current: NaiveDateTime,
    end: Option<NaiveDateTime>,
}

impl<'a> TimerIter<'a> {
    pub(crate) fn new(conf: &'a TimerConf, start: NaiveDateTime, end: Option<NaiveDateTime>) -> Self {
        Self { conf, current: start, end }
    }
}

impl<'a> Iterator for TimerIter<'a> {
    type Item = NaiveDateTime;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.conf.next_with_time(self.current);
        if let Some(end) = self.end {
            if next > end {
                return None;
            }
        }
        self.current = next;
        Some(next)
    }
}
```

### API 变更

在 `TimerConf` 上新增方法：

```rust
impl TimerConf {
    /// Returns an iterator over all scheduled times starting after `start`.
    ///
    /// The iterator is infinite — use `.take(n)` or `.take_while()` to limit.
    ///
    /// # Example
    /// ```
    /// # use timer_util::*;
    /// # use chrono::NaiveDateTime;
    /// let conf = configure_monthday(MonthDays::default_value(D1))
    ///     .build_with_hours(Hours::default_value(H9))
    ///     .build_with_minute(Minutes::default_value(M0))
    ///     .build_with_second(Seconds::default_value(S0));
    /// // Get next 5 scheduled times
    /// let times: Vec<_> = conf.iter_from(start).take(5).collect();
    /// ```
    pub fn iter_from(&self, start: NaiveDateTime) -> TimerIter<'_> {
        TimerIter::new(self, start, None)
    }

    /// Returns an iterator over scheduled times within `[start, end]`.
    pub fn iter_range(&self, start: NaiveDateTime, end: NaiveDateTime) -> TimerIter<'_> {
        TimerIter::new(self, start, Some(end))
    }
}
```

### 保持向后兼容

`datetimes()` 方法保留，但内部实现改为使用迭代器：

```rust
pub fn datetimes(&self, range: impl RangeBounds<NaiveDateTime>) -> Result<Vec<NaiveDateTime>> {
    // ... 边界解析同原逻辑 ...
    Ok(self.iter_range(start, end).collect())
}
```

### 测试数据

```rust
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_iter_from_take() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let times: Vec<_> = conf.iter_from(start).take(3).collect();
        assert_eq!(times, vec![
            // 2024-02-01, 2024-03-01, 2024-04-01
            NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        ]);
    }

    #[test]
    fn test_iter_range_matches_datetimes() {
        let conf = configure_weekday(WeekDays::default_array(&[W5, W3]))
            .conf_month_days(MonthDays::default_array(&[D5, D15, D24]))
            .build_with_hours(Hours::default_array(&[H5, H10, H15]))
            .build_with_minute(Minutes::default_array(&[M15, M30, M45]))
            .build_with_second(Seconds::default_array(&[S15, S30, S45]));
        let start = datetime(2020, 5, 15, 10, 30, 17);
        let end = datetime(2020, 5, 15, 15, 30, 30);

        let from_datetimes = conf.datetimes(start..=end).unwrap();
        let from_iter: Vec<_> = conf.iter_range(start, end).collect();
        assert_eq!(from_datetimes, from_iter);
    }

    #[test]
    fn test_iter_is_lazy() {
        // 验证迭代器不会提前计算所有值
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        let start = datetime(2024, 1, 1, 0, 0, 0);
        let mut iter = conf.iter_from(start);
        // 只取一个值，不应卡死
        let first = iter.next().unwrap();
        assert_eq!(first, datetime(2024, 2, 1, 0, 0, 0));
    }
}
```

### 验证方式

```bash
cargo test
# 特别验证迭代器与 datetimes() 输出一致
cargo test iter
```

---

## 任务 2.4：过程宏消除重复

### 背景

`data.rs` 中存在大量重复代码：`Hour`、`Minute`、`Second`、`WeekDay`、`MonthDay` 五个枚举的定义模式几乎一样，`FromData`、`AsBizData`、`TryFromData` 的实现也是高度相似的 match 语句。总计约 **500 行**重复代码。

### 当前重复模式

每个枚举都需要：
1. 枚举定义（`#[repr(u64)]` + 变体列表）
2. `FromData<u64>` 实现（大 match 语句）
3. `AsBizData<u64>` 实现（`self as u64`）
4. `TryFromData<u64>` 实现（边界检查 + 委托 FromData）

### 设计方案

创建声明宏 (`macro_rules!`)，不需要过程宏 crate（更轻量）：

```rust
// src/macros.rs 或直接在 data.rs 顶部

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
                if val < $min || val > $max {
                    Err(crate::error::TimerError::ValueOutOfRange {
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
```

### 使用方式

```rust
// data.rs

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
        M0 = 0, M1 = 1, M2 = 2, /* ... */ M59 = 59,
    }
    range: 0 => 59,
    type_name: "Minute"
}

define_time_enum! {
    /// Represents a second within a minute (0-59).
    pub enum Second {
        S0 = 0, S1 = 1, S2 = 2, /* ... */ S59 = 59,
    }
    range: 0 => 59,
    type_name: "Second"
}

define_time_enum! {
    /// Represents a day of the week (1=Mon, 7=Sun).
    pub enum WeekDay {
        W1 = 1, W2 = 2, W3 = 3, W4 = 4, W5 = 5, W6 = 6, W7 = 7,
    }
    range: 1 => 7,
    type_name: "WeekDay"
}

define_time_enum! {
    /// Represents a day of the month (1-31).
    pub enum MonthDay {
        D1 = 1, D2 = 2, D3 = 3, /* ... */ D31 = 31,
    }
    range: 1 => 31,
    type_name: "MonthDay"
}
```

### 代码量对比

| 部分 | 改造前（行数） | 改造后（行数） |
|------|---------------|---------------|
| 枚举定义 (5个) | ~220 行 | ~60 行（宏调用） |
| FromData (5个) | ~220 行 | 0（宏生成） |
| AsBizData (5个) | ~25 行 | 0（宏生成） |
| TryFromData (5个) | ~40 行 | 0（宏生成） |
| 宏定义 | 0 | ~40 行 |
| **总计** | **~505 行** | **~100 行** |

减少约 **80%** 的重复代码。

### 注意事项

- `WeekDay` 还有额外的 `From<chrono::Weekday>` 实现，需要在宏外单独保留
- `DateTime` 结构体及其转换实现不受影响
- 宏展开后的行为应与手写代码完全一致

### 测试方式

```bash
# 确保宏展开后行为一致——所有现有测试必须通过
cargo test

# 检查宏展开结果（调试用）
cargo expand --lib  # 需要 cargo-expand: cargo install cargo-expand

# 验证各枚举类型仍正常工作
cargo test data::test
```

---

## 完成标准

- [ ] `anyhow` 从公开 API 中移除，替换为 `TimerError`
- [ ] `TimerError` 实现了 `std::error::Error` + `Display` + `Debug`
- [ ] `serde` feature flag 可选启用
- [ ] 启用 serde 后所有核心类型可序列化/反序列化
- [ ] `TimerIter` 迭代器实现了 `Iterator<Item = NaiveDateTime>`
- [ ] `iter_from()` / `iter_range()` 方法已添加
- [ ] `datetimes()` 内部基于迭代器实现
- [ ] `define_time_enum!` 宏消除了 data.rs 中的重复代码
- [ ] `cargo test` 全部通过
- [ ] `cargo test --features serde` 全部通过
- [ ] `cargo clippy -- -D warnings` 无警告

## 版本规划

本目标的变更（自定义错误类型、新增迭代器 API）是 breaking change，建议与目标 1 合并到 `0.4.0` 版本发布，或单独作为 `0.5.0`。
