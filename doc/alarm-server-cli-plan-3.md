# Plan 3: 查询命令需求

## 目标

定义列表查询和单项查询的命令模型、参数和输出要求。

## 范围

覆盖：

- `list`
- `get`

## 需求

### 1. list

命令形态：

```bash
alarm-cli list
alarm-cli list --status active
alarm-cli list --status completed
```

参数要求：

- `--status`：可选
- 允许值：`active`、`completed`

行为要求：

- 不传 `--status` 时查询全部闹钟
- 传入 `--status` 时附带查询参数调用服务端接口
- 成功时默认输出 JSON

### 2. get

命令形态：

```bash
alarm-cli get <id>
```

参数要求：

- `id` 为必填位置参数

行为要求：

- 调用单个闹钟详情接口
- 成功时默认输出 JSON

### 3. 输出边界

- 默认输出 JSON
- 不要求默认表格输出
- 不要求筛选、排序、分页等扩展能力

## 验收

- 列表与详情命令的参数和行为定义明确
- 默认 JSON 输出被固定为需求
