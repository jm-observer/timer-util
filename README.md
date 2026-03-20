# timer-util

A simple tool to compute time: easy to config, and easy to use.

## Example

```rust
use std::time::Duration;
use timer_util::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Timer configuration:
    // Every Saturday
    // Every hour
    // At minutes 0, 10, 20, 30, 40, 50
    // At seconds 0, 30
    let conf = configure_weekday(WeekDays::default_value(W6))
        .build_with_hours(Hours::default_all())
        .build_with_minute(Minutes::default_array(&[M0, M10, M20, M30, M40, M50]))
        .build_with_second(Seconds::default_array(&[S0, S30]));

    let handle = tokio::spawn(async move {
        loop {
            let off_seconds = conf.next();
            println!("next seconds: {}", off_seconds);
            tokio::time::sleep(Duration::from_secs(off_seconds)).await;
        }
    });
    handle.await.unwrap();
    Ok(())
}
```

## More Examples

```rust
use timer_util::*;

// Every Monday, Wednesday, Friday at 9:00:00 and 18:00:00
let conf = configure_weekday(WeekDays::default_array(&[W1, W3, W5]))
    .build_with_hours(Hours::default_array(&[H9, H18]))
    .build_with_minute(Minutes::default_value(M0))
    .build_with_second(Seconds::default_value(S0));

// 1st and 15th of every month at midnight
let conf = configure_monthday(MonthDays::default_array(&[D1, D15]))
    .build_with_hours(Hours::default_value(H0))
    .build_with_minute(Minutes::default_value(M0))
    .build_with_second(Seconds::default_value(S0));

// Every 15 minutes, every day
let conf = configure_monthday(MonthDays::default_all())
    .build_with_hours(Hours::default_all())
    .build_with_minute(Minutes::every(15))
    .build_with_second(Seconds::default_value(S0));
```
