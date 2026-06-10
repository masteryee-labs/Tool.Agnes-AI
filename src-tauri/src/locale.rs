//! 環境語系自動校準 (Locale Auto-Calibration)
//!
//! 在執行作業系統 Shell/Cmd/PowerShell 任務前，主動進行語系環境探針，
//! 確保所有輸出與搜尋結果解碼皆為 100% 正確 UTF-8。

use std::process::Command;

// ─── Windows shell 校準 ───────────────────────────────────────────────────────

/// 為 cmd.exe 注入 chcp 65001 前綴，確保 UTF-8 輸出。
pub fn prepend_utf8_cmd(command: &str) -> String {
    format!("chcp 65001 >nul && {}", command)
}

/// 為 PowerShell 注入 [Console]::OutputEncoding 設定。
pub fn prepend_utf8_powershell(command: &str) -> String {
    format!(
        "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $OutputEncoding=[System.Text.Encoding]::UTF8; {}",
        command
    )
}

// ─── Unix/macOS shell 校準 ────────────────────────────────────────────────────

/// 為 Unix shell 注入 LANG / LC_ALL 環境變數。
pub fn set_locale_env(cmd: &mut Command, lang: Option<&str>, lc_all: Option<&str>) {
    if let Some(l) = lang {
        cmd.env("LANG", l);
    } else {
        cmd.env("LANG", "zh_TW.UTF-8");
    }
    if let Some(l) = lc_all {
        cmd.env("LC_ALL", l);
    } else {
        cmd.env("LC_ALL", "zh_TW.UTF-8");
    }
    cmd.env("PYTHONIOENCODING", "utf-8");
}

// ─── 探針 ─────────────────────────────────────────────────────────────────────

/// 系統語系探針結果
#[derive(Debug, Clone)]
pub struct LocaleProbe {
    pub output_cp: u32,
    pub acp: u32,
    pub needs_calibration: bool,
}

impl LocaleProbe {
    pub fn is_utf8_ready(&self) -> bool {
        !self.needs_calibration
    }
    pub fn is_utf8_cp(&self) -> bool {
        self.output_cp == 65001
    }
}

/// 探測系統預設編碼。
///
/// Windows 會讀取註冊表 HKLM\SYSTEM\CurrentControlSet\Control\Nls\CodePage\ACP
/// 並檢查 GetConsoleOutputCP。若任一非 65001 則需校準。
pub fn probe_system_locale() -> LocaleProbe {
    #[cfg(target_os = "windows")]
    {
        use std::sync::OnceLock;

        #[link(name = "kernel32")]
        extern "system" {
            fn GetConsoleOutputCP() -> u32;
            fn GetACP() -> u32;
        }

        static PROBE: OnceLock<LocaleProbe> = OnceLock::new();
        PROBE
            .get_or_init(|| {
                let output_cp = unsafe { GetConsoleOutputCP() };
                let acp = unsafe { GetACP() };
                LocaleProbe {
                    output_cp,
                    acp,
                    needs_calibration: output_cp != 65001 || acp != 65001,
                }
            })
            .clone()
    }

    #[cfg(not(target_os = "windows"))]
    {
        LocaleProbe {
            output_cp: 0,
            acp: 0,
            needs_calibration: false,
        }
    }
}

// ─── 安全校準 ─────────────────────────────────────────────────────────────────

/// 在執行命令前先校準語系。
/// 若系統已為 UTF-8 則直接返回原指令，否則注入校準前綴。
pub fn calibrated_command(shell: &str, command: &str) -> String {
    let probe = probe_system_locale();
    if probe.is_utf8_ready() {
        command.to_string()
    } else {
        match shell.to_lowercase().as_str() {
            "powershell" | "pwsh" => prepend_utf8_powershell(command),
            "cmd" | "command prompt" => prepend_utf8_cmd(command),
            _ => command.to_string(),
        }
    }
}
