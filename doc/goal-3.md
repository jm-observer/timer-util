# 目标 3：Cron 表达式解析

> 支持标准 cron 表达式解析，降低用户使用门槛

## 仓库概览

参见 [goal-1.md](./goal-1.md) 的仓库概览部分。本目标基于目标 1 和目标 2 完成后的代码。

---

## 背景

当前 timer-util 的配置方式是通过 Rust Builder API：

```rust
let conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
    .build_with_hours(Hours::default_array(&[H9, H18]))
    .build_with_minute(Minutes::every(15))
    .build_with_second(Seconds::default_value(S0));
```

这种方式类型安全，但学习成本高。业界通用的 cron 表达式格式更简洁直观：

```
0 */15 9,18 * * 1,3,5
```

支持从 cron 表达式解析为 `TimerConf`，可以大幅降低使用门槛，同时保留原有 Builder API。

---

## Cron 表达式格式

### 标准 6 字段格式

```
┌──────── second (0-59)
│ ┌────── minute (0-59)
│ │ ┌──── hour (0-23)
│ │ │ ┌── day of month (1-31)
│ │ │ │ ┌ month (1-12) ← timer-util 暂不支持，忽略
│ │ │ │ │ ┌ day of week (1-7, 1=Mon)
│ │ │ │ │ │
* * * * * *
```

### 支持的语法元素

| 语法 | 含义 | 示例 |
|------|------|------|
| `*` | 全选（所有值） | `* * * * * *` |
| `N` | 单个值 | `0 30 9 * * *` → 每天 9:30:00 |
| `N,M` | 列表 | `0 0 9,18 * * *` → 9:00 和 18:00 |
| `N-M` | 范围 | `0 0 9-17 * * *` → 9:00~17:00 每小时 |
| `*/N` | 步进（从最小值起） | `0 */15 * * * *` → 每15分钟 |
| `N-M/S` | 范围步进 | `0 0-30/10 * * * *` → 0,10,20,30分 |

### 与 timer-util 的映射关系

| Cron 字段 | timer-util 类型 | 值域 |
|-----------|----------------|------|
| second | `Seconds` | 0-59 |
| minute | `Minutes` | 0-59 |
| hour | `Hours` | 0-23 |
| day of month | `MonthDays` | 1-31 |
| month | 不支持（忽略，总是 `*`） | - |
| day of week | `WeekDays` | 1-7 (1=Mon) |

### 关于 month 字段

timer-util 不支持限制月份，因此 month 字段只允许 `*`。如果用户指定了具体月份，返回错误提示。

---

## 设计方案

### 新增文件

```
src/
├── cron.rs          -- cron 解析器主逻辑
└── cron_parser.rs   -- 字段级解析工具函数（可选，也可合并到 cron.rs）
```

### 核心 API

```rust
// src/cron.rs

use crate::conf::TimerConf;
use crate::error::{TimerError, Result};

impl TimerConf {
    /// Parse a cron expression into a `TimerConf`.
    ///
    /// Supports 5-field (without seconds) and 6-field (with seconds) formats:
    /// - 5 fields: `minute hour day_of_month month day_of_week`
    /// - 6 fields: `second minute hour day_of_month month day_of_week`
    ///
    /// The `month` field must be `*` (timer-util does not support month filtering).
    ///
    /// # Examples
    ///
    /// ```
    /// use timer_util::TimerConf;
    ///
    /// // Every day at 9:30:00
    /// let conf = TimerConf::from_cron("0 30 9 * * *").unwrap();
    ///
    /// // Every 15 minutes on weekdays (5-field, seconds default to 0)
    /// let conf = TimerConf::from_cron("*/15 9-17 * * 1-5").unwrap();
    ///
    /// // Every 30 seconds
    /// let conf = TimerConf::from_cron("*/30 * * * * *").unwrap();
    /// ```
    pub fn from_cron(expr: &str) -> Result<Self> {
        // ...
    }
}
```

### 错误类型扩展

在 `TimerError` 中新增变体：

```rust
pub enum TimerError {
    // ... 已有变体 ...

