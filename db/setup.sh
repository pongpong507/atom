#!/bin/bash
# =============================================
# Atom DB Setup Script
# 在 Ubuntu PostgreSQL 機執行：bash db/setup.sh
# =============================================

set -e

# ============================================================
# === 可編輯區：修改以下變數後，直接執行此腳本即可 ===
# ============================================================
DB_HOST="localhost"
DB_PORT="5432"
DB_NAME="atom_db"
DB_USER="atom_user"
DB_PASSWORD="change_me_please"          # ← 請修改此密碼
APP_SECRET_KEY="$(openssl rand -hex 32 2>/dev/null || echo 'change_this_to_64_random_chars_minimum')"  # 自動生成隨機金鑰
# ============================================================
# === 可編輯區結束 ===
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Atom DB Setup ==="
echo "Database : ${DB_NAME}"
echo "User     : ${DB_USER}"
echo "Host     : ${DB_HOST}:${DB_PORT}"
echo ""

# 建立 PostgreSQL 使用者與資料庫（需要 sudo 存取 postgres 系統帳號）
echo "→ 建立 PostgreSQL 使用者與資料庫..."
sudo -u postgres psql <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '${DB_USER}') THEN
    CREATE USER ${DB_USER} WITH PASSWORD '${DB_PASSWORD}';
  ELSE
    ALTER USER ${DB_USER} WITH PASSWORD '${DB_PASSWORD}';
  END IF;
END
\$\$;

SELECT 'CREATE DATABASE ${DB_NAME} OWNER ${DB_USER}'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '${DB_NAME}')\gexec

GRANT ALL PRIVILEGES ON DATABASE ${DB_NAME} TO ${DB_USER};
SQL

# 套用 Schema
echo "→ 套用資料庫 Schema..."
PGPASSWORD="${DB_PASSWORD}" psql \
  -h "${DB_HOST}" -p "${DB_PORT}" \
  -U "${DB_USER}" -d "${DB_NAME}" \
  -f "${SCRIPT_DIR}/schema.sql"

# 輸出 Windows 主機需要的 .env 檔案
ENV_OUTPUT="${SCRIPT_DIR}/atom_windows.env"
cat > "${ENV_OUTPUT}" <<ENV
# ================================================================
# Atom - Windows 主機環境變數設定
# 步驟：
#   1. 將 <ubuntu-tailscale-ip> 替換為 Ubuntu 機的 Tailscale IP
#   2. 將此檔案複製至 Windows 主機的 Atom 專案根目錄，重新命名為 .env
# ================================================================

DATABASE_URL=postgresql://${DB_USER}:${DB_PASSWORD}@<ubuntu-tailscale-ip>:${DB_PORT}/${DB_NAME}
OLLAMA_URL=http://host.docker.internal:11434
OLLAMA_MODEL=llama3
VAULT_PATH=/app/vault
BIND_ADDR=0.0.0.0:8080
COOKIE_SECRET=${APP_SECRET_KEY}
BIND_ADDR=0.0.0.0:8080
ENV

echo ""
echo "✅ 資料庫初始化完成"
echo "✅ Windows 連線設定已輸出至：${ENV_OUTPUT}"
echo ""
echo "⚠️  後續步驟："
echo "   1. 編輯 ${ENV_OUTPUT}"
echo "      將 <ubuntu-tailscale-ip> 替換為本機 Tailscale IP"
echo "      （執行 tailscale ip -4 可查詢）"
echo "   2. 將該檔案複製到 Windows 主機，重新命名為 .env"
echo "   3. 在 Ubuntu 上編譯 Atom：cargo build --release"
echo "   4. 執行 bash db/seed_user.sh 建立登入帳號"
