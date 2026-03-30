# Alarm Server 设计文档

## 概述

基于 timer-util 库构建的 HTTP 闹钟服务。支持通过 HTTP 接口新增、查询、删除闹钟，时间到期后请求回调接口并透传闹钟信息，程序常驻运行。

## 技术选型

| 组件 | 选择 | 说明 |
|------|------|------|
| 定时计算 | timer-util（本库） | `TimerConf::from_cron()` + `next_with_time()` |
| HTTP 框架 | actix-web 4 | 高性能 Web 框架 |
| 异步运行时 | tokio | rt-multi-thread |
| 数据库 | SQLite（rusqlite） | bundled 模式，无外部依赖 |
| HTTP 客户端 | reqwest | 用于回调请求 |
| 序列化 | serde + serde_json | JSON 处理 |
| ID 生成 | uuid v4 | 闹钟唯一标识 |
| 日志 | log + env_logger | 标准日志方案 |

## 项目结构

独立二进制 crate，通过 Cargo workspace 与 timer-util 共存。

```
timer-util/             （workspace root）
├── Cargo.toml          （添加 [workspace] 配置）
├── src/                （timer-util 库源码）
├── alarm-server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          入口：初始化日志、DB、调度器、HTTP 服务
│       ├── config.rs        环境变量配置（端口、数据库路径）
│       ├── db.rs            SQLite 操作（CRUD）
│       ├── models.rs        数据结构定义（请求/响应/记录）
│       ├── handlers.rs      actix-web 路由处理函数
│       ├── scheduler.rs     统一调度循环
│       ├── callback.rs      回调触发 + 重试逻辑
│       └── error.rs         统一错误类型
└── doc/
```

## 数据库设计

文件路径默认 `./alarms.db`，可通过 `ALARM_DB_PATH` 环境变量配置。

```sql
CREATE TABLE IF NOT EXISTS alarms (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL DEFAULT '',
    alarm_type    TEXT NOT NULL,             -- "cron" 或 "once"
    cron_expr     TEXT,                      -- cron 表达式，once 类型为 NULL
    once_at       TEXT,                      -- ISO 8601 时间，cron 类型为 NULL
    callback_url  TEXT NOT NULL,
    callback_body TEXT,                      -- 用户自定义 JSON，触发时原样透传
    status        TEXT NOT NULL DEFAULT 'active',  -- active / completed
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
```

字段说明：
- `alarm_type`：区分 `"cron"`（周期性）和 `"once"`（一次性）
- `cron_expr`：存储原始 cron 字符串，启动时通过 `TimerConf::from_cron()` 重新解析
- `once_at`：NaiveDateTime 格式的 ISO 8601 文本
- `callback_body`：用户创建时传入的任意 JSON 字符串，触发时原样透传
- `status`：`active` 表示活跃，`completed` 表示一次性闹钟已触发完成

## HTTP API

默认端口 `8080`，可通过 `ALARM_SERVER_PORT` 环境变量配置。

### 接口列表

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/alarms` | 新增闹钟 |
| GET | `/api/alarms` | 查询闹钟列表（可选 `?status=active` 过滤） |
| GET | `/api/alarms/{id}` | 查询单个闹钟 |
| DELETE | `/api/alarms/{id}` | 删除闹钟 |
| GET | `/api/health` | 健康检查 |

### POST /api/alarms — 新增闹钟

**Cron 类型请求体：**
```json
{
    "name": "每日提醒",
    "alarm_type": "cron",
    "cron_expr": "0 0 9 * * 1-5",
    "callback_url": "https://example.com/notify",
    "callback_body": {"message": "起床了", "channel": "#general"}
}
```

**一次性类型请求体：**
```json
{
    "name": "部署提醒",
    "alarm_type": "once",
    "once_at": "2026-04-01T14:30:00",
    "callback_url": "https://example.com/deploy",
    "callback_body": {"task_id": 42}
}
```

**校验规则：**
- cron 类型必须有 `cron_expr`，通过 `TimerConf::from_cron()` 验证合法性
- once 类型必须有 `once_at`，且必须是未来时间
- `callback_url` 不能为空

**成功响应（201 Created）：**
```json
{
    "id": "a1b2c3d4-...",
    "name": "每日提醒",
    "alarm_type": "cron",
    "cron_expr": "0 0 9 * * 1-5",
    "callback_url": "https://example.com/notify",
    "callback_body": {"message": "起床了", "channel": "#general"},
    "status": "active",
    "next_fire_at": "2026-03-31T09:00:00",
    "created_at": "2026-03-30T10:00:00"
}
```

`next_fire_at` 为实时计算的下次触发时间。查询接口同样返回此字段。

**错误响应（400 Bad Request）：**
```json
{
    "error": "invalid cron expression: ..."
}
```

### GET /api/alarms — 查询列表

可选查询参数 `?status=active` 过滤。

响应：
```json
{
    "alarms": [ /* 闹钟对象数组，含 next_fire_at */ ],
    "total": 42
}
```

### GET /api/alarms/{id} — 查询单个

成功返回 200 + 闹钟对象（含 `next_fire_at`），不存在返回 404。

### DELETE /api/alarms/{id} — 删除

成功返回 204，不存在返回 404。删除后调度器取消对应闹钟。

### GET /api/health — 健康检查

```json
{"status": "ok", "active_alarms": 5}
```

## 核心调度架构 — 统一调度循环

采用单一调度循环管理所有闹钟，避免每个闹钟独立 task 可能出现的 task 卡住/panic/丢失问题。

### 调度器通信

HTTP handler 通过 mpsc channel 发送指令通知调度器数据变更：

```rust
enum SchedulerCommand {
    Reload,     // 数据有变更，重新加载
    Shutdown,   // 关闭
}
```

### 调度循环逻辑

```
loop {
    从 DB 加载所有 active 闹钟
    对每个闹钟计算 next_fire_at：
      - cron: TimerConf::from_cron(expr).next_with_time(now)
      - once: 直接用 once_at
    找出最近要触发的时间点 nearest
    cap sleep 最长 1 小时（防止长时间 sleep 漂移）

    tokio::select! {
        _ = sleep(nearest - now) => {
            收集所有到期的闹钟
            对每个到期闹钟 spawn 短期 task 执行回调：
              - cron: 取消该闹钟之前的重试 task（如有），再 spawn 新回调
              - once: spawn 回调，成功后更新 DB status = "completed"
        }
        cmd = rx.recv() => {
            Reload: 跳回循环顶部重新计算
            Shutdown: 退出
        }
    }
}
```

### 启动恢复

程序启动时从 DB 加载所有 `status = 'active'` 的闹钟。过期的一次性闹钟标记为 `completed`。

## 回调机制

### 回调内容

触发时向 `callback_url` 发送 POST 请求：
- `Content-Type: application/json`
- `X-Alarm-Id: {id}`
- `X-Alarm-Name: {name}`
- Body：`callback_body` 原样透传（用户创建时传入的任意 JSON）

### 重试策略

- 回调失败（网络错误或非 2xx 响应）时记录日志并重试
- 重试间隔指数退避：5s → 10s → 20s → 40s → ... 上限 10 分钟
- **持续重试，不放弃**
- **Cron 闹钟特殊处理**：下次到期触发时，取消前次仍在重试的回调 task，开始新的回调。每个 cron 闹钟同时最多一个回调在执行/重试
- 实现：每个 cron 闹钟的回调 task 用 `JoinHandle` 跟踪，下次触发时 `abort()` 前一个

### reqwest 超时

给 reqwest Client 设置请求超时 30s，避免回调目标无响应时无限等待。
