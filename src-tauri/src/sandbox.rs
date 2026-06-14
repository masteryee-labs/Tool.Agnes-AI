//! ─── Sandbox Security Layer (Phase 2 Hardening) ──────────────────────────────
//!
//! 防禦目標：
//!  1. 命令注入 — program 白名單 + 參數引號保護 + 防止 shell 鏈接
//!  2. Path Traversal — 安全路徑解析 + symlink 防護
//!  3. Exit Code 對齊 — 所有執行路徑統一對齊邏輯
//!  4. 資源保護 — 輸入長度限制
//!  5. 工作目錄隔離 — 沙盒必須限制在工作區內

use std::path::Path;
use std::process::{Command, Stdio};
use crate::locale;

// ─── SandboxResult ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub is_aligned_success: bool,
}

impl SandboxResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0) && self.stderr.trim().is_empty()
    }

    /// 檢查是否為「虛假回報」：模型報告成功但實際 Exit Code != 0 或 stderr 有內容。
    pub fn is_false_positive(&self) -> bool {
        !self.is_aligned_success || !self.stderr.trim().is_empty()
    }
}

// ─── 常數 ─────────────────────────────────────────────────────────────────────

/// 最大指令字串長度（bytes）
const MAX_CMD_LEN: usize = 4096;

/// 最大參數數量
const MAX_ARG_COUNT: usize = 64;

/// 最大單一參數長度
const MAX_ARG_LEN: usize = 2048;

/// 允許執行的 program 白名單（常見安全命令）
const ALLOWED_PROGRAMS: &[&str] = &[
    // 基本檔案操作（唯讀 / 安全）
    "ls", "dir", "cat", "type", "echo", "mkdir", "md",
    "find", "findstr", "grep", "where", "whereis",
    // 版本 / 資訊
    "version", "--version", "-v", "--help", "help",
    "rustc", "cargo", "python", "python3", "pip", "pip3",
    "node", "npm", "npx", "go", "gcc", "g++", "make", "cmake",
    "clang",
    // 系統工具
    "whoami", "hostname", "ipconfig", "ifconfig", "uname",
    "date", "time", "pause", "cls", "clear",
    // 文字處理
    "sort", "uniq", "head", "tail", "wc", "tr", "sed", "awk",
    // 編譯/建構
    "ctest", "pytest", "gradle",
];

/// 危險 program 名稱（一票否決）
const FORBIDDEN_PROGRAMS: &[&str] = &[
    "del", "rm", "rmdir", "rd", "format", "diskpart",
    "shutdown", "restart", "poweroff", "halt",
    "wget", "curl", "nc", "ncat", "nmap", "net",
    "reg", "regedit", "sc", "netsh",
];

// ─── 白名單檢查 ───────────────────────────────────────────────────────────────

/// 驗證 program 名稱是否在白名單內。
/// 允許路徑前綴（如 `C:\Windows\System32\cmd.exe`）但取最後一個元件比較。
pub fn is_allowed_program(program: &str) -> bool {
    let base = program.split('\\').next_back().unwrap_or(program);
    let base = base.split('/').next_back().unwrap_or(base);
    let base_lower = base.to_lowercase();

    ALLOWED_PROGRAMS
        .iter()
        .any(|allowed| allowed.to_lowercase() == base_lower)
}

/// 檢查 program 是否在禁止名單中。
pub fn is_forbidden_program(program: &str) -> bool {
    let base = program.split('\\').next_back().unwrap_or(program);
    let base_lower = base.to_lowercase();
    FORBIDDEN_PROGRAMS
        .iter()
        .any(|forbidden| forbidden.to_lowercase() == base_lower)
}

// ─── 命令注入防禦 ─────────────────────────────────────────────────────────────

/// 危險的 Shell 元字元與鏈接序列。
const DANGEROUS_CHARS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '{', '}', '<', '>',
    '\n', '\r', '\'', '"', '!', '#', '~', '?', '*', '[', ']',
];

/// Shell 鏈接序列（多個字元組成的攻擊模式）
const DANGEROUS_SEQUENCES: &[&str] = &[
    "&&", "||", "|||", ";;",
    "$(", "${", "`",
    ">>", ">",
    "<&", ">&",
    "\\\n",
];

