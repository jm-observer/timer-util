# deploy-utils crate 设计文档

一个基于 `self_update` 的 Rust 工具 crate，为 CLI 项目提供 **GitHub 自更新** + **systemd 服务安装** 能力。

---

## 依赖

```toml
[dependencies]
self_update = { version = "0.42", features = ["archive-zip", "archive-tar"] }
libc = { version = "0.2", optional = true }

[features]
default = ["systemd"]
systemd = ["dep:libc"]
```

---

## 模块结构

```
deploy-utils/
├── src/
│   ├── lib.rs          # 公开 API 重导出
│   ├── update.rs       # GitHub Release 自更新（封装 self_update）
│   └── systemd.rs      # systemd 服务安装（feature = "systemd"）
└── Cargo.toml
```

---

## API 设计

### 1. 自更新 `update.rs`

```rust
pub struct UpdateConfig {
    pub repo_owner: String,
    pub repo_name: String,
    pub bin_name: String,           // 当前二进制名称
    pub extra_bins: Vec<String>,    // 同目录下一起更新的其他二进制
    pub current_version: String,    // 传入 env!("CARGO_PKG_VERSION")
    pub force: bool,
}

impl UpdateConfig {
    pub fn new(repo_owner: &str, repo_name: &str, current_version: &str) -> Self;
    pub fn bin_name(mut self, name: &str) -> Self;
    pub fn extra_bins(mut self, bins: &[&str]) -> Self;
    pub fn force(mut self, force: bool) -> Self;
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;
}
```

#### 使用方式

```rust
deploy_utils::UpdateConfig::new(
    "jm-observer", "timer-util",
    env!("CARGO_PKG_VERSION"),
)
.bin_name("alarm-cli")
.extra_bins(&["alarm-server"])
.force(args.force)
.execute()?;
```

#### 内部实现要点

```rust
pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 用 self_update 更新主二进制（alarm-cli）
    let status = self_update::backends::github::Update::configure()
        .repo_owner(&self.repo_owner)
        .repo_name(&self.repo_name)
        .bin_name(&self.bin_name)
        .current_version(&self.current_version)
        .no_confirm(true)
        .show_download_progress(true)
        // asset 命名规则：{bin}-{target}{ext}
        .target_version_tag(&format!("v{}", latest))
        .build()?
        .update()?;

    // 2. 更新 extra_bins
    //    self_update 只处理自身替换，extra_bins 需手动下载：
    //    - 拼接 asset 名称：{bin}-{target}{ext}
    //    - 从 release assets 中找到 browser_download_url
    //    - 下载到同目录，使用跨平台替换策略
    for bin in &self.extra_bins {
        self.update_extra_bin(bin, &release)?;
    }

    Ok(())
}
```

**注意**：`self_update` 已内置处理：
- GitHub API 请求 + 版本比较
- 跨平台二进制替换（Windows rename 策略）
- 下载进度条
- 解压（tar.gz / zip）

如果 release 产物是裸二进制（非压缩包），需设置 `self_update` 的 `identifier` 或自行处理 extra_bins 的下载替换。

---

### 2. Systemd 安装 `systemd.rs`

```rust
pub struct ServiceConfig {
    pub name: String,               // 服务名，如 "alarm-server"
    pub description: String,        // systemd Description
    pub exec_args: String,          // ExecStart 参数部分，如 "-w {workspace}"
    pub binaries: Vec<String>,      // 要安装到 /usr/local/bin/ 的二进制列表
    pub user: String,               // 运行用户，默认与 name 相同
    pub workspace: String,          // 工作目录，默认 /etc/{name}
    pub restart_sec: u32,           // 重启间隔，默认 5
}

impl ServiceConfig {
    pub fn new(name: &str) -> Self;
    pub fn description(mut self, desc: &str) -> Self;
    pub fn exec_args(mut self, args: &str) -> Self;
    pub fn binaries(mut self, bins: &[&str]) -> Self;
    pub fn user(mut self, user: &str) -> Self;
    pub fn workspace(mut self, path: &str) -> Self;

    /// 生成 systemd unit 内容（可用于 --dry-run 预览）
    pub fn generate_unit(&self) -> String;

    /// 执行完整安装流程，需要 root 权限
    pub fn install(&self) -> Result<(), Box<dyn std::error::Error>>;
}
```