    /// Invalid cron expression.
    InvalidCronExpression {
        expression: String,
        reason: String,
    },

    /// Month field in cron expression is not supported (must be `*`).
    CronMonthNotSupported {
        value: String,
    },
}
```

### 解析器实现思路

```rust
/// Represents a parsed cron field.
enum CronField {
    /// `*` — all values
    All,
    /// `*/N` — step from min
    Step(u64),
    /// Explicit set of values (after expanding ranges, lists, steps)
    Values(Vec<u64>),
}

/// Parse a single cron field string into a set of u64 values.
///
/// Supports: `*`, `N`, `N,M`, `N-M`, `*/N`, `N-M/S`
fn parse_field(field: &str, min: u64, max: u64) -> Result<Vec<u64>> {
    let mut values = Vec::new();

    for part in field.split(',') {
        if part == "*" {
            // 全选
            values.extend(min..=max);
        } else if let Some(step_str) = part.strip_prefix("*/") {
            // */N 步进
            let step: u64 = step_str.parse()
                .map_err(|_| TimerError::InvalidCronExpression {
                    expression: field.to_string(),
                    reason: format!("invalid step: {}", step_str),
                })?;
            if step == 0 {
                return Err(TimerError::InvalidCronExpression {
                    expression: field.to_string(),
                    reason: "step cannot be 0".to_string(),
                });
            }
            let mut v = min;
            while v <= max {
                values.push(v);
                v += step;
            }
        } else if part.contains('/') {
            // N-M/S 范围步进
            let (range_part, step_str) = part.split_once('/')
                .ok_or_else(|| TimerError::InvalidCronExpression {
                    expression: field.to_string(),
                    reason: "invalid range/step syntax".to_string(),
                })?;
            let step: u64 = step_str.parse().map_err(|_| /* ... */)?;
            let (start, end) = parse_range(range_part, min, max)?;
            let mut v = start;
            while v <= end {
                values.push(v);
                v += step;
            }
        } else if part.contains('-') {
            // N-M 范围
            let (start, end) = parse_range(part, min, max)?;
            values.extend(start..=end);
        } else {
            // 单个值
            let v: u64 = part.parse().map_err(|_| /* ... */)?;
            if v < min || v > max {
                return Err(TimerError::InvalidCronExpression {
                    expression: field.to_string(),
                    reason: format!("{} is out of range [{}, {}]", v, min, max),
                });
            }
            values.push(v);
        }
    }

    values.sort();
    values.dedup();
    Ok(values)
}

fn parse_range(s: &str, min: u64, max: u64) -> Result<(u64, u64)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(/* ... */);
    }
    let start: u64 = parts[0].parse().map_err(|_| /* ... */)?;
    let end: u64 = parts[1].parse().map_err(|_| /* ... */)?;
    if start < min || end > max || start > end {
        return Err(/* ... */);
    }
    Ok((start, end))
}
```

### 主解析入口

```rust
pub fn from_cron(expr: &str) -> Result<TimerConf> {
    let fields: Vec<&str> = expr.trim().split_whitespace().collect();

    let (sec_field, min_field, hour_field, dom_field, month_field, dow_field) = match fields.len() {
        5 => {
            // 5-field: min hour dom month dow  (seconds default to 0)
            ("0", fields[0], fields[1], fields[2], fields[3], fields[4])
        }
        6 => {
            (fields[0], fields[1], fields[2], fields[3], fields[4], fields[5])
        }
        _ => {
            return Err(TimerError::InvalidCronExpression {
                expression: expr.to_string(),
                reason: format!("expected 5 or 6 fields, got {}", fields.len()),
            });
        }
    };

    // month 必须是 *
    if month_field != "*" {
        return Err(TimerError::CronMonthNotSupported {
            value: month_field.to_string(),
        });
    }

    let seconds = build_seconds(parse_field(sec_field, 0, 59)?);
    let minutes = build_minutes(parse_field(min_field, 0, 59)?);
    let hours = build_hours(parse_field(hour_field, 0, 23)?);
    let month_days = parse_field(dom_field, 1, 31)?;
    let week_days = parse_field(dow_field, 1, 7)?;

    // 构建 Days 配置
    let dom_is_all = dom_field == "*";
    let dow_is_all = dow_field == "*";

    let conf = match (dom_is_all, dow_is_all) {
        (true, true) => {
            // 都是 * → 每天，用 MonthDays::default_all()
            configure_monthday(MonthDays::default_all())
        }
        (true, false) => {
            // 仅 weekday 限制
            configure_weekday(build_weekdays(week_days))
        }
        (false, true) => {
            // 仅 monthday 限制
            configure_monthday(build_monthdays(month_days))
        }
        (false, false) => {
            // 两者都限制 → 并集
            configure_weekday(build_weekdays(week_days))
                .conf_month_days(build_monthdays(month_days))
        }
    };

    Ok(conf
        .build_with_hours(hours)
        .build_with_minute(minutes)
        .build_with_second(seconds))
}
```

### 辅助构建函数

```rust
fn build_seconds(values: Vec<u64>) -> Seconds {
    let mut s = Seconds::_default();
    for v in values {
        s = s.add(Second::from_data(v));
    }
    s
}

