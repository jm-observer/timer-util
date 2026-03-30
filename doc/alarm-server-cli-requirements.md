# Alarm Server CLI 需求文档

## 背景

`alarm-server` 当前已经提供 HTTP API，可用于新增、查询、删除闹钟。为了降低用户直接拼装 HTTP 请求的使用成本，需要在 `alarm-server` crate 内补充一个 CLI 入口，作为现有 API 的命令行封装层。

本次只讨论需求，不涉及实现。

## 目标

提供一个可执行 CLI，使用户能够通过命令行完成以下操作：

- 新增一次性闹钟
- 新增 cron 闹钟
- 查询闹钟列表
- 查询单个闹钟详情

CLI 直接调用 `alarm-server` 的现有 HTTP API，不新增服务端业务能力，不改变现有闹钟模型。

## 范围

### 本期范围

- 在 `alarm-server` crate 内提供 CLI 能力
- 支持 `create once`
- 支持 `create cron`
- 支持 `list`
- 支持 `get`
- 默认输出 JSON
- 支持通过参数指定服务端地址

### 非本期范围

- 不支持删除闹钟
- 不支持更新闹钟
- 不支持暂停、恢复闹钟
- 不支持从文件读取 `callback-body`
- 不对 `callback-body` 的业务格式做约束
- 不新增服务端 API

## CLI 定位

CLI 是 `alarm-server` 的命令行客户端，职责是：

- 接收用户命令和参数
- 做最小必要的参数校验
- 调用现有 HTTP API
- 将服务端响应按 JSON 输出给用户

CLI 不负责：

- 解释 `callback-body` 的业务含义
- 改写服务端返回结构
- 引入独立的数据存储或本地缓存

## 命令设计

建议命令结构如下：

```bash
alarm-cli create once ...
alarm-cli create cron ...
alarm-cli list ...
alarm-cli get <id>
```

说明：

- CLI 能力作为 `alarm-server` crate 内新增的独立二进制存在
- 服务端运行入口与 CLI 入口同属一个 crate，但职责分离

## 功能需求

### 1. 新增一次性闹钟

命令示意：

```bash
alarm-cli create once \
  --name "部署提醒" \
  --once-at "2026-04-01T14:30:00" \
  --callback-url "https://example.com/deploy" \
  --callback-body '{"task_id":42}'
```

参数要求：

- `--name`：可选
- `--once-at`：必填，格式为 `YYYY-MM-DDTHH:MM:SS`
- `--callback-url`：必填
- `--callback-body`：可选，按原始字符串透传

行为要求：

- CLI 调用 `POST /api/alarms`
- 请求体字段与现有服务端接口保持一致
- 成功时输出服务端返回 JSON

### 2. 新增 cron 闹钟

命令示意：

```bash
alarm-cli create cron \
  --name "工作日提醒" \
  --cron "0 0 9 * * 1-5" \
  --callback-url "https://example.com/notify" \
  --callback-body '{"message":"start"}'
```

参数要求：

- `--name`：可选
- `--cron`：必填
- `--callback-url`：必填
- `--callback-body`：可选，按原始字符串透传

行为要求：

- CLI 调用 `POST /api/alarms`
- `alarm_type` 固定为 `cron`
- 成功时输出服务端返回 JSON

### 3. 查询闹钟列表

命令示意：

```bash
alarm-cli list
alarm-cli list --status active
alarm-cli list --status completed
```

参数要求：

- `--status`：可选，仅支持 `active` 或 `completed`

行为要求：

- CLI 调用 `GET /api/alarms`
- 当传入 `--status` 时，附带查询参数
- 成功时默认输出 JSON

### 4. 查询单个闹钟

命令示意：

```bash
alarm-cli get 9f1c3b2d-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

参数要求：

- `id`：必填，作为位置参数传入

行为要求：

- CLI 调用 `GET /api/alarms/{id}`
- 成功时输出服务端返回 JSON

## 参数与请求映射

CLI 与服务端字段保持一一对应，避免引入额外抽象。

### create once

| CLI 参数 | 请求字段 |
|---|---|
| `--name` | `name` |
| `--once-at` | `once_at` |
| `--callback-url` | `callback_url` |
| `--callback-body` | `callback_body` |
| 固定值 | `alarm_type = "once"` |

### create cron

| CLI 参数 | 请求字段 |
|---|---|
| `--name` | `name` |
| `--cron` | `cron_expr` |
| `--callback-url` | `callback_url` |
| `--callback-body` | `callback_body` |
| 固定值 | `alarm_type = "cron"` |

## 校验要求

CLI 仅做基础校验，完整校验仍由服务端负责。

CLI 侧需要覆盖：

- 必填参数缺失时直接报错
- `--once-at` 格式不合法时直接报错
- `--status` 非 `active|completed` 时直接报错
- 服务端地址为空时直接报错

CLI 侧不负责：

- 校验 cron 语义是否合法
- 校验 `callback-body` 的 JSON 结构是否合法
- 判断 `once-at` 是否晚于当前时间

这些约束应交给服务端，以保证单一校验源。

## 输出要求

默认输出 JSON。

原因：

- 与用户要求一致
- 与服务端响应结构直接对齐
- 方便脚本集成和后续自动化使用

输出要求：

- 成功时打印服务端响应 JSON
- 失败时打印可读错误信息
- 默认不引入表格、彩色输出等额外终端格式

## 配置要求

CLI 需要支持指定服务端地址。

建议配置方式：

- 命令参数：`--server <url>`
- 环境变量：`ALARM_SERVER_URL`

优先级建议：

- 命令参数高于环境变量
- 环境变量高于默认值

默认值建议：

```text
http://127.0.0.1:8080
```

## 错误处理要求

需要覆盖以下错误场景：

- 参数缺失
- 参数格式错误
- 服务端不可达
- 服务端返回 4xx
- 服务端返回 5xx
- 服务端返回非预期响应体

处理原则：

- 优先保留服务端错误语义
- CLI 不包装成复杂错误层级
- 错误信息应让用户能判断是“本地参数问题”还是“服务端调用失败”

## 与现有项目的关系

需求约束如下：

- CLI 放在 `alarm-server` crate 内，不新建独立 crate
- CLI 依赖现有 `alarm-server` HTTP API
- 不修改现有 alarm 数据模型
- 不扩大 API 范围

## 验收标准

满足以下条件即可认为需求完成：

- 用户可通过 CLI 新增一次性闹钟
- 用户可通过 CLI 新增 cron 闹钟
- 用户可通过 CLI 查询闹钟列表
- 用户可通过 CLI 查询单个闹钟
- 默认输出 JSON
- 用户可通过参数或环境变量指定服务端地址
- `callback-body` 允许原样传递，不增加格式约束

## 文档拆分

为便于后续实施，本需求拆分为以下小 plan：

- `alarm-server-cli-plan-1.md`：CLI 范围与命令结构
- `alarm-server-cli-plan-2.md`：创建命令需求
- `alarm-server-cli-plan-3.md`：查询命令需求
- `alarm-server-cli-plan-4.md`：配置、输出与错误处理
