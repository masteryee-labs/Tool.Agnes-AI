//! Agnes AI 真實 API QA 執行器。
//! 對 Agnes API 發出真實任務（小/中/大三級），執行後以「硬性證據」驗收：
//! 檔案系統實際內容 + cargo test 真實 Exit Code。絕不信任模型口頭回報。
//!
//! 用法：cargo run --release --bin qa_runner [small|medium|large|all]

use app_lib::{AgentLoop, Config, McpManager, TokenBudgeter, no_window::NoWindowExt};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 單一任務允許的最大代理步數（每步 = 一次 API 呼叫 + 工具執行）
const MAX_STEPS: usize = 6;
/// QA 任務的 API 逾時（生成大檔需要較長時間）
const QA_API_TIMEOUT_SECS: u64 = 120;
/// cargo test 失敗時的自愈修復輪數上限（沙盒硬性對齊 → 真實錯誤回饋）
const MAX_TEST_REPAIRS: usize = 2;

struct QaResult {
    name: &'static str,
    passed: bool,
    steps_used: usize,
    detail: String,
    tokens: u64,
}

fn ensure_dir(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
    std::fs::create_dir_all(path).expect("create QA workspace");
}

/// Windows temp 路徑常以 8.3 短檔名出現（MASTER~1），與 canonicalize 後的長路徑
/// 比對會失敗導致路徑圈禁誤判越權。統一轉成去除 \\?\ 前綴的長路徑。
fn long_path(p: &Path) -> PathBuf {
    match std::fs::canonicalize(p) {
        Ok(c) => {
            let s = c.to_string_lossy().to_string();
            PathBuf::from(s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s))
        }
        Err(_) => p.to_path_buf(),
    }
}

/// 在 workspace 中跑 cargo test，回傳 (exit_code, 失敗摘要)。真實證據，不經模型。
/// 測試斷言失敗印在 stdout、編譯錯誤在 stderr——兩者都擷取供自愈回饋。
fn cargo_test_in(dir: &Path) -> (i32, String) {
    let output = Command::new("cmd")
        .no_window()
        .arg("/C")
        .arg("chcp 65001 >nul && cargo test")
        .current_dir(dir)
        .output();
    match output {
        Ok(out) => {
            let code = out.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut summary: Vec<String> = stderr.lines()
                .filter(|l| l.contains("error") || l.contains("warning: unused"))
                .take(10)
                .map(str::to_string)
                .collect();
            let mut fail_context = 0usize;
            for l in stdout.lines() {
                let hit = l.contains("FAILED") || l.contains("panicked") || l.contains("assertion");
                if hit {
                    fail_context = 3;
                }
                if hit || (fail_context > 0 && (l.contains("left:") || l.contains("right:"))) {
                    summary.push(l.to_string());
                    fail_context = fail_context.saturating_sub(1);
                }
                if summary.len() >= 24 {
                    break;
                }
            }
            (code, summary.join("\n"))
        }
        Err(e) => (-1, format!("cargo 啟動失敗: {}", e)),
    }
}

/// 沙盒硬性對齊 + 自愈：cargo test 失敗時把真實錯誤砸回模型重寫，最多 MAX_TEST_REPAIRS 輪。
/// 回傳 (最終 exit_code, 最終摘要, 修復輪數, 追加步數, 追加 tokens, 追加 log)。
async fn heal_until_tests_pass(
    config: &Config,
    ws: &Path,
    db: &Path,
) -> (i32, String, usize, usize, u64, Vec<String>) {
    let (mut code, mut summary) = cargo_test_in(ws);
    let mut rounds = 0;
    let mut extra_steps = 0;
    let mut extra_tokens = 0;
    let mut extra_log = Vec::new();

    while code != 0 && rounds < MAX_TEST_REPAIRS {
        rounds += 1;
        let repair_prompt = format!(
            "[沙盒硬性對齊] 此 Rust crate 的 cargo test 失敗，Exit Code {}。真實錯誤輸出：\n{}\n\
             請先用 read_file 檢視相關的 src/ 檔案，找出實作與測試期望不一致之處，\
             以最小修改修復（修正錯誤的實作或修正錯誤的測試期望），用 write_file 重新寫入受影響檔案。",
            code, summary,
        );
        let (s, t, log) = run_agent_task(config, ws, db, &repair_prompt).await;
        extra_steps += s;
        extra_tokens += t;
        extra_log.push(format!("-- 自愈第 {} 輪 --", rounds));
        extra_log.extend(log);
        let (c2, s2) = cargo_test_in(ws);
        code = c2;
        summary = s2;
    }
    (code, summary, rounds, extra_steps, extra_tokens, extra_log)
}

