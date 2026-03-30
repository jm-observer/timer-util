# Plan 2: 创建命令需求

## 目标

定义新增闹钟相关命令的参数、请求映射和校验边界。

## 范围

覆盖：

- `create once`
- `create cron`

## 需求

### 1. create once

命令形态：

```bash
alarm-cli create once \
  --name "部署提醒" \
  --once-at "2026-04-01T14:30:00" \
  --callback-url "https://example.com/deploy" \
  --callback-body '{"task_id":42}'
```

参数要求：

- `--name`：可选
- `--once-at`：必填
- `--callback-url`：必填
- `--callback-body`：可选

请求映射：

- `alarm_type = "once"`
- `once_at = --once-at`
- `callback_url = --callback-url`
- `callback_body = --callback-body`

### 2. create cron

命令形态：

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
- `--callback-body`：可选

请求映射：

- `alarm_type = "cron"`
- `cron_expr = --cron`
- `callback_url = --callback-url`
- `callback_body = --callback-body`

### 3. callback-body 约束

- CLI 不对 `callback-body` 的业务内容做解释
- CLI 不要求用户遵循特定字段结构
- CLI 不支持从文件读取 `callback-body`
- `callback-body` 的值按原始输入传递给服务端

### 4. 基础校验

CLI 需要做的校验：

- 必填参数存在
- `once-at` 格式符合 `YYYY-MM-DDTHH:MM:SS`

CLI 不做的校验：

- cron 表达式语义校验
- `callback-body` 内容结构校验
- `once-at` 是否晚于当前时间

这些校验交由服务端处理。

## 验收

- 两类创建命令的参数定义明确
- 请求字段映射明确
- `callback-body` 的边界明确
