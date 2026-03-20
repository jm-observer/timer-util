# 目标 1：基础质量提升

> 修复 Minuter 拼写、增加测试覆盖、添加 doc comment、添加 CI/CD

## 仓库概览

**timer-util** 是一个 Rust 定时器调度库（类 cron），基于位集合 (bitset) 高效存储时间选择项，支持按星期/月日 + 时/分/秒的组合规则计算下一次调度时间。

### 核心架构

```
lib.rs          -- 公开 API 入口：configure_weekday() / configure_monthday()
├── conf.rs     -- 配置结构体：TimerConf, MonthDays, WeekDays, Hours, Minuters, Seconds, Days
├── data.rs     -- 数据枚举：Hour(H0-H23), Minuter(M0-M59), Second(S0-S59), WeekDay(W1-W7), MonthDay(D1-D31)
├── traits.rs   -- 核心 trait：ConfigOperator, Computer, FromData, AsBizData, TryFromData
├── compute.rs  -- 计算引擎：Composition, TimeUnit<T>, DayUnit
└── builder.rs  -- Builder 模式：DayConfBuilder → DayHourConfBuilder → DayHourMinuterConfBuilder → TimerConf
```

### 关键设计

- **位集合配置**：每个配置项用 `u64` 的位表示选中状态，如 `Hours(u64)` 的第 N 位为 1 表示选中第 N 小时
- **Builder 链式调用**：强制 Day → Hour → Minute → Second 顺序构建
- **泛型计算**：`TimeUnit<T: ConfigOperator>` 统一处理时/分/秒的匹配逻辑

---

## 任务 1.1：修复 Minuter 拼写

### 目标

将所有 `Minuter` 改为 `Minute`，这是 **breaking change**，版本应升至 `0.4.0`。

### 涉及的变更清单

| 文件 | 变更内容 |
|------|----------|
| `src/conf.rs` | `Minuters` → `Minutes`，所有 `minuter`/`minuters` 字段和变量名 |
| `src/data.rs` | `enum Minuter` → `enum Minute`，所有 `Minuter::` 引用 |
| `src/traits.rs` | 无直接引用，但 `ConfigOperator` 的 `DataTy` 关联类型会间接受影响 |
| `src/compute.rs` | `TimeUnit<Minuters>` → `TimeUnit<Minutes>`，字段名 `minuter` → `minute` |
| `src/builder.rs` | `DayHourMinuterConfBuilder` → `DayHourMinuteConfBuilder`，`Minuters` → `Minutes` |
| `src/lib.rs` | 公开导出：`Minuters` → `Minutes`，`Minuter` → `Minute`，`Minuter::*` → `Minute::*` |
| `examples/timer.rs` | 更新使用示例 |
| `Cargo.toml` | 版本 `0.3.6` → `0.4.0` |
| `README.md` | 更新示例代码 |

### 详细变更项

**枚举重命名：**
```rust
// before
pub enum Minuter { M0 = 0, M1, ..., M59 }
// after
pub enum Minute { M0 = 0, M1, ..., M59 }
```

**配置结构体重命名：**
```rust
// before
pub struct Minuters(u64);
// after
pub struct Minutes(u64);
```

**Builder 重命名：**
```rust
// before
pub struct DayHourMinuterConfBuilder { ... }
pub fn build_with_minuter(self, minuters: Minuters) -> DayHourMinuterConfBuilder
pub fn build_with_second(self, seconds: Seconds) -> TimerConf  // in DayHourMinuterConfBuilder
// after
pub struct DayHourMinuteConfBuilder { ... }
pub fn build_with_minute(self, minutes: Minutes) -> DayHourMinuteConfBuilder
pub fn build_with_second(self, seconds: Seconds) -> TimerConf  // in DayHourMinuteConfBuilder
```

**TimerConf 字段：**
```rust
// before
pub struct TimerConf {
    pub(crate) minuters: Minuters,
    ...
}
// after
pub struct TimerConf {
    pub(crate) minutes: Minutes,
    ...
}
```

**data.rs 中 DateTime 结构体字段：**
```rust
// before
pub(crate) minuter: Minuter,
// after
pub(crate) minute: Minute,
```

### 测试方式

```bash
# 编译通过即基本正确
cargo build

# 运行所有测试
cargo test

# 确保没有残留的 "minuter"（大小写不敏感搜索，排除 git 历史）
grep -ri "minuter" src/ examples/
# 期望结果：无匹配
```

---

## 任务 1.2：增加测试覆盖

### 目标

当前仅 9 个测试，主要集中在 `conf.rs`。需要补充以下模块和场景的测试。

### 当前测试现状