/// 跑一個多步代理任務迴圈：把工具結果回饋給模型直到沒有新工具呼叫。
async fn run_agent_task(
    config: &Config,
    workspace: &Path,
    db_path: &Path,
    prompt: &str,
) -> (usize, u64, Vec<String>) {
    let agent = AgentLoop::new(config.clone(), workspace.to_string_lossy().to_string());
    let mcp = McpManager::new();
    let budgeter = tokio::sync::Mutex::new(TokenBudgeter::new(config.api.session_budget));

    let mut messages = vec![serde_json::json!({ "role": "user", "content": prompt })];
    let mut transcript = Vec::new();
    let mut steps = 0;

    for _ in 0..MAX_STEPS {
        steps += 1;
        match agent.run_step(&mut messages, &mcp, &budgeter, db_path).await {
            Ok(step) => {
                let rejected = step.audits.iter().filter(|a| a.verdict == "REJECTED").count();
                let result_head: String = step.execution_results.iter()
                    .map(|r| r.chars().take(120).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" | ");
                transcript.push(format!(
                    "step {}: tools={} rejected_audits={} approval={} result=[{}]",
                    steps, step.executed_tools.len(), rejected, step.requires_approval, result_head,
                ));
                messages.push(serde_json::json!({
                    "role": "assistant", "content": step.response_text.clone(),
                }));
                if step.executed_tools.is_empty() {
                    break;
                }
                let results = step.execution_results.join("\n---\n");
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": format!(
                        "[TOOL RESULTS]\n{}\n\n若任務已完成請只回覆「DONE」，否則繼續使用工具。",
                        results,
                    ),
                }));
                if step.response_text.contains("DONE") {
                    break;
                }
            }
            Err(e) => {
                transcript.push(format!("step {}: ERROR {}", steps, e));
                break;
            }
        }
    }

    let spent = budgeter.lock().await.total_spent();
    (steps, spent, transcript)
}

/// 小型任務：單檔寫入。驗收 = 檔案存在且內容含關鍵詞。
async fn qa_small(config: &Config, base: &Path, db: &Path) -> QaResult {
    let ws = base.join("small");
    ensure_dir(&ws);
    let ws = long_path(&ws);
    let prompt = "請建立檔案 hello.txt，內容必須包含一行「你好 Agnes」。只需要這一個檔案。";
    let (steps, tokens, log) = run_agent_task(config, &ws, db, prompt).await;

    let target = ws.join("hello.txt");
    let content = std::fs::read_to_string(&target).unwrap_or_default();
    let passed = target.exists() && content.contains("你好 Agnes");
    QaResult {
        name: "small（單檔寫入）",
        passed,
        steps_used: steps,
        tokens,
        detail: format!(
            "hello.txt exists={} contains_keyword={}\n{}",
            target.exists(), content.contains("你好 Agnes"), log.join("\n"),
        ),
    }
}

