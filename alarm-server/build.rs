use std::process::Command;

/// 编译期把当前 git short commit 嵌进 `GIT_COMMIT`。取不到（非 git 仓 /
/// 无 git / 命令失败）则兜底 "unknown"，绝不 panic。
fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_COMMIT={commit}");
    // HEAD 变了就重跑，让 commit 跟随。
    println!("cargo:rerun-if-changed=../.git/HEAD");
}
