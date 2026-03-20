use std::time::Duration;
use timer_util::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    custom_utils::logger::logger_stdout_debug();

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
