# ⚛ Atom

> 紀念已逝去的 Atom 編輯器。

全地端、高隱私的 AI 個人知識管理與每日問答系統。自動掃描 Obsidian 筆記，透過本地 Ollama LLM 生成主動召回（Active Recall）題目，協助加深學習印象。

---

## 系統架構

```
iPhone (Obsidian)
    ↕ Möbius Sync / VaultSync
Ubuntu (Syncthing + PostgreSQL)  ←→ Tailscale VPN ←→  Windows (Docker + Ollama)
```

| 節點 | 角色 |
|------|------|
| iPhone | 記錄 Obsidian 筆記 |
| Ubuntu (24/7) | PostgreSQL 資料庫 + Syncthing 備份節點 |
| Windows | Ollama LLM + Atom Web 服務（Docker） |

---

## 功能

- 每日自動掃描當天修改的 Obsidian `.md` 筆記
- 生成 6 道題目：2 選擇題 + 2 簡答題 + 2 反思申論題
- 選擇題自動判分；申論題由 LLM 二次評分並給出評語
- 筆記質量評估（1–5 星）與改進建議
- 儀表板：答題正確率、筆記字數、AI 學習導師週報

---

## 技術棧

| 層 | 技術 |
|----|------|
| Web 後端 | Rust / Axum |
| 前端 | HTML + HTMX + Chart.js |
| 資料庫 | PostgreSQL (SQLx runtime queries) |
| LLM | Ollama (Llama 3 / Mistral) |
| 認證 | Argon2 密碼雜湊 + SignedCookieJar |
| 同步 | Syncthing (跨平台) |
| 網路 | Tailscale VPN |

---

## 快速部署

### 前置條件

- Ubuntu 機：已安裝 PostgreSQL、Rust (rustup)、Syncthing
- Windows 機：已安裝 Docker Desktop、Ollama（並下載 llama3 模型）
- 兩台機器已加入同一 Tailscale 網路

---

### 步驟一：Ubuntu — 資料庫初始化

```bash
git clone https://github.com/<your-username>/atom.git
cd atom

# 1. 編輯 db/setup.sh 頂部的密碼
nano db/setup.sh

# 2. 執行 setup（自動建 DB + Schema + 輸出 Windows 用的 .env）
bash db/setup.sh

# 3. 編譯 Atom binary
cargo build --release

# 4. 建立登入帳號（密碼輸入隱藏，由 Rust argon2 處理）
bash db/seed_user.sh
```

setup.sh 執行完後會輸出 `db/atom_windows.env`，內含 Windows 需要的所有連線設定。

---

### 步驟二：Ubuntu — Syncthing 設定

1. 在 Syncthing 介面新增 Obsidian Vault 資料夾
2. 複製 `db/stignore.example` 為 Vault 根目錄的 `.stignore`
3. 與 iPhone 及 Windows 配對

---

### 步驟三：Windows — 啟動 Atom

```bash
# 1. 將 Ubuntu 的 db/atom_windows.env 複製到此，重新命名為 .env
#    並將 <ubuntu-tailscale-ip> 替換為 Ubuntu 的 Tailscale IP
#    （Ubuntu 上執行 tailscale ip -4 可查詢）

# 2. 確認 Ollama 已啟動並下載模型
ollama pull llama3

# 3. 啟動容器
docker-compose up --build -d

# 4. 開啟瀏覽器
start http://localhost:8080
```

---

### 步驟四：iPhone — 筆記同步

安裝 **Möbius Sync** 或 **VaultSync**，連接至 Ubuntu Syncthing 節點。
由於 iOS 背景限制，寫完筆記後需手動開啟 App 觸發同步。

---

## 環境變數說明

複製 `.env.example` 為 `.env` 並填入：

| 變數 | 說明 |
|------|------|
| `DATABASE_URL` | PostgreSQL 連線字串（含 Ubuntu Tailscale IP） |
| `OLLAMA_URL` | Ollama API，Docker 內固定用 `http://host.docker.internal:11434` |
| `OLLAMA_MODEL` | 模型名稱，預設 `llama3` |
| `VAULT_PATH` | 容器內 Vault 路徑，預設 `/app/vault` |
| `BIND_ADDR` | 伺服器綁定位址，預設 `0.0.0.0:8080` |
| `COOKIE_SECRET` | Session Cookie 簽名金鑰，至少 64 字元（`openssl rand -hex 32`） |

---

## 本地開發（無 Docker）

```bash
# 需要本地 PostgreSQL 和 Ollama
cp .env.example .env
# 填入 DATABASE_URL 指向本地 PostgreSQL

cargo run          # 啟動 Web 伺服器
cargo run add-user # 新增使用者
```

---

## 資料庫 Schema

```
users              → 登入帳號
sessions           → 登入 session（7天效期，每小時自動清理過期）
daily_notes_summary → 每日筆記質量評估
questions          → AI 生成的問題庫
user_answers       → 答題記錄（含 AI 評語與分數）
```

---

## 授權

MIT License