/// 單一參數安全檢查。reject 任何包含 Shell 元字元或危險序列的參數。
pub fn sanitize_arg(arg: &str) -> Result<String, String> {
    if arg.is_empty() {
        return Ok(String::new());
    }

    // 長度限制
    if arg.len() > MAX_ARG_LEN {
        return Err(format!("Argument too long: {} bytes (max {})", arg.len(), MAX_ARG_LEN));
    }

    // 檢查危險序列
    for seq in DANGEROUS_SEQUENCES {
        if arg.contains(*seq) {
            return Err(format!(
                "Argument rejected: dangerous sequence '{}'",
                seq.replace('\n', "\\n").replace('\r', "\\r")
            ));
        }
    }

    // 檢查危險字元
    for ch in DANGEROUS_CHARS {
        if arg.contains(*ch) {
            return Err(format!("Argument rejected: dangerous character '{}'", ch));
        }
    }

    // 拒絕路徑穿越
    if has_path_traversal_component(arg) {
        return Err("Argument rejected: path traversal attempt".to_string());
    }

    Ok(arg.to_string())
}

// ─── 安全路徑解析 ─────────────────────────────────────────────────────────────

/// 檢測路徑中是否包含 `..` 路徑穿越元件。
/// 使用路徑元件檢查而非字串包含，避免誤判（如 `..hello` 不算穿越）。
pub fn has_path_traversal_component(path: &str) -> bool {
    let path = path.trim();
    for component in path.split(['/', '\\']) {
        if component == ".." {
            return true;
        }
    }
    false
}

/// 安全路徑解析：檢查 candidate 是否在 workspace 內部。
/// 包含 symlink 防範：如果 workspace 本身是 symlink，先解析。
pub fn is_path_in_workspace(
    workspace: &Path,
    candidate: &Path,
) -> bool {
    // 路徑穿越預檢
    if has_path_traversal_component(candidate.to_string_lossy().as_ref()) {
        return false;
    }

    // Helper to strip Windows UNC prefix \\?\
    let strip_unc = |path: &Path| -> std::path::PathBuf {
        let s = path.to_string_lossy();
        match s.strip_prefix(r"\\?\") {
            Some(stripped) => std::path::PathBuf::from(stripped),
            None => path.to_path_buf(),
        }
    };

    // 解析 workspace 的實體路徑（處理 symlink）
    let ws_canonical = std::fs::canonicalize(workspace);

    // 解析 candidate 的實體路徑（如果存在）
    let cand_canonical = std::fs::canonicalize(candidate);

    match (ws_canonical, cand_canonical) {
        (Ok(ws), Ok(cand)) => {
            let clean_cand = strip_unc(&cand);
            let clean_ws = strip_unc(&ws);
            clean_cand.starts_with(&clean_ws)
        }
        (Ok(ws), Err(_)) => {
            // candidate 不存在：使用前綴檢查（但需先確認穿越）
            let clean_ws = strip_unc(&ws);
            let clean_cand = strip_unc(candidate);
            clean_cand.starts_with(&clean_ws)
        }
        (Err(ws_err), _) => {
            // workspace 無法解析：退回基本檢查
            eprintln!(
                "[SANDBOX WARN] workspace canonicalize failed: {}",
                ws_err
            );
            let clean_workspace = strip_unc(workspace);
            let clean_cand = strip_unc(candidate);
            clean_cand.starts_with(&clean_workspace)
        }
    }
}

// ─── 參數陣列驗證 ─────────────────────────────────────────────────────────────

/// 全參數安全檢查。在 non-full-access 模式下，逐一檢查所有參數。
pub fn sanitize_all_args(args: &[&str]) -> Result<Vec<String>, String> {
    if args.len() > MAX_ARG_COUNT {
        return Err(format!(
            "Too many arguments: {} (max {})",
            args.len(),
            MAX_ARG_COUNT
        ));
    }

    let mut sanitized = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        sanitized.push(sanitize_arg(arg)?);
        // 檢查是否包含危險序列
        for seq in DANGEROUS_SEQUENCES {
            if arg.contains(*seq) {
                return Err(format!(
                    "Argument #{} contains dangerous sequence: '{}'",
                    i,
                    seq.replace('\n', "\\n").replace('\r', "\\r")
                ));
            }
        }
    }
    Ok(sanitized)
}

// ─── Program 白名單與長度 ─────────────────────────────────────────────────────

/// 驗證 program 名稱長度。
pub fn is_program_valid_length(program: &str) -> bool {
    !program.is_empty() && program.len() <= MAX_ARG_LEN
}

