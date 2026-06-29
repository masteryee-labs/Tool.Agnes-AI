# Lessons — 蒸餾後的教訓（FIFO 上限 30 條，每條 ≤2 行）

> 格式：`- [日期] 領域 | 教訓（≤2 行）`
> 滿 30 條時刪最舊。重複教訓禁止重複記錄——先掃描既有條目，雷同則更新而非新增。
> 跨 Session／跨工具通用。任務開始時先掃本檔避免重複犯錯。

- [2026-06] token | AGENTS.md 與 .toon 重複定義造成 token 浪費；解法：規則只在一處定義，AGENTS.md 只做路由器。
- [2026-06] memory | Docs 太大導致 AI token 壓力下跳過；解法：在 00_OVERVIEW.md 加快速決策錨，簡單任務無需讀其他 Docs。
- [2026-06] general | memory.toon mode:all 是隱藏的最大 Token 浪費源（每次任務都載入 104 行）；所有 toon 必須設 on-demand + LOAD_GATE。
- [2026-06] general | supersedes 欄位若指向不存在的舊檔，AI 工具可能嘗試尋找並報錯；改用 note/supersedes_note。
