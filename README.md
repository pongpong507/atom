# ⚛ Atom

> 紀念已逝去的 Atom 編輯器。

全地端、高隱私的 AI 個人知識管理與每日問答系統。自動掃描 Obsidian 筆記，透過本地 Ollama LLM 生成主動召回（Active Recall）題目，協助加深學習印象。

**全程 Rust 開發，資料不離開你的設備。**

---

## 系統架構

```
iPhone (Obsidian)
    ↕ Möbius Sync（手動觸發）
Ubuntu (Syncthing + PostgreSQL)  ←→ Tailscale VPN ←→  Windows (Docker + Ollama)
```

| 節點 | 角色 |
|------|------|
| iPhone | 用 Obsidian App 記錄每日學習筆記 |
| Ubuntu (24/7) | PostgreSQL 資料庫 + Syncthing 中繼備份節點 |
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

- **Ubuntu 機**：已安裝 PostgreSQL、Rust (`rustup`)、Syncthing
- **Windows 機**：已安裝 Docker Desktop、[Ollama](https://ollama.ai)（並下載 llama3 模型）
- 兩台機器已加入同一 **Tailscale** 網路
- **iPhone**：已安裝 Obsidian App、Möbius Sync App

---

### 步驟一：Ubuntu — 資料庫初始化

```bash
git clone https://github.com/pongpong507/atom.git
cd atom

# 1. 編輯 db/setup.sh 頂部的密碼（唯一需要修改的地方）
nano db/setup.sh

# 2. 執行 setup（自動建 DB + Schema + 輸出 Windows 用的 .env）
bash db/setup.sh

# 3. 編譯 Atom binary（第一次需要幾分鐘）
cargo build --release

# 4. 建立登入帳號（密碼輸入隱藏，由 Rust argon2 處理）
bash db/seed_user.sh
```

`setup.sh` 執行完後會在 `db/atom_windows.env` 輸出 Windows 所需的所有連線設定。

---

### 步驟二：Ubuntu — Syncthing 設定

1. 開啟 Syncthing Web UI（通常是 `http://localhost:8384`）
2. 點選「新增資料夾」，路徑設為要存放 Vault 的位置，例如 `~/ObsidianVault`
3. 複製 `db/stignore.example` 為該資料夾內的 `.stignore`：
   ```bash
   cp db/stignore.example ~/ObsidianVault/.stignore
   ```
4. 記下 Ubuntu 機的 **Syncthing Device ID**（Web UI 右上角「顯示 ID」）

---

### 步驟三：Windows — 啟動 Atom

```powershell
# 1. 從 Ubuntu 取得 db/atom_windows.env，放到專案根目錄重新命名為 .env
#    （scp 或直接複製貼上內容）
#    然後將 <ubuntu-tailscale-ip> 替換為 Ubuntu 的 Tailscale IP

# 查詢 Ubuntu 的 Tailscale IP（在 Ubuntu 上執行）
tailscale ip -4

# 2. 確認 Ollama 已啟動並下載模型
ollama pull llama3

# 3. 啟動容器（首次 build 需幾分鐘）
docker-compose up --build -d

# 4. 開啟瀏覽器
start http://localhost:8080
```

健康檢查：`curl http://localhost:8080/health` 回傳 `ok` 表示正常運行。

---

### 步驟四：iPhone — Obsidian + Möbius Sync 完整設定

#### 4-1 建立 Obsidian Vault

1. 安裝 [Obsidian](https://apps.apple.com/app/obsidian-connected-notes/id1557175442)
2. 打開 Obsidian → 「建立新 Vault」
3. **重要**：選擇儲存位置為「**On My iPhone**」（Files App 下的「我的 iPhone」→「Obsidian」）
   - **不要**選 iCloud Drive：會與 Syncthing 產生同步衝突
   - 位置路徑：`On My iPhone/Obsidian/<VaultName>`

#### 4-2 安裝並設定 Möbius Sync

1. 安裝 [Möbius Sync](https://apps.apple.com/app/möbius-sync/id1539203216)（免費，支援 Syncthing 協定）
2. 打開 Möbius Sync → 底部選單點「Syncthing」
3. 點右上角「＋」→「Add Remote Device」
   - 輸入 Ubuntu 機的 **Syncthing Device ID**（從步驟二取得）
   - Device Name 可自訂，例如「Atom-Ubuntu」
4. **在 Ubuntu Syncthing Web UI** 接受來自 iPhone 的配對請求
5. 回到 Möbius Sync → 「Folders」→「＋」→「Add Existing Folder」
   - 選擇剛建立的 Obsidian Vault 資料夾
   - 在下方「Share With」選擇「Atom-Ubuntu」
6. **在 Ubuntu Syncthing Web UI** 接受共享此資料夾的請求，路徑設為 `~/ObsidianVault`

#### 4-3 Windows Syncthing（若需要三向同步）

若希望 Windows 也有 Vault 備份（可選）：
1. 安裝 [SyncTrayzor](https://github.com/canton7/SyncTrayzor/releases)（Windows 版 Syncthing GUI）
2. 在 Ubuntu Syncthing 加入 Windows 裝置，共享同一個 Vault 資料夾
3. 路徑設為 `D:\ObsidianVault`（與 `docker-compose.yml` 的 Volume 掛載一致）

#### 4-4 Syncthing 忽略設定

套用 `.stignore` 以防止衝突：
```
# 各 Obsidian Vault 根目錄的 .stignore 內容
.obsidian/workspace
.obsidian/workspace.json
.obsidian/workspace-mobile.json
.obsidian/cache
.DS_Store
Thumbs.db
```

---

### 步驟五：日常使用流程

```
① iPhone 上用 Obsidian 寫筆記
        ↓
② 打開 Möbius Sync → 點右下角「同步」按鈕
   （iOS 背景限制：必須手動觸發，無法完全自動）
        ↓
③ 等待 Möbius Sync 顯示「Up to Date」
        ↓
④ 在電腦打開 http://localhost:8080
        ↓
⑤ 登入後點「今日問答」→ 自動生成題目並開始作答
        ↓
⑥ 每週查看「儀表板」→ 生成 AI 學習導師評語
```

> **iOS 背景同步說明**
> iOS 系統限制 App 在背景執行網路操作，因此 Möbius Sync **無法自動在背景同步**。
> 建議習慣：寫完筆記後，切換到 Möbius Sync，等到頁面顯示「Synced」或進度條完成，再關閉 App。
> 整個過程通常 5–30 秒（視筆記大小與網路速度）。

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
# 編輯 .env，填入本地 DATABASE_URL

cargo run          # 啟動 Web 伺服器（http://localhost:8080）
cargo run add-user # 新增使用者
```

---

## 資料庫 Schema

```
users               → 登入帳號
sessions            → 登入 session（7天效期，每小時自動清理過期）
daily_notes_summary → 每日筆記質量評估
questions           → AI 生成的問題庫
user_answers        → 答題記錄（含 AI 評語與分數）
```

---

## 授權

MIT License
