# EXLO

> 安全地将本地服务暴露到公网。无需下载，无需配置。开源且可自托管。

一个自托管的 SSH 反向隧道服务，注重隐私与可控性。

[English](README.md) · [免责声明](DISCLAIMER.md)

## 快速开始（Docker）

1. 复制示例环境变量文件：

```bash
cp .env.docker.example .env
```

2. 编辑 `.env`，设置强密码与密钥（尤其是 `POSTGRES_PASSWORD` 和 `AUTH_SECRET`）。
3. 构建并启动所有服务：

```bash
docker compose up -d --build
```

4. 打开控制台 `http://localhost:3000`。
5. 创建隧道：

```bash
ssh -R 8000:localhost:8000 -p 2222 test@localhost
```

6. 访问隧道（子域名可在日志或控制台查看）：

```bash
curl -H "Host: <subdomain>.localhost" http://localhost:8080/
```

## 本地开发

1. 安装依赖：

```bash
bun install
```

2. 启动 Postgres（开发数据库）：

```bash
bun run db:start
```

3. 配置 Web 应用：

```bash
cp apps/web/.env.example apps/web/.env
```

在 `apps/web/.env` 中设置（示例为本地数据库）：

```
DATABASE_URL=postgresql://postgres:password@localhost:5432/exlo
AUTH_SECRET=your-auth-secret-min-32-chars
HOMEPAGE_URL=http://localhost:3000
```

4. 应用数据库结构：

```bash
bun run db:push
```

5. 启动 Web 控制台：

```bash
bun run dev:web
```

6. 启动 SSH 反向隧道服务：

```bash
cd apps/tunnel
RUST_LOG=info DATABASE_URL=postgresql://postgres:password@localhost:5432/exlo cargo run
```

## 端口

- Web 控制台：`3000`
- SSH 服务：`2222`
- HTTP 代理：`8080`
- 管理 API（内部）：`9090`

## 项目宣言

1. **自托管与私有**：仅用于私有部署。不提供公共注册。你的基础设施，由你掌控。
2. **无需客户端**：使用标准 `ssh -R` 透明连接，无需在客户端安装额外 CLI。
3. **安全**：通过 Web 控制台完整管理，监控活动隧道并可随时终止连接。

## 技术栈与架构

系统由两个主要容器组成，采用“Sidecar”模式。

### 1. 数据平面（Rust 容器）
- **核心**：基于 `russh`（SSH 服务器 `:2222`）与 `hyper`（HTTP 代理 `:8080`）。
- **状态**：内存 `Arc<RwLock<AppState>>` 并与 PostgreSQL 同步。
- **内部 API**：`axum` 服务 `:9090`（仅内部使用）。
- **关键特性**：“虚拟绑定”（不占用物理端口）与持久化 Host Key。

### 2. 控制平面（Node.js 容器）
- **框架**：TanStack Start（SSR）。
- **认证**：Better Auth + PostgreSQL 适配器。
- **数据库**：Drizzle ORM。
- **模式**：BFF（Backend for Frontend），代理到 Rust 内部 API。

## 开发路线图

### 阶段 1：基础与数据结构（PostgreSQL/Drizzle）
**目标**：建立严格的访问控制数据结构。
- [x] `user` / `session` 表（Better Auth）。
- [ ] `invitations` 表（邀请制流程）。
- [x] `activation_codes` 表（设备流）。
- [x] `tunnels` 表（持久化隧道存储）。

### 阶段 2：Web 控制平面（认证与管理）
**目标**：收紧入口。
- [ ] **首次运行体验**：若无用户则重定向至 `/setup`。
- [ ] **邀请系统**：管理后台生成 `/join` 链接，关闭公开注册。

### 阶段 3：Rust 核心 - SSH 服务与 Key 持久化
**目标**：稳定、可持久的 SSH 服务。
- [x] **Key 持久化**：实现 `id_ed25519` 的 `load_or_generate` 逻辑。
- [x] **虚拟绑定**：将 `ssh -R` 映射到内部通道，不占用主机端口。
- [x] **终端 UI**：使用 `console` crate 的盒线字符美化设备激活界面。

### 阶段 4：Sidecar 管理 API
**目标**：打通数据平面与控制平面通信。
- [x] `axum` 服务 `:9090`。
- [x] `GET /tunnels`：列出活动会话。
- [x] `DELETE /tunnels/:subdomain`：终止特定连接。
- [x] 隧道注册/注销的内部 API。

### 阶段 5：仪表盘与实时监控
**目标**：“上帝视角”管理 UI。
- [ ] BFF Loader 获取隧道状态。
- [ ] Server Action `kickTunnel(subdomain)`。
- [ ] 实时轮询 UI。

### 阶段 6：设备流整合
**目标**：面向无界面客户端的无缝认证。
- [x] SSH 连接触发设备流（生成 code）。
- [x] Web `/activate` 页面验证。
- [x] SSH 会话轮询验证状态。
- [x] 授权过程加载动画。
- [x] 成功/错误 UI 盒子样式输出。

## 许可证

本项目采用 GNU Affero General Public License v3。参见 `LICENSE`。

## 免责声明

使用本软件需自行承担风险。参见 `DISCLAIMER.md`。
