# Plan 2: 基础类型 + 数据库层

## 目标

实现 config、models、error、db 四个模块，完成 SQLite 建表和完整的 CRUD 操作。

## 前置依赖

Plan 1 完成（workspace 和 crate 骨架已就位）。

## 实现方式

### 1. config.rs — 环境变量配置

从环境变量读取配置，提供默认值：

```rust
pub struct Config {
    pub port: u16,           // ALARM_SERVER_PORT，默认 8080
    pub db_path: String,     // ALARM_DB_PATH，默认 "./alarms.db"
}
```

实现 `Config::from_env()` 方法。

### 2. error.rs — 统一错误类型

```rust
pub enum AppError {
    Validation(String),    // 400
    NotFound(String),      // 404
    Internal(String),      // 500
}
```

实现 `actix_web::ResponseError`，返回 JSON 格式 `{"error": "..."}` 响应。
实现 `From<rusqlite::Error>` 等常用转换。

### 3. models.rs — 数据结构

**请求结构体：**
```rust
#[derive(Deserialize)]
pub struct CreateAlarmRequest {
    pub name: Option<String>,
    pub alarm_type: String,              // "cron" 或 "once"
    pub cron_expr: Option<String>,
    pub once_at: Option<String>,         // ISO 8601 NaiveDateTime
    pub callback_url: String,
    pub callback_body: Option<serde_json::Value>,
}
```

**数据库记录结构体：**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmRecord {
    pub id: String,
    pub name: String,
    pub alarm_type: String,
    pub cron_expr: Option<String>,
    pub once_at: Option<String>,
    pub callback_url: String,
    pub callback_body: Option<String>,   // JSON 字符串
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
```

**响应结构体：**
```rust
#[derive(Serialize)]
pub struct AlarmResponse {
    // AlarmRecord 的所有字段 +
    pub next_fire_at: Option<String>,    // 实时计算
}

#[derive(Serialize)]
pub struct AlarmListResponse {
    pub alarms: Vec<AlarmResponse>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}
```

`next_fire_at` 的计算逻辑：
- cron 类型：`TimerConf::from_cron(expr).next_with_time(Local::now().naive_local())`
- once 类型：如果 `once_at` 在未来则返回 `once_at`，否则为 None
- status 为 completed 的闹钟返回 None

### 4. db.rs — SQLite 数据库操作

使用 `rusqlite`，连接用 `Arc<Mutex<Connection>>` 包装。

**核心方法：**

```rust
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self>;
    pub fn initialize(&self);                              // CREATE TABLE IF NOT EXISTS
    pub fn insert_alarm(&self, alarm: &AlarmRecord) -> Result<()>;
    pub fn get_alarm(&self, id: &str) -> Result<Option<AlarmRecord>>;
    pub fn list_alarms(&self, status: Option<&str>) -> Result<Vec<AlarmRecord>>;
    pub fn delete_alarm(&self, id: &str) -> Result<bool>;  // 返回是否存在
    pub fn update_status(&self, id: &str, status: &str) -> Result<bool>;
    pub fn count_by_status(&self, status: &str) -> Result<usize>;
}
```

建表 SQL 见 `alarm-server-design.md` 数据库设计章节。

在 async handler 中调用时，使用 `actix_web::web::block()` 包装（内部使用 `spawn_blocking`）。

## 涉及文件

- `alarm-server/src/config.rs` — 填充实现
- `alarm-server/src/error.rs` — 填充实现
- `alarm-server/src/models.rs` — 填充实现
- `alarm-server/src/db.rs` — 填充实现
- `alarm-server/src/main.rs` — 添加 `mod` 声明

## 测试

在 `db.rs` 中编写 `#[cfg(test)] mod tests`：

1. **测试建表**：创建内存数据库（`:memory:`），调用 `initialize()`，验证不报错
2. **测试插入和查询**：插入一条闹钟记录，通过 `get_alarm` 查回，验证字段一致
3. **测试列表过滤**：插入 active 和 completed 状态的闹钟，验证 `list_alarms(Some("active"))` 只返回 active 的
4. **测试删除**：插入后删除，验证 `get_alarm` 返回 None
5. **测试状态更新**：插入后调用 `update_status`，验证状态已变更

```bash
cargo test -p alarm-server
```

所有测试通过即为成功。