/// 全面安全檢查：program 白名單 + 參數驗證 + 路徑穿越。
/// 返回 `Err(String)` 表示安全檢查失敗，必須拒絕執行。
pub fn validate_sandbox_input(
    program: &str,
    args: &[&str],
    full_access: bool,
    workspace: Option<&Path>,
) -> Result<(), String> {
    if full_access {
        // full_access 跳過大部分檢查，但仍限制長度與禁止危險序列
        if program.is_empty() {
            return Err("Empty program name".to_string());
        }
        // 即使 full_access 也禁止危險序列
        for seq in DANGEROUS_SEQUENCES {
            if program.contains(*seq) {
                return Err(format!(
                    "Program contains dangerous sequence: '{}'",
                    seq.replace('\n', "\\n").replace('\r', "\\r")
                ));
            }
        }
        for (i, arg) in args.iter().enumerate() {
            for seq in DANGEROUS_SEQUENCES {
                if arg.contains(*seq) {
                    return Err(format!(
                        "Arg #{} contains dangerous sequence: '{}'",
                        i,
                        seq.replace('\n', "\\n").replace('\r', "\\r")
                    ));
                }
            }
        }
        return Ok(());
    }

    // ─── 1. Program 白名單 ───
    if !is_allowed_program(program) {
        return Err(format!(
            "Program not in allowlist: '{}'",
            program
        ));
    }

    // ─── 2. Program 禁止名單 ───
    if is_forbidden_program(program) {
        return Err(format!(
            "Program is forbidden: '{}'",
            program
        ));
    }

    // ─── 3. Program 長度 ───
    if !is_program_valid_length(program) {
        return Err("Program name too long".to_string());
    }

    // ─── 4. 參數驗證 ───
    sanitize_all_args(args)?;

    // ─── 5. 路徑穿越檢查 ───
    if has_path_traversal_component(program) {
        return Err("Program path contains traversal attempt".to_string());
    }

    // ─── 6. 工作區路徑檢查 ───
    if let Some(ws) = workspace {
        if program.contains(':') || program.contains('/') || program.contains('\\') {
            let prog_path = Path::new(program);
            if !is_path_in_workspace(ws, prog_path) {
                return Err(format!(
                    "Program path is outside workspace: '{}' vs workspace '{}'",
                    program,
                    ws.display()
                ));
            }
        }
    }

    Ok(())
}

// ─── Shell 注入防護 ───────────────────────────────────────────────────────────

/// 檢查 program 是否為 shell 可執行程式（可能被用來間接注入）。
/// 若 program 是 `/bin/sh`, `/bin/bash`, `/bin/zsh` 等，在非 full_access 模式下拒絕。
pub fn is_shell_program(program: &str) -> bool {
    let base = program.split(['/', '\\']).next_back().unwrap_or(program);
    matches!(
        base.to_lowercase().as_str(),
        "sh" | "bash" | "zsh" | "fish" | "csh" | "ksh" | "cmd" | "cmd.exe"
            | "powershell" | "pwsh" | "pwsh.exe"
    )
}

/// 拒絕間接 shell 注入：非 full_access 模式下不允許透過 shell 執行程序。
pub fn check_indirect_shell_injection(
    program: &str,
    args: &[&str],
    full_access: bool,
) -> Result<(), String> {
    if !full_access && is_shell_program(program) {
        return Err(format!(
            "Shell program blocked in sandbox: '{}'. Use specific tools instead.",
            program
        ));
    }

    // 檢查參數中是否有命令替換模式
    for arg in args {
        if arg.contains("$(") || arg.contains('`') {
            return Err(format!(
                "Command substitution detected in arg: '{}'",
                arg
            ));
        }
    }

    Ok(())
}

// ─── 執行長度限制 ─────────────────────────────────────────────────────────────

/// 驗證指令字串總長度。
pub fn validate_cmd_length(program: &str, args: &[&str]) -> Result<(), String> {
    let total_len = program.len() + args.iter().map(|a| a.len() + 1).sum::<usize>();
    if total_len > MAX_CMD_LEN {
        return Err(format!(
            "Command too long: {} bytes (max {})",
            total_len, MAX_CMD_LEN
        ));
    }
    Ok(())
}

// ─── 主執行 ───────────────────────────────────────────────────────────────────

