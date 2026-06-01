#!/bin/bash
# ═══════════════════════════════════════════════════
#  灵犀节点 (SparkNode) 一键初始化脚本
# ═══════════════════════════════════════════════════

set -e

echo "🌌 灵犀节点 (SparkNode) 初始化中..."
echo "════════════════════════════════════════════"

# 1. 检查前置依赖
echo "📋 检查前置依赖..."
command -v docker >/dev/null 2>&1 || { echo "❌ 需要安装 Docker"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "❌ 需要安装 Rust"; exit 1; }
command -v node >/dev/null 2>&1 || { echo "❌ 需要安装 Node.js"; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "❌ 需要安装 Python 3.12+"; exit 1; }
echo "✓ 所有依赖已就绪"

# 2. 复制环境配置
if [ ! -f .env ]; then
    cp .env.example .env
    echo "✓ .env 已创建 (请填入实际的 API Key)"
fi

# 3. 启动基础设施
echo "🐳 启动 Docker 基础设施..."
docker compose up -d
echo "✓ PostgreSQL + Qdrant + Memgraph + Redis 已启动"

# 4. 等待数据库就绪
echo "⏳ 等待 PostgreSQL 就绪..."
sleep 5

# 5. 运行数据库迁移
echo "🗄️ 执行数据库迁移..."
for f in $(ls migrations/*.sql 2>/dev/null | sort); do
    echo "  执行: $f"
    docker exec -i sp-postgres psql -U spark -d sparknode < "$f"
done
echo "✓ 数据库迁移完成"

# 6. 安装前端依赖
echo "📦 安装前端依赖..."
cd web && npm install && cd ..
echo "✓ 前端依赖已安装"

# 7. 安装 Python 依赖
echo "🐍 安装 Python 依赖..."
cd services/sp-llm-router && pip install -e . && cd ../..
echo "✓ Python 依赖已安装"

# 8. 编译 Protobuf
echo "📝 编译 Protobuf..."
cd crates/sp-common && cargo build && cd ../..
echo "✓ Protobuf 已编译"

echo ""
echo "════════════════════════════════════════════"
echo "🚀 灵犀节点初始化完成！"
echo ""
echo "启动命令:"
echo "  cargo run -p sp-gateway          # Rust 网关"
echo "  cd services/sp-llm-router && python -m src.main  # LLM 服务"
echo "  cd web && npm run dev             # 前端"
echo ""
echo "访问地址:"
echo "  API 网关:    http://localhost:3001"
echo "  LLM 服务:    http://localhost:8001"
echo "  前端:        http://localhost:3000"
echo "  Qdrant UI:   http://localhost:6333/dashboard"
echo "  Memgraph UI: http://localhost:7444"
echo "════════════════════════════════════════════"