| 模块 | 测试数 | 覆盖情况 |
|------|--------|----------|
| conf.rs | 8 | 有基本的调度计算、范围查询、WeekDay 转 MonthDay、every() |
| compute.rs | 1 | 仅测试了 TimeUnit 的秒级匹配 |
| builder.rs | 0 | 无测试 |
| data.rs | 0 | 无测试 |
| traits.rs | 0 | 通过其他模块间接测试 |

### 需要新增的测试用例

#### A. `data.rs` 测试

```rust
#[cfg(test)]
mod test {
    use super::*;

    // 1. FromData 正常转换
    #[test]
    fn test_hour_from_data() {
        assert_eq!(Hour::from_data(0), H0);
        assert_eq!(Hour::from_data(23), H23);
    }

    // 2. FromData 边界 panic（用 should_panic）
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

    // 3. TryFromData 正常和错误路径
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
        assert!(Minute::try_from_data(60).is_err());  // 拼写修复后
        assert!(Second::try_from_data(60).is_err());
    }

    // 4. AsBizData 往返一致性
    #[test]
    fn test_as_biz_data_roundtrip() {
        for i in 0..24 {
            assert_eq!(Hour::from_data(i).as_data(), i);
        }
        for i in 1..=31 {
            assert_eq!(MonthDay::from_data(i).as_data(), i);
        }
    }

    // 5. DateTime ↔ NaiveDateTime 互转
    #[test]
    fn test_datetime_conversion_roundtrip() {
        let ndt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),  // 闰年
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        );
        let dt: DateTime = ndt.into();
        let back: NaiveDateTime = dt.into();
        assert_eq!(ndt, back);
    }

    // 6. WeekDay ← chrono::Weekday 转换
    #[test]
    fn test_weekday_from_chrono() {
        assert_eq!(WeekDay::from(chrono::Weekday::Mon), W1);
        assert_eq!(WeekDay::from(chrono::Weekday::Sun), W7);
    }
}
```

#### B. `builder.rs` 测试

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    // 1. 基本构建流程
    #[test]
    fn test_builder_basic() {
        let conf = configure_weekday(WeekDays::default_value(W1))
            .build_with_hours(Hours::default_value(H0))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        // 验证 conf 能正常计算
        let _ = conf.next_with_time(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        );
    }

    // 2. 同时配置 weekday 和 monthday
    #[test]
    fn test_builder_combined_days() {
        let conf = configure_weekday(WeekDays::default_value(W1))
            .conf_month_days(MonthDays::default_value(D15))
            .build_with_hours(Hours::default_value(H12))
            .build_with_minute(Minutes::default_value(M0))
            .build_with_second(Seconds::default_value(S0));
        // 验证月日和星期的并集生效
        let _ = conf.next_with_time(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        );
    }

    // 3. 从 monthday 入口构建
    #[test]
    fn test_builder_from_monthday() {
        let conf = configure_monthday(MonthDays::default_value(D1))
            .build_with_hours(Hours::default_all())
            .build_with_minute(Minutes::every(15))
            .build_with_second(Seconds::default_value(S0));
        let _ = conf.next_with_time(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            )
        );
    }
}
```

#### C. `compute.rs` 补充测试

```rust
// 补充 TimeUnit<Hours> 和 TimeUnit<Minutes> 的测试
#[test]
fn test_time_unit_hour() {
    let conf = Hours::default_array(&[H0, H8, H16]);
    let unit = TimeUnit::new(H0, conf.clone());
    assert!(unit.is_match());
    assert_eq!(unit.next_val(), Some(H8));

    let unit = TimeUnit::new(H10, conf.clone());
    assert!(!unit.is_match());
    assert_eq!(unit.next_val(), Some(H16));

    let unit = TimeUnit::new(H20, conf);
    assert!(!unit.is_match());
    assert_eq!(unit.next_val(), None);
}

// 闰年 2 月 29 日跨月测试
#[test]
fn test_leap_year_feb29() {
    let conf = configure_monthday(MonthDays::default_value(D29))
        .build_with_hours(Hours::default_value(H0))
        .build_with_minute(Minutes::default_value(M0))
        .build_with_second(Seconds::default_value(S0));
    let start = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 2, 28).unwrap(),
        NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
    );
    let next = conf.next_with_time(start);
    // 2024 是闰年，2月有29天
    assert_eq!(next, NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    ));
}

// 非闰年 2 月跳过 29 日
#[test]
fn test_non_leap_year_skip_feb29() {
    let conf = configure_monthday(MonthDays::default_value(D29))
        .build_with_hours(Hours::default_value(H0))
        .build_with_minute(Minutes::default_value(M0))
        .build_with_second(Seconds::default_value(S0));
    let start = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2023, 1, 30).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    let next = conf.next_with_time(start);
    // 2023 非闰年，2月无 29 日，应跳到 3 月 29 日
    assert_eq!(next, NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2023, 3, 29).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    ));
}
```

#### D. `conf.rs` 补充测试

```rust
// ConfigOperator trait 方法测试
#[test]
fn test_config_operator_basic() {
    // default_all
    let hours = Hours::default_all();
    assert_eq!(hours.to_vec().len(), 24);

    // intersection
    let h1 = Hours::default_array(&[H0, H6, H12, H18]);
    let h2 = Hours::default_array(&[H6, H12]);
    let inter = h1.intersection(&h2);
    assert_eq!(inter.to_vec(), vec![6, 12]);

    // contain
    assert!(h1.contain(H0));
    assert!(!h1.contain(H1));

    // is_zero
    let empty = Hours::_default();
    assert!(empty.is_zero());
    assert!(!h1.is_zero());
}