fn build_minutes(values: Vec<u64>) -> Minutes {
    let mut m = Minutes::_default();
    for v in values {
        m = m.add(Minute::from_data(v));
    }
    m
}

fn build_hours(values: Vec<u64>) -> Hours {
    let mut h = Hours::_default();
    for v in values {
        h = h.add(Hour::from_data(v));
    }
    h
}

fn build_monthdays(values: Vec<u64>) -> MonthDays {
    let mut md = MonthDays::_default();
    for v in values {
        md = md.add(MonthDay::from_data(v));
    }
    md
}

fn build_weekdays(values: Vec<u64>) -> WeekDays {
    let mut wd = WeekDays::_default();
    for v in values {
        wd = wd.add(WeekDay::from_data(v));
    }
    wd
}
```

---

## 测试数据

### 基本解析测试

```rust
#[cfg(test)]
mod test {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            NaiveTime::from_hms_opt(h, min, s).unwrap(),
        )
    }

    /// 每天 9:30:00
    #[test]
    fn test_cron_daily_930() {
        let conf = TimerConf::from_cron("0 30 9 * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 8, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 30, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 10, 0, 0));
        assert_eq!(next, dt(2024, 3, 16, 9, 30, 0));
    }

    /// 每15分钟（0, 15, 30, 45）
    #[test]
    fn test_cron_every_15min() {
        let conf = TimerConf::from_cron("0 */15 * * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 10, 14, 0));
        assert_eq!(next, dt(2024, 3, 15, 10, 15, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 10, 45, 0));
        assert_eq!(next, dt(2024, 3, 15, 11, 0, 0));
    }

    /// 工作日 9-17 点每小时
    #[test]
    fn test_cron_weekday_business_hours() {
        let conf = TimerConf::from_cron("0 0 9-17 * * 1-5").unwrap();
        // 2024-03-15 是周五 (W5)
        let next = conf.next_with_time(dt(2024, 3, 15, 17, 0, 0));
        // 下一个工作日是周一 3/18
        assert_eq!(next, dt(2024, 3, 18, 9, 0, 0));
    }

    /// 每月 1 号和 15 号 0:00:00
    #[test]
    fn test_cron_monthday() {
        let conf = TimerConf::from_cron("0 0 0 1,15 * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 1, 0, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 0, 0, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 0, 0, 0));
        assert_eq!(next, dt(2024, 4, 1, 0, 0, 0));
    }

    /// 每30秒
    #[test]
    fn test_cron_every_30sec() {
        let conf = TimerConf::from_cron("*/30 * * * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 10, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 10, 0, 30));

        let next = conf.next_with_time(dt(2024, 3, 15, 10, 0, 30));
        assert_eq!(next, dt(2024, 3, 15, 10, 1, 0));
    }

    /// 5 字段格式（无秒字段，默认秒=0）
    #[test]
    fn test_cron_5_fields() {
        let conf = TimerConf::from_cron("30 9 * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 8, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 30, 0));
    }

    /// 范围步进 0-30/10
    #[test]
    fn test_cron_range_step() {
        let conf = TimerConf::from_cron("0 0-30/10 9 * * *").unwrap();
        // 应匹配 9:00, 9:10, 9:20, 9:30
        let next = conf.next_with_time(dt(2024, 3, 15, 9, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 10, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 9, 25, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 30, 0));
    }
}
```

### 错误处理测试

```rust
#[cfg(test)]
mod error_test {
    use super::*;

    #[test]
    fn test_cron_invalid_field_count() {
        assert!(TimerConf::from_cron("* * *").is_err());
        assert!(TimerConf::from_cron("* * * * * * *").is_err());
    }

    #[test]
    fn test_cron_month_not_supported() {
        let result = TimerConf::from_cron("0 0 9 * 1,6 *");
        assert!(matches!(result, Err(TimerError::CronMonthNotSupported { .. })));
    }

    #[test]
    fn test_cron_value_out_of_range() {
        // second > 59
        assert!(TimerConf::from_cron("60 * * * * *").is_err());
        // hour > 23
        assert!(TimerConf::from_cron("0 0 25 * * *").is_err());
        // weekday > 7
        assert!(TimerConf::from_cron("0 0 0 * * 8").is_err());
        // monthday 0
        assert!(TimerConf::from_cron("0 0 0 0 * *").is_err());
    }

    #[test]
    fn test_cron_invalid_syntax() {
        assert!(TimerConf::from_cron("abc * * * * *").is_err());
        assert!(TimerConf::from_cron("0 */0 * * * *").is_err());  // step=0 无效
        assert!(TimerConf::from_cron("0 10-5 * * * *").is_err());  // 范围倒置
    }

    #[test]
    fn test_cron_empty_string() {
        assert!(TimerConf::from_cron("").is_err());
        assert!(TimerConf::from_cron("   ").is_err());
    }
}
```

### 与 Builder API 一致性测试

```rust
/// 验证 cron 解析结果与 Builder API 构建结果行为一致
#[test]
fn test_cron_matches_builder() {
    // Builder 方式
    let builder_conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
        .build_with_hours(Hours::default_array(&[H9, H18]))
        .build_with_minute(Minutes::every(15))
        .build_with_second(Seconds::default_value(S0));

    // Cron 方式
    let cron_conf = TimerConf::from_cron("0 */15 9,18 * * 1,3,5").unwrap();

    // 比较多个时间点的 next 结果
    let test_times = vec![
        dt(2024, 3, 11, 0, 0, 0),   // 周一
        dt(2024, 3, 13, 9, 15, 0),   // 周三
        dt(2024, 3, 15, 18, 45, 0),  // 周五
        dt(2024, 3, 16, 10, 0, 0),   // 周六（非工作日）
    ];

    for t in test_times {
        assert_eq!(
            builder_conf.next_with_time(t),
            cron_conf.next_with_time(t),
            "Mismatch at {:?}", t
        );
    }
}
```

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/cron.rs` | 新建 | cron 解析器实现 |
| `src/lib.rs` | 修改 | 添加 `mod cron;` |
| `src/error.rs` | 修改 | 添加 `InvalidCronExpression`、`CronMonthNotSupported` 变体 |
| `README.md` | 修改 | 添加 cron 表达式用法示例 |

### 依赖

不需要新增任何外部依赖，纯字符串解析实现。

---

## 完成标准

- [ ] `TimerConf::from_cron()` 方法可用
- [ ] 支持 5 字段和 6 字段格式
- [ ] 支持 `*`、`N`、`N,M`、`N-M`、`*/N`、`N-M/S` 语法
- [ ] month 字段非 `*` 时返回明确错误
- [ ] 所有解析错误返回 `TimerError`，附带清晰的错误信息
- [ ] cron 解析结果与等效 Builder API 行为一致
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy -- -D warnings` 无警告
- [ ] README 包含 cron 用法示例

## 版本规划

建议版本 `0.5.0` 或 `0.6.0`（取决于目标 2 的版本号）。
