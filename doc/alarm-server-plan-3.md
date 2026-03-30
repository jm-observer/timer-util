# Plan 3: 回调 + 重试机制

## 目标

实现回调触发逻辑，包含指数退避持续重试策略和 Cron 闹钟的取消机制。

## 前置依赖

Plan 2 完成（models 已定义 AlarmRecord）。

## 实现方式

### 1. callback.rs — 回调函数

**核心函数签名：**

```rust
pub async fn fire_callback(
    client: &reqwest::Client,
    alarm: &AlarmRecord,
    cancel: CancellationToken,
)
```

**回调请求构造：**
- 方法：POST
- URL：`alarm.callback_url`
- Headers：
  - `Content-Type: application/json`
  - `X-Alarm-Id: {alarm.id}`
  - `X-Alarm-Name: {alarm.name}`
- Body：`alarm.callback_body` 原样发送（如果有的话）

**重试逻辑（在 fire_callback 内部）：**

```rust
let mut retry_interval = Duration::from_secs(5);
let max_interval = Duration::from_secs(600); // 10 分钟

loop {
    match send_request(client, alarm).await {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Alarm '{}' callback succeeded", alarm.id);
            return;
        }
        Ok(resp) => {
            log::warn!("Alarm '{}' callback got status {}", alarm.id, resp.status());
        }
        Err(e) => {
            log::error!("Alarm '{}' callback failed: {}", alarm.id, e);
        }
    }

    // 等待重试，期间监听取消信号
    tokio::select! {
        _ = tokio::time::sleep(retry_interval) => {}
        _ = cancel.cancelled() => {
            log::info!("Alarm '{}' callback retry cancelled", alarm.id);
            return;
        }
    }

    retry_interval = (retry_interval * 2).min(max_interval);
}
```

### 2. reqwest Client 配置

在 main.rs 中创建共享的 reqwest Client，设置超时：

```rust
let http_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .expect("Failed to create HTTP client");
```

### 3. CancellationToken 用途

- 使用 `tokio_util::sync::CancellationToken`（或简单用 `JoinHandle::abort()`）
- Cron 闹钟：调度器记住每个闹钟当前的回调 task handle，下次触发时 abort 前一个
- 一次性闹钟：删除时 abort 正在重试的回调
- 闹钟被用户删除时：abort 对应的回调 task

考虑到简洁性，直接使用 `JoinHandle::abort()` 即可，不额外引入 `tokio_util` 依赖。

## 涉及文件

- `alarm-server/src/callback.rs` — 填充实现
- `alarm-server/src/main.rs` — 可能调整 reqwest Client 创建位置

## 测试

在 `callback.rs` 中编写测试：

1. **测试成功回调**：启动一个本地 mock HTTP server（用 `actix_web::test`），验证回调 POST 到达且 body 内容正确
2. **测试重试**：mock server 前两次返回 500，第三次返回 200，验证重试后最终成功
3. **测试取消**：发起一个会失败的回调，短暂后 abort handle，验证任务退出

```bash
cargo test -p alarm-server -- callback
```

所有测试通过即为成功。
