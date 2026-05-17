use crate::conf::{Days, Hours, Minutes, MonthDays, Seconds, TimerConf, WeekDays};
use crate::data::{Hour, Minute, MonthDay, Second, WeekDay};
use crate::error::TimerError;
use crate::traits::{ConfigOperator, FromData};

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
    pub fn from_cron(expr: &str) -> crate::error::Result<Self> {
        let fields: Vec<&str> = expr.split_whitespace().collect();

        let (sec_field, min_field, hour_field, dom_field, month_field, dow_field) =
            match fields.len() {
                5 => ("0", fields[0], fields[1], fields[2], fields[3], fields[4]),
                6 => (
                    fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
                ),
                _ => {
                    return Err(TimerError::InvalidCronExpression {
                        expression: expr.to_string(),
                        reason: format!("expected 5 or 6 fields, got {}", fields.len()),
                    });
                }
            };

        // month must be *
        if month_field != "*" {
            return Err(TimerError::CronMonthNotSupported {
                value: month_field.to_string(),
            });
        }

        let sec_values = parse_field(sec_field, 0, 59, expr)?;
        let min_values = parse_field(min_field, 0, 59, expr)?;
        let hour_values = parse_field(hour_field, 0, 23, expr)?;
        let dom_values = parse_field(dom_field, 1, 31, expr)?;
        let dow_values = parse_field(dow_field, 1, 7, expr)?;

        let seconds = build_seconds(&sec_values);
        let minutes = build_minutes(&min_values);
        let hours = build_hours(&hour_values);

        let dom_is_all = dom_field == "*";
        let dow_is_all = dow_field == "*";

        let days = match (dom_is_all, dow_is_all) {
            (true, true) => Days::MonthDays(MonthDays::default_all()),
            (true, false) => Days::WeekDays(build_weekdays(&dow_values)),
            (false, true) => Days::MonthDays(build_monthdays(&dom_values)),
            (false, false) => {
                Days::MonthAndWeekDays(build_monthdays(&dom_values), build_weekdays(&dow_values))
            }
        };

        Ok(TimerConf {
            days,
            hours,
            minutes,
            seconds,
        })
    }
}

