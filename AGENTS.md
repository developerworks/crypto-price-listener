# AGENTS.md - AI 编码助手项目指南

本文档为 AI 编码助手提供项目背景、构建流程和开发规范。

---

## 项目概述

**crypto-price-listener** 是一个基于 Rust 的加密货币价格监听服务。该应用通过 WebSocket 连接到 Polymarket 的实时数据流，订阅加密货币价格更新，并将接收到的数据持久化到 MySQL 数据库中。

### 核心功能
- 连接到 Polymarket WebSocket 服务 (`wss://ws-live-data.polymarket.com`)
- 订阅 `crypto_prices_chainlink` 主题的价格更新
- 将价格数据（交易对符号、时间戳、价格）写入 MySQL 数据库
- 使用 tracing 进行结构化日志记录

---

## 技术栈

| 组件 | 用途 | 版本 |
|------|------|------|
| Rust | 编程语言 | Edition 2024 |
| Tokio | 异步运行时 | 1.50.0 |
| tokio-tungstenite | WebSocket 客户端 | 0.29.0 |
| sqlx | 异步 SQL 工具包 | 0.8.6 |
| serde/serde_json | 序列化/反序列化 | 1.0.x |
| tracing | 日志记录 | 0.1.x |
| anyhow | 错误处理 | 1.0.x |
| dotenv | 环境变量加载 | 0.15.0 |
| rust_decimal | 高精度十进制数 | 1.41.0 |

---

## 项目结构

```
crypto-price-listener/
├── Cargo.toml          # 项目配置和依赖
├── Cargo.lock          # 依赖锁定文件
├── .env                # 环境变量配置（gitignored）
├── .gitignore          # Git 忽略规则
├── README.md           # 项目文档（中文）
├── AGENTS.md           # 本文件
└── src/
    └── main.rs         # 单一入口文件（全部逻辑）
```

### 代码组织

当前项目采用**单文件架构**，所有逻辑集中在 `src/main.rs`：

- **数据模型** (`Subscription`, `SubscriptionItem`, `PriceUpdateMessage`, `PriceUpdatePayload`)
- **WebSocket 连接管理** - 使用 `tokio_tungstenite::connect_async`
- **消息处理循环** - 解析 JSON 消息并提取价格数据
- **数据库写入** - 使用 `sqlx::query!` 宏执行插入操作

---

## 数据库架构

### 表结构

```sql
CREATE TABLE `crypto_prices` (
  `id` int unsigned NOT NULL AUTO_INCREMENT,
  `symbol` char(8) COLLATE utf8mb4_general_ci NOT NULL,
  `price` decimal(38,0) NOT NULL,
  `timestamp` bigint NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
```

### 字段说明
- `symbol`: 交易对符号（如 BTCUSD、ETHUSD）
- `price`: 价格值，使用 `decimal(38,0)` 存储完整精度
- `timestamp`: Unix 时间戳（毫秒）

---

## 构建和运行

### 前置要求

1. **Rust 工具链** (>= 1.94.0)
2. **MySQL 数据库** - 运行并可通过网络访问
3. **环境变量配置** - 在项目根目录创建 `.env` 文件

### 环境配置

创建 `.env` 文件：

```bash
# 格式: mysql://用户名:密码@主机:端口/数据库名
DATABASE_URL="mysql://root:password@localhost:3306/polymarket"
```

### 构建命令

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行（会自动读取 .env）
cargo run
```

### sqlx 编译时检查

项目使用 `sqlx::query!` 宏进行**编译时 SQL 验证**。这要求：
- `.env` 文件必须存在且包含有效的 `DATABASE_URL`
- 数据库必须可访问且包含相应的表结构
- 如果数据库不可用，编译将失败

---

## 代码风格指南

### 一般约定

1. **语言**：代码注释和文档使用中文
2. **错误处理**：使用 `anyhow::Result` 进行错误传播
3. **日志记录**：使用 `tracing` crate，日志级别为 `INFO`
4. **异步风格**：使用 `tokio` 运行时，函数标记为 `async`

### 命名规范

- 结构体：PascalCase (`PriceUpdateMessage`)
- 函数/变量：snake_case (`price_update`, `full_accuracy_value`)
- 常量：SCREAMING_SNAKE_CASE (`URL`)

### 特殊标记

```rust
#[allow(dead_code)]  // 用于暂时未使用的字段，避免编译器警告
#[serde(rename = "type")]  // 处理 Rust 关键字冲突的字段名
type_: String,
```

---

## 测试策略

当前项目**未包含自动化测试**。如需添加测试：

```bash
# 运行测试
cargo test

# 添加单元测试示例
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization() {
        // 测试 JSON 反序列化
    }
}
```

---

## 部署注意事项

### 环境变量

生产环境必须设置 `DATABASE_URL`，建议：
- 使用 secrets 管理服务存储数据库凭据
- 避免将 `.env` 文件提交到版本控制

### 数据库连接池

当前配置：最大 5 个连接 (`max_connections(5)`)
根据负载情况可能需要调整。

### 错误恢复

WebSocket 连接断开后不会自动重连。生产环境可能需要：
- 添加指数退避重连逻辑
- 监控连接健康状态
- 添加优雅关闭处理（SIGTERM/SIGINT）

---

## 安全考虑

1. **数据库凭据**：`.env` 文件已添加到 `.gitignore`，不会意外提交
2. **TLS 连接**：`tokio-tungstenite` 使用 `native-tls` 功能进行加密 WebSocket 连接
3. **SQL 注入**：使用 `sqlx::query!` 宏，参数化查询防止注入

---

## 扩展建议

如需扩展功能，可考虑：
- 将单文件拆分为模块（`models/`, `db/`, `websocket/`）
- 添加配置管理（支持多个交易对订阅）
- 实现优雅关闭和连接重连
- 添加指标导出（Prometheus）
- 支持其他数据库（PostgreSQL、SQLite）