/// 中型任務：在既有 cargo crate 內實作函式與測試。驗收 = cargo test Exit Code 0。
async fn qa_medium(config: &Config, base: &Path, db: &Path) -> QaResult {
    let ws = base.join("medium");
    ensure_dir(&ws);
    let ws = long_path(&ws);
    std::fs::write(
        ws.join("Cargo.toml"),
        "[package]\nname = \"qa_med\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    ).unwrap();
    std::fs::create_dir_all(ws.join("src")).unwrap();
    std::fs::write(ws.join("src").join("main.rs"), "fn main() {}\n").unwrap();

    let prompt = "這是一個既有的 Rust crate。請覆寫 src/main.rs：實作 fn fib(n: u64) -> u64（迭代版，禁止遞迴）、\
                  fn main() 印出 fib(10)，並加入 #[cfg(test)] 模組驗證 fib(0)==0、fib(1)==1、fib(10)==55。\
                  代碼必須 100% 完整可編譯。";
    let (steps, tokens, mut log) = run_agent_task(config, &ws, db, prompt).await;

    let (code, summary, rounds, extra_steps, extra_tokens, extra_log) =
        heal_until_tests_pass(config, &ws, db).await;
    log.extend(extra_log);
    QaResult {
        name: "medium（Rust 函式 + 單元測試）",
        passed: code == 0,
        steps_used: steps + extra_steps,
        tokens: tokens + extra_tokens,
        detail: format!(
            "cargo test exit={} 自愈輪數={}\n{}\n{}",
            code, rounds, summary, log.join("\n"),
        ),
    }
}

/// 大型任務：多檔函式庫。驗收 = 至少 3 個 src 檔 + cargo test Exit Code 0。
async fn qa_large(config: &Config, base: &Path, db: &Path) -> QaResult {
    let ws = base.join("large");
    ensure_dir(&ws);
    let ws = long_path(&ws);
    std::fs::write(
        ws.join("Cargo.toml"),
        "[package]\nname = \"qa_large\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    ).unwrap();
    std::fs::create_dir_all(ws.join("src")).unwrap();

    let prompt = "這是一個既有的 Rust 函式庫 crate（src/lib.rs 尚未建立）。請建立三個檔案：\
                  (1) src/math.rs：pub fn add(a:i64,b:i64)->i64 與 pub fn clamp(v:i64,lo:i64,hi:i64)->i64，含測試；\
                  (2) src/strings.rs：pub fn reverse(s:&str)->String 與 pub fn count_cjk(s:&str)->usize（計算中日韓字元數），含測試（必須包含繁體中文測試字串）；\
                  (3) src/lib.rs：pub mod math; pub mod strings;。\
                  每個檔案都要 100% 完整可編譯，測試必須能通過。";
    let (steps, tokens, mut log) = run_agent_task(config, &ws, db, prompt).await;

    let files = ["src/lib.rs", "src/math.rs", "src/strings.rs"];
    let existing = files.iter().filter(|f| ws.join(f).exists()).count();
    let (code, summary, rounds, extra_steps, extra_tokens, extra_log) =
        heal_until_tests_pass(config, &ws, db).await;
    log.extend(extra_log);
    QaResult {
        name: "large（多檔函式庫 + CJK 測試）",
        passed: existing == files.len() && code == 0,
        steps_used: steps + extra_steps,
        tokens: tokens + extra_tokens,
        detail: format!(
            "files {}/{} cargo test exit={} 自愈輪數={}\n{}\n{}",
            existing, files.len(), code, rounds, summary, log.join("\n"),
        ),
    }
}

/// 自愈展示：植入「實作與測試不一致」的壞 crate，驗證真實錯誤回饋 → 模型修復 → 測試轉綠。
async fn qa_heal_demo(config: &Config, base: &Path, db: &Path) -> QaResult {
    let ws = base.join("heal_demo");
    ensure_dir(&ws);
    let ws = long_path(&ws);
    std::fs::write(
        ws.join("Cargo.toml"),
        "[package]\nname = \"qa_heal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    ).unwrap();
    std::fs::create_dir_all(ws.join("src")).unwrap();
    // 故意的缺陷：double 實作成 +1，測試期望 *2 —— cargo test 必然失敗
    std::fs::write(
        ws.join("src").join("lib.rs"),
        "pub fn double(x: i64) -> i64 {\n    x + 1\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn test_double() {\n        assert_eq!(double(2), 4);\n        assert_eq!(double(10), 20);\n    }\n}\n",
    ).unwrap();

    let (before_code, before_summary) = cargo_test_in(&ws);
    let (code, summary, rounds, steps, tokens, log) =
        heal_until_tests_pass(config, &ws, db).await;

    QaResult {
        name: "heal-demo（植入缺陷 → 真實錯誤回饋 → 模型自愈）",
        passed: before_code != 0 && code == 0 && rounds >= 1,
        steps_used: steps,
        tokens,
        detail: format!(
            "修復前 exit={}（{}）\n修復後 exit={} 自愈輪數={}\n{}\n{}",
            before_code,
            before_summary.lines().next().unwrap_or(""),
            code, rounds, summary, log.join("\n"),
        ),
    }
}

#[tokio::main]
async fn main() {
    let filter = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());

    // 先載入組態（會向上尋找 config.local.toml），再準備 QA 工作區
    let mut config = Config::load().expect("無法載入 config.local.toml — 請先在 GUI 設定 API 金鑰");
    if config.api.key.is_empty() {
        eprintln!("[QA ABORT] API 金鑰為空");
        std::process::exit(2);
    }
    // QA 模式：自動審查（工具直接執行，仍過 22-gate 驗證）+ 專案模式 + 延長 API 逾時
    config.security.auto_review = true;
    config.general.project_mode = "project".to_string();
    config.api.timeout_seconds = QA_API_TIMEOUT_SECS;

    let base = std::env::temp_dir().join("agnes_qa");
    std::fs::create_dir_all(&base).unwrap();
    let db_path: PathBuf = base.join("qa_state.db");
    let _ = std::fs::remove_file(&db_path);
    {
        let conn = app_lib::open_connection(&db_path).expect("open QA db");
        app_lib::init_db(&conn).expect("init QA db");
    }

    println!("=== Agnes AI 真實 API QA（模型: {} / 端點: {}）===", config.api.model, config.api.base_url);
    let mut results: Vec<QaResult> = Vec::new();

    if filter == "all" || filter == "small" {
        results.push(qa_small(&config, &base, &db_path).await);
        println!("[{}] {}", if results.last().unwrap().passed { "PASS" } else { "REJECT" }, results.last().unwrap().name);
    }
    if filter == "all" || filter == "medium" {
        results.push(qa_medium(&config, &base, &db_path).await);
        println!("[{}] {}", if results.last().unwrap().passed { "PASS" } else { "REJECT" }, results.last().unwrap().name);
    }
    if filter == "all" || filter == "large" {
        results.push(qa_large(&config, &base, &db_path).await);
        println!("[{}] {}", if results.last().unwrap().passed { "PASS" } else { "REJECT" }, results.last().unwrap().name);
    }
    if filter == "heal" {
        results.push(qa_heal_demo(&config, &base, &db_path).await);
        println!("[{}] {}", if results.last().unwrap().passed { "PASS" } else { "REJECT" }, results.last().unwrap().name);
    }

    println!("\n=== QA 報告 ===");
    let mut all_pass = true;
    for r in &results {
        all_pass &= r.passed;
        println!(
            "\n[{}] {} | steps={} tokens={}\n{}",
            if r.passed { "PASS" } else { "REJECT" },
            r.name, r.steps_used, r.tokens, r.detail,
        );
    }
    println!("\n總結: {}/{} 通過", results.iter().filter(|r| r.passed).count(), results.len());
    std::process::exit(if all_pass { 0 } else { 1 });
}