// datetimes 错误输入
#[test]
fn test_datetimes_invalid_range() {
    let conf = configure_monthday(MonthDays::default_value(D1))
        .build_with_hours(Hours::default_value(H0))
        .build_with_minute(Minutes::default_value(M0))
        .build_with_second(Seconds::default_value(S0));
    let t1 = datetime(2024, 6, 1, 0, 0, 0);
    let t2 = datetime(2024, 1, 1, 0, 0, 0);
    // start > end 应返回错误
    assert!(conf.datetimes(t1..t2).is_err());
}

// default_all_by_max
#[test]
fn test_default_all_by_max() {
    let hours = Hours::default_all_by_max(H12);
    assert_eq!(hours.to_vec(), vec![0,1,2,3,4,5,6,7,8,9,10,11,12]);
}
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test --lib data::test
cargo test --lib builder::test
cargo test --lib compute::test
cargo test --lib conf::test

# 查看测试覆盖率（需安装 cargo-tarpaulin）
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

---

## 任务 1.3：添加 doc comment

### 目标

为所有公开的类型、方法、trait 添加 `///` 文档注释，使 `docs.rs` 上有完整的 API 文档。

### 需要添加文档的公开项

#### `lib.rs`

```rust
/// Create a timer configuration starting with weekday selection.
///
/// # Example
/// ```
/// use timer_util::*;
/// let conf = configure_weekday(WeekDays::default_value(W1))
///     .build_with_hours(Hours::default_value(H9))
///     .build_with_minute(Minutes::default_value(M0))
///     .build_with_second(Seconds::default_value(S0));
/// ```
pub fn configure_weekday(week_day: WeekDays) -> DayConfBuilder
```

#### `conf.rs` 公开类型

需要文档的项：
- `TimerConf` — 已有简单中文注释，建议补充英文文档 + 示例
- `MonthDays` / `WeekDays` / `Hours` / `Minutes` / `Seconds` — 已有简单中文注释，补充英文
- `TimerConf::next()` / `next_with_time()` / `datetimes()` — 已有中文注释，补充英文 + 示例
- `Minutes::every()` — 添加文档 + 示例

#### `traits.rs` 公开 trait

- `ConfigOperator` — 每个方法都需要文档
- `AsBizData` / `FromData` / `TryFromData` — trait 级别文档
- `Computer` — trait 级别文档

#### `data.rs` 公开枚举

- `Hour` / `Minute` / `Second` / `WeekDay` / `MonthDay` — 枚举级别文档

#### `builder.rs` 公开类型

- `DayConfBuilder` / `DayHourConfBuilder` / `DayHourMinuteConfBuilder` — 类型 + 方法文档

### 文档规范

1. 使用英文编写（作为 crates.io 上的库）
2. 第一行是简短摘要
3. 包含 `# Example` 代码块（会被 `cargo test --doc` 自动测试）
4. 公开方法说明参数含义和返回值

### 验证方式

```bash
# 文档测试（doc example 编译运行）
cargo test --doc

# 生成本地文档预览
cargo doc --open

# 检查是否有缺少文档的公开项（启用 lint）
# 在 lib.rs 顶部添加: #![warn(missing_docs)]
cargo build 2>&1 | grep "missing_docs"
```

---

## 任务 1.4：添加 CI/CD

### 目标

添加 GitHub Actions 工作流，自动执行编译、测试、lint 检查。

### 文件：`.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --all-targets

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-targets
      - run: cargo test --doc

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --all-targets -- -D warnings
```

### 验证方式

```bash
# 本地验证各步骤
cargo check --all-targets
cargo test --all-targets
cargo test --doc
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

---

## 完成标准

- [ ] 所有 `Minuter` / `Minuters` 已重命名为 `Minute` / `Minutes`
- [ ] `Cargo.toml` 版本升至 `0.4.0`
- [ ] 测试数量从 9 个增加到 25+ 个
- [ ] `cargo test` 全部通过
- [ ] 所有公开 API 有 `///` 文档注释
- [ ] `cargo test --doc` 通过
- [ ] `cargo clippy -- -D warnings` 无警告
- [ ] `.github/workflows/ci.yml` 已创建并在 push 后能正常运行
