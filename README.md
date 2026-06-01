# 灵犀节点 (SparkNode) · 高维叙事与世界演算引擎

> **High-Dimensional Narrative & World Simulation Engine**

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Next.js](https://img.shields.io/Next.js-15-black.svg)](https://nextjs.org/)
[![Python](https://img.shields.io/Python-3.12-blue.svg)](https://www.python.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## 🌌 愿景

灵犀节点不是一个传统的内容平台。它是一个结合 **4D 世界模型**、**量子叙事叠加态**、**硅基灵魂** 与 **DePIN 算力本位经济** 的终极元宇宙引擎。

## 🏗️ 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                    客户端层 (Client)                      │
│  WebGPU渲染 │ WebNN情感计算 │ 3D Gaussian Splatting Viewer │
├─────────────────────────────────────────────────────────┤
│                    API 网关层 (Gateway)                    │
│           Rust/Axum + gRPC + WebSocket 双向流              │
├──────────┬──────────┬──────────────┬────────────────────┤
│ 叙事引擎  │ 世界引擎  │   灵魂引擎    │     经济引擎        │
│ Narrative │ World    │   Soul       │     Economy        │
│ Engine    │ Engine   │   Engine     │     Engine         │
├──────────┴──────────┴──────────────┴────────────────────┤
│              核心基础设施层 (Infrastructure)                │
│  向量数据库 │ 事件总线 │ 任务调度 │ 区块链 │ P2P算力网络     │
└─────────────────────────────────────────────────────────┘
```

## 🛠️ 技术栈

| 层级 | 技术选型 |
|------|---------|
| **高并发核心** | Rust + Axum + gRPC |
| **AI 智能层** | Python + FastAPI + Ray |
| **前端渲染** | Next.js 15 + WebGPU + WASM |
| **向量数据库** | Qdrant (Rust 原生) |
| **关系数据库** | PostgreSQL + pgvector |
| **图数据库** | Memgraph (内存级) |
| **消息/缓存** | Redis |
| **容器编排** | Docker Compose |

## 🚀 快速开始

### 前置要求

- Rust 1.80+
- Node.js 20+
- Python 3.12+
- Docker & Docker Compose

### Windows 一鍵環境配置（推薦）

以**系統管理員** PowerShell 執行（可自動設定 Rust 路徑、Defender 排除、`.env`、Docker、migrations、依賴與 `cargo check`）：

```powershell
cd E:\Jerry_python\SparkNode
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\setup-env.ps1
```

常用參數：

| 參數 | 說明 |
|------|------|
| `-RustRoot "D:\rust"` | 自訂 Rust 安裝目錄（預設 `E:\rust-toolchain`） |
| `-SkipDocker` | 不啟動 docker compose / migrations |
| `-SkipDeps` | 不執行 `npm install` / `pip install` |
| `-SkipRust` | 不安裝 Rust、不執行 cargo check |
| `-NoUserEnv` | 不寫入使用者永久環境變數 |

日常開發前載入環境（新終端機）：

```powershell
. .\scripts\rust-env.ps1
# 或
. .\scripts\sparknode.env.ps1
```

亦可雙擊 [`scripts/setup-env.bat`](scripts/setup-env.bat)。

**常見問題：** C 槽空間不足請用 `-RustRoot` 指向 E/D 槽；`sqlx` 解壓失敗多為防毒阻擋 `.sql`，需將工具鏈與專案目錄加入 Defender 排除。

Linux/macOS 仍使用 `./scripts/setup.sh`（Git Bash / WSL）。

### 一键启动

```bash
# 1. 克隆项目
git clone https://github.com/your-org/sparknode.git
cd sparknode

# 2. 启动基础设施
docker compose up -d

# 3. 初始化数据库
./scripts/setup.sh

# 4. 启动 Rust 网关
cargo run -p sp-gateway

# 5. 启动 Python LLM 服务
cd services/sp-llm-router && uvicorn src.main:app --port 8001

# 6. 启动前端
cd web && npm run dev
```

## 📂 项目结构

```
SparkNode/
├── crates/                    # Rust 核心微服务
│   ├── sp-common/            # 共享类型与协议
│   ├── sp-gateway/           # API 网关 (Axum)
│   ├── sp-narrative-engine/  # 叙事引擎 (DAG + MDP + Chronos)
│   ├── sp-world-engine/      # 世界引擎 (物理模拟)
│   ├── sp-soul-engine/       # 灵魂引擎 (NPC Agent)
│   └── sp-economy-engine/    # 经济引擎 ($SPARK + DAO)
├── services/                  # Python AI 服务
│   ├── sp-llm-router/        # LLM 路由调度
│   └── sp-multi-agent/       # 多智能体模拟
├── web/                       # 前端 (Next.js + WebGPU)
├── contracts/                 # 智能合约
├── migrations/                # 数据库迁移
└── scripts/                   # 开发脚本
```

## 📜 核心术语表

| 术语 | 说明 |
|------|------|
| **星轨编织器 (StarLoom)** | 万级 DAG 节点的非线性剧情编辑器 |
| **灵犀矩阵 (SparkMatrix)** | 多 LLM 并发流式对比面板 |
| **Chronos 引擎** | 时间轴回溯与蝴蝶效应可视化引擎 |
| **海马体架构 (Hippocampus)** | NPC 持久化向量记忆网络 |
| **波函数坍缩 (Observer Collapse)** | 读者情感数据实时注入剧情概率 |
| **PulseStream** | WebSocket 实时事件推送协议 |

## License

MIT