//! Git Worktree 隔離管理（Phase 5C）
//!
//! 多個 Generator 子代理在獨立 git worktree 中工作，共用同一份專案歷史，
//! 動不到彼此檔案。完成後 merge 回主分支；Evaluator 在唯讀 worktree 中驗證。
//!
//! 對齊 Loop Engineering Worktree 組件 + OpenAI Codex 子代理隔離模式。

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::WorktreeConfig;
use crate::no_window::NoWindowExt;

/// Worktree 管理器：為子代理建立隔離工作目錄 + 分支。
pub struct WorktreeManager {
    /// workspace 根目錄（git repo 根）
    workspace_path: PathBuf,
    config: WorktreeConfig,
}

/// 建立的 worktree 資訊。
#[derive(Debug, Clone)]
pub struct WorktreeHandle {
    /// worktree 的絕對路徑（子代理的工作目錄）
    pub path: PathBuf,
    /// 對應的 git 分支名稱
    pub branch: String,
    /// 子代理識別碼（用於分支命名與清理）
    pub agent_id: String,
}

impl WorktreeManager {
    pub fn new(workspace_path: PathBuf, config: WorktreeConfig) -> Self {
        Self {
            workspace_path,
            config,
        }
    }

    /// 解析 worktree 根目錄（相對路徑以 workspace 為基準）。
    fn worktree_root(&self) -> PathBuf {
        let base = Path::new(&self.config.base_dir);
        if base.is_absolute() {
            base.to_path_buf()
        } else {
            self.workspace_path.join(base)
        }
    }