#### 使用方式

```rust
let service = deploy_utils::ServiceConfig::new("alarm-server")
    .description("Alarm Server - Recurring alarm scheduler")
    .exec_args("-w {workspace}")
    .binaries(&["alarm-server", "alarm-cli"]);

if dry_run {
    println!("{}", service.generate_unit());
} else {
    service
        .user(&args.user)
        .workspace(&args.workspace)
        .install()?;
}
```

#### `install()` 内部流程

```rust
pub fn install(&self) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 检查 root
    #[cfg(unix)]
    if unsafe { libc::geteuid() } != 0 {
        return Err("requires root, run with sudo".into());
    }

    // 2. 复制二进制到 /usr/local/bin/
    let src_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    for bin in &self.binaries {
        std::fs::copy(src_dir.join(bin), format!("/usr/local/bin/{}", bin))?;
        // chmod 755
    }

    // 3. 创建系统用户（如不存在）
    //    useradd --system --no-create-home --shell /usr/sbin/nologin {user}

    // 4. 创建工作目录 + chown
    //    mkdir -p {workspace} && chown {user}:{user} {workspace}

    // 5. 写入 /etc/systemd/system/{name}.service
    std::fs::write(
        format!("/etc/systemd/system/{}.service", self.name),
        self.generate_unit(),
    )?;

    // 6. systemctl daemon-reload && systemctl enable {name}

    Ok(())
}
```

#### 生成的 unit 模板

```ini
[Unit]
Description={description}
After=network.target

[Service]
Type=simple
User={user}
Group={user}
ExecStart=/usr/local/bin/{name} {exec_args}   # {workspace} 被实际路径替换
Restart=on-failure
RestartSec={restart_sec}
WorkingDirectory={workspace}

[Install]
WantedBy=multi-user.target
```

---

## 3. 接入方使用示例（完整）

```rust
// 项目的 cli.rs

#[derive(Subcommand)]
enum Commands {
    // ... 业务命令 ...

    /// 更新到最新版本
    Update {
        #[arg(long)]
        force: bool,
    },
    /// 安装为 systemd 服务
    Install {
        #[arg(long, default_value = "/etc/alarm-server")]
        workspace: String,
        #[arg(long, default_value = "alarm-server")]
        user: String,
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    match cli.command {
        Commands::Update { force } => {
            deploy_utils::UpdateConfig::new(
                "jm-observer", "timer-util",
                env!("CARGO_PKG_VERSION"),
            )
            .bin_name("alarm-cli")
            .extra_bins(&["alarm-server"])
            .force(force)
            .execute()
            .unwrap();
        }
        Commands::Install { workspace, user, dry_run } => {
            let svc = deploy_utils::ServiceConfig::new("alarm-server")
                .description("Alarm Server")
                .exec_args("-w {workspace}")
                .binaries(&["alarm-server", "alarm-cli"])
                .user(&user)
                .workspace(&workspace);

            if dry_run {
                println!("{}", svc.generate_unit());
            } else {
                svc.install().unwrap();
            }
        }
    }
}
```

---

## 4. 接入项目的 Cargo.toml

```toml
[dependencies]
deploy-utils = { version = "0.1", features = ["systemd"] }
# 或 git 依赖
# deploy-utils = { git = "https://github.com/jm-observer/deploy-utils" }
```

Windows 项目只用自更新、不需要 systemd：

```toml
[dependencies]
deploy-utils = { version = "0.1", default-features = false }
```

---

## 5. Release 产物命名约定

crate 默认假设 GitHub Release 产物按以下规则命名：

```
{bin_name}-{target}{ext}
```

| 产物示例 | 说明 |
|---------|------|
| `alarm-cli-x86_64-pc-windows-msvc.exe` | Windows x86_64 |
| `alarm-cli-aarch64-unknown-linux-gnu` | Linux ARM64 |
| `alarm-server-x86_64-pc-windows-msvc.exe` | Windows x86_64 |
| `alarm-server-aarch64-unknown-linux-gnu` | Linux ARM64 |

接入项目的 `.github/workflows/release.yml` 需保持一致（参考 deploy-guide.md）。
