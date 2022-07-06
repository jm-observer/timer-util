use std::time::Duration;
use timer_util::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    custom_utils::logger::logger_stdout_debug();

    // 定时器配置（timer configs）：
    // every weekday or 1st..10st 15st..25st every month    每周六 或者每月的1号到9号、15号到24号
    // every hour   每小时
    // 0st/10st/20st/30st/40st/50st minuter 第0/10/20/30/40/50分钟
    // 0st/30st second  第0/30秒
    // let conf = configure_weekday(WeekDays::default_value(W6))
    //     .conf_month_days(MonthDays::default_range(D1..D10)?.add_range(D15..D25)?)
    //     .build_with_hours(Hours::default_all())
    //     .build_with_minuter(Minuters::default_array(&[M0, M10, M20, M30, M40, M50]))
    //     .build_with_second(Seconds::default_array(&[S0, S30]));

    let conf = configure_weekday(WeekDays::default_value(W6))
        .build_with_hours(Hours::default_all())
        .build_with_minuter(Minuters::default_array(&[M0, M10, M20, M30, M40, M50]))
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
