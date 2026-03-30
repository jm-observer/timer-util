# Alarm Server 控制面板实现计划

## Context

alarm-server 当前只有 REST API，没有可视化界面。用户需要一个 Web 控制面板来直观查看闹钟状态、回调通知记录等信息。此外，当前的 callback 执行没有日志记录，需要新增 `notification_logs` 表来持久化回调历史。

## 变更概览

1. 新增 `notification_logs` 数据库表，记录每次回调尝试
2. 修改 `callback.rs`，在回调执行过程中写入日志
3. 新增 Dashboard API 端点（统计数据、通知列表）
4. 新增嵌入式 HTML 控制面板页面（`GET /`）

---

## Phase 1: 数据模型 & 数据库层

### 新增数据库表

```sql
CREATE TABLE IF NOT EXISTS notification_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    alarm_id TEXT NOT NULL,
    alarm_name TEXT NOT NULL DEFAULT '',
    callback_url TEXT NOT NULL,
    status TEXT NOT NULL,          -- "success" / "failed" / "retrying" / "cancelled"
    http_status INTEGER,           -- HTTP 状态码，网络错误时为 NULL
    error_message TEXT,
    attempt INTEGER NOT NULL DEFAULT 1,
    triggered_at TEXT NOT NULL,    -- 触发时间（同一次触发的所有重试共享）
    completed_at TEXT NOT NULL     -- 本次尝试完成时间
);
CREATE INDEX IF NOT EXISTS idx_notification_logs_alarm_id ON notification_logs(alarm_id);
CREATE INDEX IF NOT EXISTS idx_notification_logs_triggered_at ON notification_logs(triggered_at);
```

> `alarm_name` 冗余存储，这样即使闹钟被删除，日志仍可读。

### 修改文件：`src/models.rs`

新增结构体：
- `NotificationLog` — 通知日志记录（Serialize + Deserialize）
- `NotificationStats` — 聚合统计（total/success/failed）
- `DashboardStatsResponse` — Dashboard 统计 API 返回值
- `NotificationListResponse` — 分页通知列表返回值
- `NotificationListQuery` — 通知列表查询参数（page/per_page/status/alarm_id）

### 修改文件：`src/db.rs`

- `initialize()` 中新增 `notification_logs` 表和索引的创建
- 新增 `insert_notification_log(&self, log: &NotificationLog)` 方法
- 新增 `list_notification_logs(&self, alarm_id, status, page, per_page) -> (Vec<NotificationLog>, usize)` 方法（带分页和总数）
- 新增 `notification_stats(&self) -> NotificationStats` 方法

---

## Phase 2: 回调日志集成

### 修改文件：`src/callback.rs`

修改 `fire_callback` 签名，增加 `db: Arc<Database>` 参数：

```rust
pub async fn fire_callback(
    client: &Client,
    alarm: &AlarmRecord,
    cancel: CancellationToken,
    db: Arc<Database>,
)
```

在重试循环中：
- 进入函数时记录 `triggered_at` 时间戳，`attempt` 从 1 开始
- 成功（2xx）：写入 `status = "success"` 日志，break
- 非 2xx 响应：写入 `status = "retrying"` 日志，含 http_status
- 网络错误：写入 `status = "retrying"` 日志，含 error_message
- 取消：写入 `status = "cancelled"` 日志，break
- 每次循环 `attempt += 1`

### 修改文件：`src/scheduler.rs`

`trigger_alarm` 方法中，将 `self.db.clone()` 传递给 `fire_callback`：

```rust
// 当前（第70行）：
fire_callback(&client, &alarm_clone, cancel_token).await;
// 改为：
fire_callback(&client, &alarm_clone, cancel_token, db.clone()).await;
```

其中 `db` 已经在 `trigger_alarm` 第64行以 `let db = self.db.clone();` 获取。

---

## Phase 3: Dashboard API 端点

### 修改文件：`src/handlers.rs`

新增处理函数：

1. **`dashboard_page()`** — `GET /`
   - 返回 `Content-Type: text/html`，内容为嵌入的 HTML 控制面板

2. **`dashboard_stats()`** — `GET /api/dashboard/stats`
   - 返回 JSON：
     ```json
     {
       "total_alarms": 12,
       "active_alarms": 8,
       "completed_alarms": 4,
       "total_notifications": 156,
       "successful_notifications": 140,
       "failed_notifications": 16,
       "recent_notifications": [/* 最近5条 */]
     }
     ```

3. **`list_notifications()`** — `GET /api/notifications`
   - 查询参数：`?page=1&per_page=20&status=success&alarm_id=xxx`
   - 返回分页列表

在 `init_routes` 中注册新路由：
```rust
cfg.service(web::resource("/").route(web::get().to(dashboard_page)))
   .service(web::resource("/api/dashboard/stats").route(web::get().to(dashboard_stats)))
   .service(web::resource("/api/notifications").route(web::get().to(list_notifications)))
```

---

## Phase 4: 控制面板 UI

### 新增文件：`src/dashboard.rs`

包含 `pub const DASHBOARD_HTML: &str = r#"..."#;`，一个完整的 HTML 页面。

**UI 布局**：

```
+----------------------------------------------------------+
| Alarm Server Dashboard                       [刷新按钮]   |
+----------------------------------------------------------+
| 统计卡片区域                                               |
| [总闹钟: 12] [活跃: 8] [已完成: 4]                         |
| [总通知: 156] [成功: 140] [失败: 16]                       |
+----------------------------------------------------------+
| 闹钟列表                                                   |
| | 名称 | 类型 | 状态 | 下次触发 | 回调URL | 操作 |        |
+----------------------------------------------------------+
| 通知记录                              [状态筛选下拉框]      |
| | 时间 | 闹钟 | 状态 | HTTP码 | 重试次数 | 错误信息 |     |
| [上一页]  第 1/N 页  [下一页]                               |
+----------------------------------------------------------+
```

**技术选择**：
- Simple.css CDN（`https://cdn.simplecss.org/simple.min.css`）— 零配置美化语义 HTML
- 原生 `fetch()` + DOM 操作，无框架依赖
- 每 30 秒自动刷新
- 状态颜色标识：绿色=success/active，红色=failed，黄色=retrying，灰色=completed/cancelled

### 修改文件：`src/main.rs`

添加 `mod dashboard;` 声明。

---

## 文件变更汇总

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/models.rs` | 修改 | 新增 NotificationLog 等结构体 |
| `src/db.rs` | 修改 | 新增表、索引、CRUD 方法 |
| `src/callback.rs` | 修改 | fire_callback 增加 db 参数，写入日志 |
| `src/scheduler.rs` | 修改 | 传递 db 给 fire_callback |
| `src/handlers.rs` | 修改 | 新增 3 个路由处理函数 |
| `src/dashboard.rs` | **新增** | HTML 控制面板模板 |
| `src/main.rs` | 修改 | 添加 mod dashboard |

---

## 验证方式

1. `cargo build` — 确保编译通过
2. `cargo test` — 确保现有测试和新增 DB 测试通过
3. 启动服务后访问 `http://localhost:8080/` — 查看控制面板页面
4. 创建几个测试闹钟，验证统计数据和通知记录是否正确显示
5. 验证分页和筛选功能正常工作