/// Parse a single cron field string into a sorted, deduplicated set of u64 values.
///
/// Supports: `*`, `N`, `N,M`, `N-M`, `*/N`, `N-M/S`
fn parse_field(field: &str, min: u64, max: u64, expr: &str) -> crate::error::Result<Vec<u64>> {
    let mut values = Vec::new();

    for part in field.split(',') {
        if part == "*" {
            values.extend(min..=max);
        } else if let Some(step_str) = part.strip_prefix("*/") {
            let step: u64 = step_str
                .parse()
                .map_err(|_| TimerError::InvalidCronExpression {
                    expression: expr.to_string(),
                    reason: format!("invalid step: {}", step_str),
                })?;
            if step == 0 {
                return Err(TimerError::InvalidCronExpression {
                    expression: expr.to_string(),
                    reason: "step cannot be 0".to_string(),
                });
            }
            let mut v = min;
            while v <= max {
                values.push(v);
                v += step;
            }
        } else if part.contains('/') {
            // N-M/S range step
            let (range_part, step_str) =
                part.split_once('/')
                    .ok_or_else(|| TimerError::InvalidCronExpression {
                        expression: expr.to_string(),
                        reason: "invalid range/step syntax".to_string(),
                    })?;
            let step: u64 = step_str
                .parse()
                .map_err(|_| TimerError::InvalidCronExpression {
                    expression: expr.to_string(),
                    reason: format!("invalid step: {}", step_str),
                })?;
            if step == 0 {
                return Err(TimerError::InvalidCronExpression {
                    expression: expr.to_string(),
                    reason: "step cannot be 0".to_string(),
                });
            }
            let (start, end) = parse_range(range_part, min, max, expr)?;
            let mut v = start;
            while v <= end {
                values.push(v);
                v += step;
            }
        } else if part.contains('-') {
            // N-M range
            let (start, end) = parse_range(part, min, max, expr)?;
            values.extend(start..=end);
        } else {
            // single value
            let v: u64 = part
                .parse()
                .map_err(|_| TimerError::InvalidCronExpression {
                    expression: expr.to_string(),
                    reason: format!("invalid value: {}", part),
                })?;
            if v < min || v > max {
                return Err(TimerError::InvalidCronExpression {
                    expression: expr.to_string(),
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

fn parse_range(s: &str, min: u64, max: u64, expr: &str) -> crate::error::Result<(u64, u64)> {
    let (start_str, end_str) =
        s.split_once('-')
            .ok_or_else(|| TimerError::InvalidCronExpression {
                expression: expr.to_string(),
                reason: format!("invalid range: {}", s),
            })?;
    let start: u64 = start_str
        .parse()
        .map_err(|_| TimerError::InvalidCronExpression {
            expression: expr.to_string(),
            reason: format!("invalid range start: {}", start_str),
        })?;
    let end: u64 = end_str
        .parse()
        .map_err(|_| TimerError::InvalidCronExpression {
            expression: expr.to_string(),
            reason: format!("invalid range end: {}", end_str),
        })?;
    if start < min || end > max || start > end {
        return Err(TimerError::InvalidCronExpression {
            expression: expr.to_string(),
            reason: format!(
                "range {}-{} is invalid (valid range: [{}, {}])",
                start, end, min, max
            ),
        });
    }
    Ok((start, end))
}

fn build_seconds(values: &[u64]) -> Seconds {
    let mut s = Seconds::_default();
    for &v in values {
        s = s.add(Second::from_data(v));
    }
    s
}

fn build_minutes(values: &[u64]) -> Minutes {
    let mut m = Minutes::_default();
    for &v in values {
        m = m.add(Minute::from_data(v));
    }
    m
}

fn build_hours(values: &[u64]) -> Hours {
    let mut h = Hours::_default();
    for &v in values {
        h = h.add(Hour::from_data(v));
    }
    h
}

fn build_monthdays(values: &[u64]) -> MonthDays {
    let mut md = MonthDays::_default();
    for &v in values {
        md = md.add(MonthDay::from_data(v));
    }
    md
}

fn build_weekdays(values: &[u64]) -> WeekDays {
    let mut wd = WeekDays::_default();
    for &v in values {
        wd = wd.add(WeekDay::from_data(v));
    }
    wd
}

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

    /// Every day at 9:30:00
    #[test]
    fn test_cron_daily_930() {
        let conf = TimerConf::from_cron("0 30 9 * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 8, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 30, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 10, 0, 0));
        assert_eq!(next, dt(2024, 3, 16, 9, 30, 0));
    }

    /// Every 15 minutes (0, 15, 30, 45)
    #[test]
    fn test_cron_every_15min() {
        let conf = TimerConf::from_cron("0 */15 * * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 10, 14, 0));
        assert_eq!(next, dt(2024, 3, 15, 10, 15, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 10, 45, 0));
        assert_eq!(next, dt(2024, 3, 15, 11, 0, 0));
    }

    /// Weekday business hours 9-17
    #[test]
    fn test_cron_weekday_business_hours() {
        let conf = TimerConf::from_cron("0 0 9-17 * * 1-5").unwrap();
        // 2024-03-15 is Friday (W5)
        let next = conf.next_with_time(dt(2024, 3, 15, 17, 0, 0));
        // Next business day is Monday 3/18
        assert_eq!(next, dt(2024, 3, 18, 9, 0, 0));
    }

    /// Monthly on 1st and 15th at 0:00:00
    #[test]
    fn test_cron_monthday() {
        let conf = TimerConf::from_cron("0 0 0 1,15 * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 1, 0, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 0, 0, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 0, 0, 0));
        assert_eq!(next, dt(2024, 4, 1, 0, 0, 0));
    }

    /// Every 30 seconds
    #[test]
    fn test_cron_every_30sec() {
        let conf = TimerConf::from_cron("*/30 * * * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 10, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 10, 0, 30));

        let next = conf.next_with_time(dt(2024, 3, 15, 10, 0, 30));
        assert_eq!(next, dt(2024, 3, 15, 10, 1, 0));
    }

    /// 5-field format (no seconds, defaults to 0)
    #[test]
    fn test_cron_5_fields() {
        let conf = TimerConf::from_cron("30 9 * * *").unwrap();
        let next = conf.next_with_time(dt(2024, 3, 15, 8, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 30, 0));
    }

    /// Range step 0-30/10
    #[test]
    fn test_cron_range_step() {
        let conf = TimerConf::from_cron("0 0-30/10 9 * * *").unwrap();
        // Should match 9:00, 9:10, 9:20, 9:30
        let next = conf.next_with_time(dt(2024, 3, 15, 9, 0, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 10, 0));

        let next = conf.next_with_time(dt(2024, 3, 15, 9, 25, 0));
        assert_eq!(next, dt(2024, 3, 15, 9, 30, 0));
    }

    /// Verify cron matches builder API
    #[test]
    fn test_cron_matches_builder() {
        use crate::*;

        // Builder
        let builder_conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
            .build_with_hours(Hours::default_array(&[H9, H18]))
            .build_with_minute(Minutes::every(15))
            .build_with_second(Seconds::default_value(S0));

        // Cron
        let cron_conf = TimerConf::from_cron("0 */15 9,18 * * 1,3,5").unwrap();

        let test_times = vec![
            dt(2024, 3, 11, 0, 0, 0),   // Monday
            dt(2024, 3, 13, 9, 15, 0),  // Wednesday
            dt(2024, 3, 15, 18, 45, 0), // Friday
            dt(2024, 3, 16, 10, 0, 0),  // Saturday
        ];

        for t in test_times {
            assert_eq!(
                builder_conf.next_with_time(t),
                cron_conf.next_with_time(t),
                "Mismatch at {:?}",
                t
            );
        }
    }
}

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
        assert!(matches!(
            result,
            Err(TimerError::CronMonthNotSupported { .. })
        ));
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
        assert!(TimerConf::from_cron("0 */0 * * * *").is_err()); // step=0
        assert!(TimerConf::from_cron("0 10-5 * * * *").is_err()); // inverted range
    }

    #[test]
    fn test_cron_empty_string() {
        assert!(TimerConf::from_cron("").is_err());
        assert!(TimerConf::from_cron("   ").is_err());
    }
}
