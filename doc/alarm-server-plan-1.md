# Plan 1: Workspace 改造 + Crate 骨架

## 目标

建立 Cargo workspace 结构，创建 alarm-server 二进制 crate，配好所有依赖，确保项目能编译通过。

## 实现方式

### 1. 修改根 Cargo.toml

在 `D:\git\timer-util\Cargo.toml` 顶部添加 workspace 配置：

```toml
[workspace]
members = [".", "alarm-server"]
```

保持原有 `[package]`、`[dependencies]` 等内容不变。

### 2. 创建 alarm-server 目录和 Cargo.toml

新建 `alarm-server/Cargo.toml`：

```toml
[package]
name = "alarm-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "alarm-server"
path = "src/main.rs"

[dependencies]
timer-util = { path = "..", features = ["serde"] }
actix-web = "4"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
log = "0.4"
env_logger = "0.11"
```

### 3. 创建 main.rs 空壳

新建 `alarm-server/src/main.rs`，内含最简的 actix-web 启动代码：

```rust
use actix_web::{web, App, HttpServer, HttpResponse};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    log::info!("alarm-server starting...");

    HttpServer::new(|| {
        App::new()
            .route("/api/health", web::get().to(|| async {
                HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
            }))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
```

### 4. 创建空模块文件

创建以下空文件（仅含模块声明占位），后续 plan 逐步填充：
- `alarm-server/src/config.rs`
- `alarm-server/src/db.rs`
- `alarm-server/src/models.rs`
- `alarm-server/src/handlers.rs`
- `alarm-server/src/scheduler.rs`
- `alarm-server/src/callback.rs`
- `alarm-server/src/error.rs`

## 涉及文件

- `D:\git\timer-util\Cargo.toml` — 修改（添加 workspace）
- `alarm-server/Cargo.toml` — 新建
- `alarm-server/src/main.rs` — 新建
- `alarm-server/src/*.rs` — 新建空模块

## 测试

```bash
cargo build -p alarm-server
```

编译通过即为成功。可选：`cargo run -p alarm-server` 启动后 `curl http://localhost:8080/api/health` 验证返回 `{"status":"ok"}`。
