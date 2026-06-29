# Pitfalls — 重複踩過的雷（跨 Session／跨工具，分類去重）

> 格式：`- [領域] 雷的描述 | 根因 | 防護措施`
> 觸發條件：發現「同類」錯誤重複發生（跨 Session 或跨工具）時追加。
> 去重規則：追加前先掃描既有條目，若同領域同根因已存在 → 更新防護措施而非新增。
> 硬性上限：每領域 ≤5 條，全文 ≤40 條。超過則合併最舊兩條。
> 任務開始時先掃本檔，尤其要對照本次任務領域的條目。

## 領域分類
- `arch` — 架構／模組／資料流
- `memory` — 記憶／RAG／蒸餾
- `security` — 安全／沙盒／金鑰
- `qa` — 驗證／測試／QA
- `token` — Token／預算／模型路由
- `agents` — 多代理編排
- `ui` — UI／介面
- `general` — 跨領域通用

## 已知陷阱

- [token] Discover 階段強制讀 SQLite + 所有 memory 檔 = Session 開頭 token 耗盡 | 根因：規則設計要求太多強制讀取 | 防護：簡單任務只讀 lessons.md + pitfalls.md；複雜/跨 Session 才讀 SQLite 和 Docs
- [token] agents.toon 221 行在非代理編排任務被載入 | 根因：路由條件不夠嚴格 | 防護：agents.toon 已加 LOAD_GATE；僅在修改編排相關檔案時才讀
- [token] AGENTS.md 與各 .toon 重複定義同一規則（鋼鐵戒律、反模式） | 根因：多次迭代追加造成冗餘 | 防護：規則只在一處定義；AGENTS.md 刪重複，.toon 為詳細實作
- [memory] Docs 6–18KB 在 token 壓力下被 AI 跳過，導致產出不符企劃 | 根因：Discover 強制讀 Docs 但 AI 面對 token 壓力省略 | 防護：00_OVERVIEW.md 加入快速決策錨，簡單任務無需讀其他 Docs
- [general] CLAUDE.md 與 AGENTS.md 不同步（Session 中斷後 AI 忘記 copy） | 根因：同步依賴 AI 主動記憶 | 防護：AGENTS.md 同步協議明確標示「下一步必須立即執行」；未來可加 git hook 備援
