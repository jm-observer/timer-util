# 项目部署与自动更新方案（通用模板）

本文档整理了一套适用于 Rust 项目的完整部署方案，包含：GitHub CI 编译发布、二进制自更新、systemd 服务安装、版本管理。可直接复用到其他项目。

---

## 1. GitHub Actions 编译发布

### 目标平台

| 平台 | Target Triple | 运行环境 |
|------|--------------|---------|
| Windows x86_64 | `x86_64-pc-windows-msvc` | `windows-latest` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `ubuntu-latest` + 交叉编译 |

### 工作流 `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-pc-windows-msvc
            os: windows-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install ARM64 toolchain
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Add target
        run: rustup target add ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }} --workspace
        env:
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc

      - name: Prepare artifacts
        shell: bash
        run: |
          mkdir -p dist
          # ========== 修改点：填入你的二进制名称 ==========
          BINS="your-server your-cli"
          for bin in $BINS; do
            SRC="target/${{ matrix.target }}/release/${bin}"
            EXT=""
            if [ -f "${SRC}.exe" ]; then
              SRC="${SRC}.exe"
              EXT=".exe"
            fi
            if [ -f "$SRC" ]; then
              cp "$SRC" "dist/${bin}-${{ matrix.target }}${EXT}"
            fi
          done

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: binaries_${{ matrix.target }}
          path: dist/*

  release:
    needs: build
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')

    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          files: dist/*
          generate_release_notes: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### 产物命名规则

```
{binary}-{target}{ext}
```

示例：
- `alarm-server-x86_64-pc-windows-msvc.exe`
- `alarm-server-aarch64-unknown-linux-gnu`
- `alarm-cli-x86_64-pc-windows-msvc.exe`
- `alarm-cli-aarch64-unknown-linux-gnu`

---

## 2. 二进制自更新功能

### 实现要点

CLI 增加 `update` 子命令，流程：

1. 通过 GitHub API 获取最新 release：`GET https://api.github.com/repos/{owner}/{repo}/releases/latest`
2. 对比 `tag_name`（去掉 `v` 前缀）与 `env!("CARGO_PKG_VERSION")`
3. 根据当前编译平台确定 target triple：
   ```rust
   fn current_target() -> &'static str {
       if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
           "x86_64-pc-windows-msvc"
       } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
           "aarch64-unknown-linux-gnu"
       } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
           "x86_64-unknown-linux-gnu"
       } else {
           "unknown"
       }
   }
   ```
4. 拼接 asset 名称 `{bin}-{target}{ext}`，从 release assets 中匹配
5. 下载 `browser_download_url` 获取二进制数据
6. 替换本地文件

### 二进制替换策略（跨平台）

| 平台 | 策略 |
|------|------|
| **Linux** | 写入 `.tmp` 文件 → `chmod 755` → `rename` 原子替换（运行中的进程不受影响） |
| **Windows** | 将当前 exe 重命名为 `.old.exe` → 写入新 exe 到原路径（运行中的 exe 无法直接覆盖） |

### 需要的依赖

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "blocking"] }
serde_json = "1"
```

### CLI 参数设计

```
your-cli update [--force]
```

- 默认：版本相同时跳过
- `--force`：强制重新下载

---

## 3. Systemd 服务安装

### 服务模板 `your-app.service`

```ini
[Unit]
Description=Your App Description
After=network.target

[Service]
Type=simple
User={user}
Group={user}
ExecStart=/usr/local/bin/{binary} -w {workspace}
Restart=on-failure
RestartSec=5
WorkingDirectory={workspace}

[Install]
WantedBy=multi-user.target
```

### CLI `install` 子命令流程

```
your-cli install [--workspace /etc/your-app] [--user your-app] [--dry-run]
```

执行步骤：

1. **检查权限**：`libc::geteuid() == 0`，非 root 提示用 `sudo`
2. **复制二进制**：将同目录下的 server 和 cli 二进制复制到 `/usr/local/bin/`
3. **创建系统用户**：`useradd --system --no-create-home --shell /usr/sbin/nologin {user}`
4. **创建工作目录**：`mkdir -p {workspace}` + `chown {user}:{user} {workspace}`
5. **写入 systemd unit**：写到 `/etc/systemd/system/{app}.service`
6. **启用服务**：`systemctl daemon-reload` + `systemctl enable {app}.service`
7. **提示后续操作**：
   ```
   Start:   sudo systemctl start your-app
   Status:  sudo systemctl status your-app
   Logs:    sudo journalctl -u your-app -f
   ```

### 需要的依赖（仅 Linux）

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

---

## 4. 版本管理

### 推荐：workspace 统一版本

```toml
# 根 Cargo.toml
[workspace.package]
version = "0.7.0"

# 子 crate Cargo.toml
[package]
version.workspace = true
```

### 版本发布脚本

**Bash（`scripts/bump-version.sh`）：**

```bash
#!/usr/bin/env bash
set -euo pipefail
NEW_VERSION="$1"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"

CURRENT=$(grep -m1 '^version' "$CARGO_TOML" | sed 's/.*"\(.*\)".*/\1/')
sed -i "s/^version = \"$CURRENT\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

cd "$REPO_ROOT"
cargo check --workspace
git add Cargo.toml
git commit -m "chore: bump version to v$NEW_VERSION"
git tag "v$NEW_VERSION"
echo "Done! Run: git push && git push --tags"
```

**PowerShell（`scripts/bump-version.ps1`）：**

```powershell
param([Parameter(Mandatory)][string]$NewVersion)
$CargoToml = Join-Path (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)) "Cargo.toml"
$content = Get-Content $CargoToml -Raw
$content -match 'version\s*=\s*"(\d+\.\d+\.\d+)"' | Out-Null
$content = $content -replace "version = `"$($Matches[1])`"", "version = `"$NewVersion`""
Set-Content -Path $CargoToml -Value $content -NoNewline
cargo check --workspace
git add Cargo.toml
git commit -m "chore: bump version to v$NewVersion"
git tag "v$NewVersion"
Write-Host "Done! Run: git push && git push --tags"
```

### 发布流程

```bash
# 1. 改版本 + 打 tag
./scripts/bump-version.sh 0.8.0

# 2. 推送触发 CI 构建
git push && git push --tags

# 3. GitHub Actions 自动构建并创建 Release
```

---

## 5. 项目接入清单

在新项目中接入此方案的步骤：

- [ ] 根 Cargo.toml 使用 `workspace.package.version`
- [ ] 复制 `.github/workflows/release.yml`，修改 `BINS` 列表
- [ ] CLI 添加 `update` 子命令，修改 `GITHUB_REPO` 常量
- [ ] CLI 添加 `install` 子命令，修改服务描述和默认参数
- [ ] 添加 `your-app.service` 模板文件
- [ ] 复制 `scripts/bump-version.sh` 和 `.ps1`
- [ ] Cargo.toml 添加 `libc` 依赖（cfg(unix)）
- [ ] 确认 `reqwest` 依赖包含 `blocking` feature