/// 執行 Command 並擷取 stdout / stderr / exit code。
fn capture_output(cmd: &mut Command) -> SandboxResult {
    let output = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(o) => o,
        Err(err) => {
            return SandboxResult {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: format!("執行進程失敗: {}", err),
                is_aligned_success: false,
            }
        }
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout).replace('\0', "");
    let stderr_str = String::from_utf8_lossy(&output.stderr).replace('\0', "");
    let exit_code = output.status.code();
    // 硬性對齊：只有 Exit Code == 0 且 stderr 為空才算成功
    let is_aligned_success = exit_code == Some(0) && stderr_str.trim().is_empty();
    SandboxResult {
        exit_code,
        stdout: stdout_str,
        stderr: stderr_str,
        is_aligned_success,
    }
}

/// 根據 shell 類型校準 UTF-8 環境後執行。
fn run_with_locale(cmd: &mut Command, _shell: &str, full_access: bool) -> SandboxResult {
    let is_windows = cfg!(target_os = "windows");

    if !is_windows {
        locale::set_locale_env(cmd, None, None);
    } else if !full_access {
        cmd.current_dir(std::env::current_dir().unwrap_or_default());
    }

    capture_output(cmd)
}

/// 構建 cmd.exe 命令字串。
fn build_cmd_string(program: &str, args: &[&str]) -> String {
    let mut s = program.to_string();
    for arg in args {
        s.push(' ');
        if arg.contains(' ') || arg.contains('"') {
            let escaped = arg.replace('"', "\"\"");
            s.push_str(&format!("\"{}\"", escaped));
        } else {
            s.push_str(arg);
        }
    }
    s
}

/// 構建 PowerShell 命令字串。
fn build_powershell_string(program: &str, args: &[&str]) -> String {
    let mut s = program.to_string();
    for arg in args {
        s.push(' ');
        if arg.contains(' ') || arg.contains('"') || arg.contains('\'') {
            s.push_str(&format!("'{}'", arg.replace('\'', "''")));
        } else {
            s.push_str(arg);
        }
    }
    s
}

/// 主沙盒執行入口。
///
/// 完整安全流程：
///  1. validate_sandbox_input → 白名單 + 參數 + 路徑穿越
///  2. check_indirect_shell_injection → 防止透過 shell 間接注入
///  3. validate_cmd_length → 防止巨量指令
///  4. 根據 shell 類型執行（注入 locale）
///  5. capture_output → Exit Code 對齊
pub fn run_in_sandbox(
    program: &str,
    args: &[&str],
    shell_preference: &str,
    full_access: bool,
    workspace: Option<&std::path::PathBuf>,
) -> SandboxResult {
    let is_windows = cfg!(target_os = "windows");
    let workspace_path: Option<&std::path::Path> = workspace.map(|v| &**v);

    // ─── 安全預檢 ───
    match validate_sandbox_input(program, args, full_access, workspace_path) {
        Ok(()) => {}
        Err(e) => {
            return SandboxResult {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: format!("安全驗證失敗: {}", e),
                is_aligned_success: false,
            }
        }
    }

    // ─── Shell 間接注入檢查 ───
    if let Err(e) = check_indirect_shell_injection(program, args, full_access) {
        return SandboxResult {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: format!("Shell 注入防護: {}", e),
            is_aligned_success: false,
        };
    }

    // ─── 指令長度 ───
    if let Err(e) = validate_cmd_length(program, args) {
        return SandboxResult {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: format!("指令長度限制: {}", e),
            is_aligned_success: false,
        };
    }

    // ─── 根據 shell 類型執行 ───
    let shell_lower = shell_preference.to_lowercase();

    if is_windows {
        match shell_lower.as_str() {
            "powershell" | "pwsh" => {
                let cmd_str =
                    locale::prepend_utf8_powershell(&build_powershell_string(program, args));
                let mut cmd = Command::new("powershell");
                cmd.arg("-Command").arg(cmd_str);
                run_with_locale(&mut cmd, "powershell", full_access)
            }
            _ => {
                // "cmd" 與預設：wrapped 在 cmd 中以確保 chcp 65001（解決中文亂碼）
                let cmd_str = locale::prepend_utf8_cmd(&build_cmd_string(program, args));
                let mut cmd = Command::new("cmd");
                cmd.arg("/C").arg(cmd_str);
                run_with_locale(&mut cmd, "cmd", full_access)
            }
        }
    } else {
        // Unix/macOS：直接執行，locale 環境變數在 run_with_locale 中設定
        let mut cmd = Command::new(program);
        cmd.args(args);
        run_with_locale(&mut cmd, "shell", full_access)
    }
}
