# Plan 5: HTTP 路由 + 主程序组装

## 目标

实现所有 HTTP 路由处理函数，组装 main.rs 启动完整服务，完成端到端可用的闹钟服务。

## 前置依赖

Plan 1-4 全部完成。

## 实现方式

### 1. handlers.rs — 路由处理函数

所有 handler 共享的 app state：
- `web::Data<Database>` — 数据库
- `web::Data<mpsc::Sender<SchedulerCommand>>` — 调度器通信

**create_alarm — POST /api/alarms**

```rust
pub async fn create_alarm(
    body: web::Json<CreateAlarmRequest>,
    db: web::Data<Database>,
    tx: web::Data<mpsc::Sender<SchedulerCommand>>,
) -> Result<HttpResponse, AppError>
```

逻辑：
1. 校验 alarm_type（"cron" 或 "once"）
2. cron 类型：验证 `cron_expr` 存在且 `TimerConf::from_cron()` 合法
3. once 类型：验证 `once_at` 存在、格式正确、是未来时间
4. 生成 UUID，构造 AlarmRecord
5. `web::block` 调用 `db.insert_alarm()`
6. 发送 `SchedulerCommand::Reload` 通知调度器
7. 计算 `next_fire_at`，构造 AlarmResponse，返回 201

**list_alarms — GET /api/alarms**

```rust
pub async fn list_alarms(
    query: web::Query<ListQuery>,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError>
```

逻辑：
1. 从 DB 查询闹钟列表（可选 status 过滤）
2. 对每条记录计算 `next_fire_at`
3. 返回 `AlarmListResponse`

**get_alarm — GET /api/alarms/{id}**

```rust
pub async fn get_alarm(
    path: web::Path<String>,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError>
```

逻辑：
1. 从 DB 查询，不存在返回 404
2. 计算 `next_fire_at`，返回 AlarmResponse

**delete_alarm — DELETE /api/alarms/{id}**

```rust
pub async fn delete_alarm(
    path: web::Path<String>,
    db: web::Data<Database>,
    tx: web::Data<mpsc::Sender<SchedulerCommand>>,
) -> Result<HttpResponse, AppError>
```

逻辑：
1. `web::block` 调用 `db.delete_alarm()`，不存在返回 404
2. 发送 `SchedulerCommand::Reload` 通知调度器
3. 返回 204

**health — GET /api/health**

```rust
pub async fn health(db: web::Data<Database>) -> Result<HttpResponse, AppError>
```

返回 `{"status": "ok", "active_alarms": N}`。

### 2. main.rs — 完整组装

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    // 1. 加载配置
    let config = Config::from_env();

    // 2. 初始化数据库
    let db = Database::new(&config.db_path).expect("Failed to open database");
    db.initialize();

    // 3. 启动恢复：过期一次性闹钟标记 completed
    recover_expired_alarms(&db);

    // 4. 创建调度器通信 channel
    let (tx, rx) = tokio::sync::mpsc::channel::<SchedulerCommand>(256);

    // 5. 创建 HTTP client
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // 6. 启动调度器
    let scheduler = Scheduler::new(rx, Arc::new(db.clone()), http_client);
    tokio::spawn(scheduler.run());

    // 7. 启动 HTTP 服务
    let db_data = web::Data::new(db);
    let tx_data = web::Data::new(tx);

    log::info!("alarm-server listening on 0.0.0.0:{}", config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(db_data.clone())
            .app_data(tx_data.clone())
            .route("/api/health", web::get().to(handlers::health))
            .route("/api/alarms", web::post().to(handlers::create_alarm))
            .route("/api/alarms", web::get().to(handlers::list_alarms))
            .route("/api/alarms/{id}", web::get().to(handlers::get_alarm))
            .route("/api/alarms/{id}", web::delete().to(handlers::delete_alarm))
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
```

## 涉及文件

- `alarm-server/src/handlers.rs` — 填充所有路由处理函数
- `alarm-server/src/main.rs` — 完整重写为最终版本

## 测试

### 编译测试
```bash
cargo build -p alarm-server
```

### 端到端测试（curl）

启动服务：
```bash
RUST_LOG=info cargo run -p alarm-server
```

1. **健康检查**：
```bash
curl http://localhost:8080/api/health
# 期望: {"status":"ok","active_alarms":0}
```

2. **创建 Cron 闹钟**（每 5 秒触发）：
```bash
curl -X POST http://localhost:8080/api/alarms \
  -H "Content-Type: application/json" \
  -d '{"name":"test-cron","alarm_type":"cron","cron_expr":"*/5 * * * * *","callback_url":"https://webhook.site/YOUR-ID","callback_body":{"msg":"hello"}}'
# 期望: 201，响应含 next_fire_at
```

3. **创建一次性闹钟**（10 秒后）：
```bash
curl -X POST http://localhost:8080/api/alarms \
  -H "Content-Type: application/json" \
  -d '{"name":"test-once","alarm_type":"once","once_at":"2026-04-01T00:00:10","callback_url":"https://webhook.site/YOUR-ID","callback_body":{"msg":"one-time"}}'
# 期望: 201
```

4. **查询列表**：
```bash
curl http://localhost:8080/api/alarms
curl http://localhost:8080/api/alarms?status=active
```

5. **查询单个**：
```bash
curl http://localhost:8080/api/alarms/{id}
```

6. **删除**：
```bash
curl -X DELETE http://localhost:8080/api/alarms/{id}
# 期望: 204，闹钟不再触发
```

7. **回调验证**：在 webhook.site 或本地 mock server 观察：
   - POST 请求到达
   - Header 含 X-Alarm-Id、X-Alarm-Name
   - Body 为创建时传入的 callback_body 原样

8. **重启恢复**：停止服务后重新启动，验证之前创建的 active 闹钟恢复调度

9. **回调重试**：创建闹钟指向不可达 URL，观察日志中的重试记录和退避间隔