    /// 確認 workspace 是 git repo。
    pub fn is_git_repo(&self) -> bool {
        let output = Command::new("git").no_window()
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&self.workspace_path)
            .output();
        match output {
            Ok(o) => o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true",
            Err(_) => false,
        }
    }

    /// 為子代理建立隔離 worktree + 新分支。
    ///
    /// 流程：
    /// 1. 建立 worktree 根目錄（若不存）
    /// 2. 從目前 HEAD 建立新分支 `<prefix>-<agent_id>`
    /// 3. `git worktree add <path> <branch>`
    pub fn create(&self, agent_id: &str) -> Result<WorktreeHandle, String> {
        if !self.is_git_repo() {
            return Err(format!(
                "workspace {:?} 不是 git repo，無法建立 worktree",
                self.workspace_path
            ));
        }

        let root = self.worktree_root();
        std::fs::create_dir_all(&root)
            .map_err(|e| format!("建立 worktree 根目錄失敗: {}", e))?;

        let branch = format!("{}-{}", self.config.branch_prefix, agent_id);
        let wt_path = root.join(agent_id);

        // 若 worktree 已存在，先移除（冪等性）
        if wt_path.exists() {
            let _ = self.remove_worktree_dir(&wt_path, &branch);
        }

        // 建立分支 + worktree（一步完成：git worktree add -b <branch> <path>）
        let output = Command::new("git").no_window()
            .args([
                "worktree",
                "add",
                "-b",
                &branch,
                wt_path.to_string_lossy().as_ref(),
            ])
            .current_dir(&self.workspace_path)
            .output()
            .map_err(|e| format!("git worktree add 執行失敗: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git worktree add 失敗: {}", stderr.trim()));
        }

        Ok(WorktreeHandle {
            path: wt_path,
            branch,
            agent_id: agent_id.to_string(),
        })
    }

    /// 將 worktree 的變更 merge 回主分支。
    ///
    /// 流程：
    /// 1. 在 worktree 中 commit 所有變更（若有的話）
    /// 2. 切回主 workspace
    /// 3. `git merge <branch>`
    pub fn merge(&self, handle: &WorktreeHandle) -> Result<(), String> {
        // 在 worktree 中 stage + commit
        let commit_msg = format!("agent-{}: sub-agent changes", handle.agent_id);
        for args in [
            &["add", "-A"][..],
            &["commit", "-m", commit_msg.as_str(), "--allow-empty"][..],
        ] {
            let output = Command::new("git").no_window()
                .args(args)
                .current_dir(&handle.path)
                .output()
                .map_err(|e| format!("git {:?} 執行失敗: {}", args, e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // commit 無變更不是錯誤
                if !args.contains(&"commit") || !stderr.contains("nothing to commit") {
                    let _ = stderr; // 避免 unused
                }
            }
        }

        // 回主 workspace merge
        let output = Command::new("git").no_window()
            .args(["merge", "--no-edit", &handle.branch])
            .current_dir(&self.workspace_path)
            .output()
            .map_err(|e| format!("git merge 執行失敗: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // merge 衝突時 abort 並回報
            let _ = Command::new("git").no_window()
                .args(["merge", "--abort"])
                .current_dir(&self.workspace_path)
                .output();
            return Err(format!("git merge 失敗（已 abort）: {}", stderr.trim()));
        }

        Ok(())
    }

    /// 清理 worktree + 分支。
    pub fn cleanup(&self, handle: &WorktreeHandle) -> Result<(), String> {
        self.remove_worktree_dir(&handle.path, &handle.branch)
    }

    /// 內部：移除 worktree 目錄 + 刪除分支。
    fn remove_worktree_dir(&self, path: &Path, branch: &str) -> Result<(), String> {
        // git worktree remove（會清 .git/worktrees 設定）
        let _ = Command::new("git").no_window()
            .args(["worktree", "remove", "--force", path.to_string_lossy().as_ref()])
            .current_dir(&self.workspace_path)
            .output();

        // 若目錄仍在，強制刪除
        if path.exists() {
            let _ = std::fs::remove_dir_all(path);
        }

        // 刪除分支
        let _ = Command::new("git").no_window()
            .args(["branch", "-D", branch])
            .current_dir(&self.workspace_path)
            .output();

        Ok(())
    }

    /// 列出目前所有 agnes worktree（以 branch_prefix 過濾）。
    pub fn list(&self) -> Vec<WorktreeHandle> {
        let output = Command::new("git").no_window()
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&self.workspace_path)
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                text.lines()
                    .filter(|l| l.starts_with("worktree "))
                    .filter_map(|l| {
                        let path = l.strip_prefix("worktree ")?.trim();
                        let p = PathBuf::from(path);
                        // 從路徑推導 agent_id（最後一段）
                        let agent_id = p.file_name()?.to_string_lossy().to_string();
                        if agent_id.starts_with(&self.config.branch_prefix)
                            || self.worktree_root().join(&agent_id) == p
                        {
                            let branch = format!("{}-{}", self.config.branch_prefix, agent_id);
                            Some(WorktreeHandle {
                                path: p,
                                branch,
                                agent_id,
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_config_defaults() {
        let cfg = WorktreeConfig::default();
        assert_eq!(cfg.base_dir, ".agnes-worktrees");
        assert_eq!(cfg.branch_prefix, "agnes-agent");
        assert!(cfg.auto_cleanup);
    }

    #[test]
    fn worktree_root_relative() {
        let mgr = WorktreeManager::new(
            PathBuf::from("/tmp/project"),
            WorktreeConfig::default(),
        );
        assert_eq!(
            mgr.worktree_root(),
            PathBuf::from("/tmp/project/.agnes-worktrees")
        );
    }

    #[test]
    fn worktree_root_absolute() {
        let cfg = WorktreeConfig {
            base_dir: "/tmp/wt".to_string(),
            ..WorktreeConfig::default()
        };
        let mgr = WorktreeManager::new(PathBuf::from("/tmp/project"), cfg);
        assert_eq!(mgr.worktree_root(), PathBuf::from("/tmp/wt"));
    }

    #[test]
    fn is_git_repo_nonexistent() {
        let mgr = WorktreeManager::new(
            PathBuf::from("/nonexistent/path/that/does/not/exist"),
            WorktreeConfig::default(),
        );
        assert!(!mgr.is_git_repo());
    }
}
