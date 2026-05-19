#!/bin/bash
# =============================================
# Atom - 新增登入使用者
# 在 Ubuntu 上執行：bash db/seed_user.sh
# 前提：已執行 setup.sh，且已編譯 atom binary
# =============================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ATOM_BIN="${SCRIPT_DIR}/../target/release/atom"
ENV_FILE="${SCRIPT_DIR}/../.env"

# 確認 .env 存在（atom binary 需要讀取 DATABASE_URL）
if [ ! -f "${ENV_FILE}" ]; then
  echo "❌ 找不到 .env 檔案，請先將 atom_windows.env 複製為 .env"
  echo "   cp ${SCRIPT_DIR}/atom_windows.env ${ENV_FILE}"
  echo "   並填入正確的 DATABASE_URL"
  exit 1
fi

# 確認 atom binary 已編譯
if [ ! -f "${ATOM_BIN}" ]; then
  echo "❌ 找不到 atom binary"
  echo "   請先在此機器執行：cargo build --release"
  exit 1
fi

echo "=== Atom 使用者管理 ==="
echo "→ 呼叫 atom add-user（由 Rust argon2 處理密碼 hash）"
echo ""

"${ATOM_BIN}" add-user
