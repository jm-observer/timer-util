# Plan 4: 统一调度循环

## 目标

实现调度器核心，集成 timer-util 计算触发时间，管理所有闹钟的生命周期。

## 前置依赖

Plan 2（db 层）和 Plan 3（callback）完成。

## 实现方式

### 1. scheduler.rs — 调度器

**通信协议：**

```rust
pub enum SchedulerCommand {
    Reload,     // 数据变更，重新加载计算
    Shutdown,   // 关闭调度器
}
```

**调度器结构：**

```rust
pub struct Scheduler {
    rx: mpsc::Receiver<SchedulerCommand>,
    db: Arc<Database>,
    http_client: reqwest::Client,
    /// 跟踪每个 cron 闹钟当前正在执行/重试的回调 task
    active_callbacks: HashMap<String, JoinHandle<()>>,
}
```

**核心循环逻辑：**

```rust
impl Scheduler {
    pub async fn run(mut self) {
        loop {
            // 1. 从 DB 加载所有 active 闹钟
            let alarms = self.db.list_alarms(Some("active"));

            // 2. 计算每个闹钟的 next_fire_at
            let now = Local::now().naive_local();
            let mut fire_list: Vec<(AlarmRecord, NaiveDateTime)> = vec![];
            for alarm in &alarms {
                if let Some(next) = compute_next_fire(&alarm, now) {
                    fire_list.push((alarm.clone(), next));
                }
            }

            // 3. 找最近的触发时间
            let nearest = fire_list.iter().map(|(_, t)| *t).min();
            let sleep_duration = match nearest {
                Some(t) if t > now => {
                    let dur = (t - now).to_std().unwrap_or(Duration::from_secs(3600));
                    dur.min(Duration::from_secs(3600)) // cap 1 小时
                }
                Some(_) => Duration::ZERO,  // 已到期
                None => Duration::from_secs(3600),  // 无闹钟，等 1 小时
            };

            // 4. 等待触发或新指令
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    let now = Local::now().naive_local();
                    for (alarm, fire_at) in &fire_list {
                        if *fire_at <= now {
                            self.trigger_alarm(alarm);
                        }
                    }
                }
                cmd = self.rx.recv() => {
                    match cmd {
                        Some(SchedulerCommand::Reload) => continue,
                        Some(SchedulerCommand::Shutdown) | None => {
                            self.cancel_all();
                            break;
                        }
                    }
                }
            }
        }
    }
}
```

### 2. compute_next_fire 函数

```rust
fn compute_next_fire(alarm: &AlarmRecord, now: NaiveDateTime) -> Option<NaiveDateTime> {
    match alarm.alarm_type.as_str() {
        "cron" => {
            let expr = alarm.cron_expr.as_ref()?;
            let conf = TimerConf::from_cron(expr).ok()?;
            conf.next_with_time(now)
        }
        "once" => {
            let at_str = alarm.once_at.as_ref()?;
            let at = NaiveDateTime::parse_from_str(at_str, "%Y-%m-%dT%H:%M:%S").ok()?;
            if at > now { Some(at) } else { None }
        }
        _ => None,
    }
}
```

此函数也在 handlers.rs 中复用，用于计算 API 响应中的 `next_fire_at`。

### 3. trigger_alarm 方法

```rust
fn trigger_alarm(&mut self, alarm: &AlarmRecord) {
    // Cron 闹钟：取消前次回调（如果还在重试中）
    if alarm.alarm_type == "cron" {
        if let Some(prev) = self.active_callbacks.remove(&alarm.id) {
            prev.abort();
        }
    }

    // Spawn 短期回调 task
    let client = self.http_client.clone();
    let alarm = alarm.clone();
    let db = self.db.clone();
    let is_once = alarm.alarm_type == "once";

    let handle = tokio::spawn(async move {
        fire_callback(&client, &alarm).await;
        if is_once {
            let _ = db.update_status(&alarm.id, "completed");
        }
    });

    // Cron 闹钟：记录 handle 以便下次取消
    if !is_once {
        self.active_callbacks.insert(alarm.id.clone(), handle);
    }
}
```

### 4. 启动恢复

在 main.rs 启动调度器前处理：

```rust
// 将过期的一次性闹钟标记为 completed
let active_alarms = db.list_alarms(Some("active"));
for alarm in &active_alarms {
    if alarm.alarm_type == "once" {
        if let Some(at_str) = &alarm.once_at {
            if let Ok(at) = NaiveDateTime::parse_from_str(at_str, "%Y-%m-%dT%H:%M:%S") {
                if at <= Local::now().naive_local() {
                    db.update_status(&alarm.id, "completed");
                }
            }
        }
    }
}
```

调度器启动后第一次循环自动加载剩余 active 闹钟。

## 涉及文件

- `alarm-server/src/scheduler.rs` — 核心实现
- `alarm-server/src/main.rs` — 创建 channel，spawn 调度器

## 测试

1. **单元测试 compute_next_fire**：构造不同类型的 AlarmRecord，验证返回的 next_fire_at 正确
2. **集成测试**：创建一个短间隔 cron 闹钟（如 `*/2 * * * * *`），配合 mock callback server，验证在预期时间内收到回调
3. **测试 Reload**：启动调度器后通过 channel 发送 Reload，验证新增的闹钟被纳入调度

```bash
cargo test -p alarm-server -- scheduler
```
