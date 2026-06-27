//! Windows 無視窗執行 helper。
//!
//! Windows 上 std::process::Command 預設會為 console 子系統程式建立新的 console 視窗，
//! 導致使用者桌面不斷彈出 CMD/PowerShell 視窗。本模組提供 `silent_command()` 包裝，
//! 統一注入 `CREATE_NO_WINDOW` flag，讓所有子進程在背景靜默執行。
//!
//! 用法：
//! ```ignore
//! use crate::no_window::silent_command;
//! let output = silent_command("cargo", &["test"]).output().unwrap();
//! ```

use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 建構一個已注入無視窗 flag 的 Command。
///
/// Windows：設定 `CREATE_NO_WINDOW` creation flag。
/// Unix：直接回傳普通 Command（無需特殊處理）。
pub fn silent_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Trait：為既有 std::process::Command 注入無視窗 flag（用於已建構的 Command）。
pub trait NoWindowExt {
    /// 在 Windows 上注入 CREATE_NO_WINDOW flag；Unix 上無操作。
    fn no_window(&mut self) -> &mut Self;
}

impl NoWindowExt for Command {
    #[cfg(target_os = "windows")]
    fn no_window(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(CREATE_NO_WINDOW);
        self
    }

    #[cfg(not(target_os = "windows"))]
    fn no_window(&mut self) -> &mut Self {
        self
    }
}

/// Trait：為 tokio::process::Command 注入無視窗 flag。
pub trait NoWindowExtTokio {
    /// 在 Windows 上注入 CREATE_NO_WINDOW flag；Unix 上無操作。
    fn no_window(&mut self) -> &mut Self;
}

impl NoWindowExtTokio for tokio::process::Command {
    #[cfg(target_os = "windows")]
    fn no_window(&mut self) -> &mut Self {
        // tokio::process::Command 在 Windows 上支援 creation_flags（透過 std os 桥接）
        self.creation_flags(CREATE_NO_WINDOW);
        self
    }

    #[cfg(not(target_os = "windows"))]
    fn no_window(&mut self) -> &mut Self {
        self
    }
}
