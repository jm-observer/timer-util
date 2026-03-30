# Plan 1: CLI 范围与命令结构

## 目标

明确 CLI 在 `alarm-server` crate 内的定位、命令边界和整体结构，避免在实现阶段继续扩张范围。

## 范围

本 plan 只定义 CLI 的顶层命令结构与职责边界，不涉及具体参数细节。

## 需求

### 1. 放置位置

- CLI 放在 `alarm-server` crate 内
- 不新增独立 crate
- CLI 与服务端运行入口属于同一项目交付物

### 2. 能力边界

首期仅支持以下能力：

- 新增一次性闹钟
- 新增 cron 闹钟
- 查询闹钟列表
- 查询单个闹钟

明确不支持：

- 删除闹钟
- 更新闹钟
- 暂停、恢复闹钟

### 3. 顶层命令结构

建议命令结构：

```bash
alarm-cli create once ...
alarm-cli create cron ...
alarm-cli list ...
alarm-cli get <id>
```

### 4. 设计原则

- CLI 只是现有 HTTP API 的封装层
- 不改变现有 API 语义
- 不引入本地状态
- 不额外解释业务字段

## 验收

后续设计与实现不得超出本 plan 的范围定义。
