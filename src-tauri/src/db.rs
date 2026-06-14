#![allow(dead_code)]

use rusqlite::{params, Connection, Result};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::path::Path;

// ─── Task state constants ────────────────────────────────────────────────────

pub const TASK_STATUS_PENDING: &str = "PENDING";
pub const TASK_STATUS_IN_PROGRESS: &str = "IN_PROGRESS";
pub const TASK_STATUS_SUCCESS: &str = "SUCCESS";
pub const TASK_STATUS_FAILED: &str = "FAILED";
pub const TASK_STATUS_CANCELLED: &str = "CANCELLED";

// ─── Data models ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub status: String,
    pub payload: String,
    pub created_at: String,
    pub updated_at: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub id: i32,
    pub task_id: String,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub run_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLog {
    pub id: i32,
    pub task_id: String,
    pub agent_name: String,
    pub verdict: String,
    pub reason: String,
    pub audited_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub folders: String,
    pub work_mode: Option<String>,
    pub shell: Option<String>,
    pub language: Option<String>,
    pub require_approval: Option<bool>,
    pub default_permissions: Option<bool>,
    pub auto_review: Option<bool>,
    pub full_access: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationMessage {
    pub id: i32,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// 單筆檔案變更：write_file 工具寫入前後的完整內容快照（GUI 右側 diff 面板資料來源）。
#[derive(Debug, Clone)]
pub struct FileChangeRecord {
    pub id: i64,
    pub conversation_id: String,
    pub file_path: String,
    pub before_content: String,
    pub after_content: String,
    pub written_at: String,
}

// ─── DB initialization ───────────────────────────────────────────────────────

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'PENDING',
            payload TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            project_id TEXT
        );

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            folders TEXT NOT NULL,
            work_mode TEXT,
            shell TEXT,
            language TEXT,
            require_approval INTEGER DEFAULT 1,
            default_permissions INTEGER DEFAULT 1,
            auto_review INTEGER DEFAULT 0,
            full_access INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS execution_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL,
            command TEXT NOT NULL,
            stdout TEXT NOT NULL DEFAULT '',
            stderr TEXT NOT NULL DEFAULT '',
            exit_code INTEGER,
            run_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            verdict TEXT NOT NULL,
            reason TEXT NOT NULL,
            audited_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT '',
            project_id TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS conversation_audits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            verdict TEXT NOT NULL,
            reason TEXT NOT NULL,
            audited_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS conversation_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(conversation_id) REFERENCES conversations(id)
        );

        CREATE TABLE IF NOT EXISTS token_ledger (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT,
            model_name TEXT NOT NULL,
            prompt_tokens INTEGER NOT NULL,
            completion_tokens INTEGER NOT NULL,
            total_tokens INTEGER NOT NULL,
            cost_usd REAL NOT NULL,
            warning_triggered INTEGER NOT NULL DEFAULT 0,
            logged_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS repair_table (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            error_code TEXT NOT NULL,
            instruction TEXT NOT NULL,
            repaired_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_index USING fts5(
            file_path UNINDEXED,
            tag,
            content
        );

        CREATE TABLE IF NOT EXISTS file_changes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            before_content TEXT NOT NULL,
            after_content TEXT NOT NULL,
            written_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS distill_markers (
            conv_hash TEXT PRIMARY KEY,
            tokens INTEGER NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_execution_logs_task
            ON execution_logs(task_id);

        CREATE INDEX IF NOT EXISTS idx_audit_logs_task
            ON audit_logs(task_id);

        CREATE INDEX IF NOT EXISTS idx_messages_conv
            ON conversation_messages(conversation_id);

        CREATE INDEX IF NOT EXISTS idx_conv_audits
            ON conversation_audits(conversation_id);

        CREATE INDEX IF NOT EXISTS idx_file_changes_conv
            ON file_changes(conversation_id);
        ",
    )?;

    // 既有資料庫遷移：舊版 conversations 無 project_id 欄。
    // ALTER 對已有該欄的表會失敗——該錯誤即「無需遷移」，安全忽略。
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN project_id TEXT", []);
    let _ = conn.execute("ALTER TABLE token_ledger ADD COLUMN warning_triggered INTEGER NOT NULL DEFAULT 0", []);

    Ok(())
}

/// 全域模式 Session 的 project_id 哨兵值（不對應任何 projects 列）。
pub const GLOBAL_PROJECT_ID: &str = "global";

/// 多執行緒同檔競態下的寫入等待上限（rusqlite 預設 0 = 立即回 SQLITE_BUSY）
pub const DB_BUSY_TIMEOUT_MS: u64 = 3000;

/// Open a new Connection at the given path. Returns an error if the DB cannot
/// be created / opened. Calls `init_db` automatically.
pub fn open_connection(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_millis(DB_BUSY_TIMEOUT_MS))?;
    init_db(&conn)?;
    Ok(conn)
}

// ─── 蒸餾水位記號（防止超過閾值後每輪重複蒸餾燒 token）──────────────────────

/// 取得該對話上次蒸餾時的 token 水位（無記錄回傳 0）。
pub fn get_distill_marker(conn: &Connection, conv_hash: &str) -> Result<i64> {
    let result = conn.query_row(
        "SELECT tokens FROM distill_markers WHERE conv_hash = ?1",
        params![conv_hash],
        |row| row.get(0),
    );
    match result {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        Err(e) => Err(e),
    }
}

/// 寫入/更新該對話的蒸餾水位。
pub fn set_distill_marker(conn: &Connection, conv_hash: &str, tokens: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO distill_markers (conv_hash, tokens, updated_at)
         VALUES (?1, ?2, CURRENT_TIMESTAMP)
         ON CONFLICT(conv_hash) DO UPDATE SET tokens = ?2, updated_at = CURRENT_TIMESTAMP",
        params![conv_hash, tokens],
    )?;
    Ok(())
}

// ─── Task CRUD ────────────────────────────────────────────────────────────────

pub fn create_task(
    conn: &Connection,
    name: &str,
    payload: &str,
    project_id: Option<&str>,
) -> Result<String> {
    let task_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO tasks (id, name, status, payload, project_id) \
         VALUES (?1, ?2, 'PENDING', ?3, ?4)",
        params![task_id, name, payload, project_id],
    )?;
    Ok(task_id)
}

pub fn update_task_status(conn: &Connection, task_id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![status, task_id],
    )?;
    Ok(())
}

pub fn get_task_status(conn: &Connection, task_id: &str) -> Result<String> {
    let mut stmt = conn.prepare("SELECT status FROM tasks WHERE id = ?1")?;
    stmt.query_row(params![task_id], |row| row.get(0))
}

pub fn get_all_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, status, payload, created_at, updated_at, project_id \
         FROM tasks ORDER BY created_at ASC"
    )?;
    query_all_tasks(&mut stmt)
}

fn query_all_tasks(stmt: &mut rusqlite::Statement) -> Result<Vec<Task>> {
    let task_iter = stmt.query_map([], |row| {
        Ok(Task {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row.get(2)?,
            payload: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            project_id: row.get(6)?,
        })
    })?;
    task_iter.collect()
}

pub fn get_tasks_for_project(conn: &Connection, project_id: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, status, payload, created_at, updated_at, project_id \
         FROM tasks \
         WHERE project_id = ?1 \
         ORDER BY created_at ASC"
    )?;
    query_project_tasks(&mut stmt, project_id)
}

fn query_project_tasks(stmt: &mut rusqlite::Statement, project_id: &str) -> Result<Vec<Task>> {
    let task_iter = stmt.query_map(params![project_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row.get(2)?,
            payload: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            project_id: row.get(6)?,
        })
    })?;
    task_iter.collect()
}

// ─── Execution log ────────────────────────────────────────────────────────────

pub fn add_execution_log(
    conn: &Connection,
    task_id: &str,
    command: &str,
    stdout: &str,
    stderr: &str,
    exit_code: Option<i32>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO execution_logs (task_id, command, stdout, stderr, exit_code) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![task_id, command, stdout, stderr, exit_code],
    )?;
    Ok(())
}

pub fn get_execution_logs_for_task(conn: &Connection, task_id: &str) -> Result<Vec<ExecutionLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, command, stdout, stderr, exit_code, run_at \
         FROM execution_logs \
         WHERE task_id = ?1 \
         ORDER BY run_at ASC"
    )?;
    query_exec_logs(&mut stmt, task_id)
}

fn query_exec_logs(stmt: &mut rusqlite::Statement, task_id: &str) -> Result<Vec<ExecutionLog>> {
    let log_iter = stmt.query_map(params![task_id], |row| {
        Ok(ExecutionLog {
            id: row.get(0)?,
            task_id: row.get(1)?,
            command: row.get(2)?,
            stdout: row.get(3)?,
            stderr: row.get(4)?,
            exit_code: row.get(5)?,
            run_at: row.get(6)?,
        })
    })?;
    log_iter.collect()
}

// ─── Audit log ────────────────────────────────────────────────────────────────

pub fn add_audit_log(
    conn: &Connection,
    task_id: &str,
    agent_name: &str,
    verdict: &str,
    reason: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO audit_logs (task_id, agent_name, verdict, reason) \
         VALUES (?1, ?2, ?3, ?4)",
        params![task_id, agent_name, verdict, reason],
    )?;
    Ok(())
}

/// 取代式寫入對話審查批次：清掉舊批次後整批寫入最新一輪（22 筆）。
/// 注意：不可重用 audit_logs——其 task_id 外鍵指向 tasks(id)，而 rusqlite bundled
/// SQLite 預設啟用外鍵；對話 id 不在 tasks 表，寫入會以 FOREIGN KEY constraint 失敗
///（GUI 實機 QA 抓到的真實缺陷）。
pub fn replace_conversation_audits(
    conn: &Connection,
    conversation_id: &str,
    audits: &[(String, String, String)], // (agent_name, verdict, reason)
) -> Result<()> {
    conn.execute(
        "DELETE FROM conversation_audits WHERE conversation_id = ?1",
        params![conversation_id],
    )?;
    for (agent_name, verdict, reason) in audits {
        conn.execute(
            "INSERT INTO conversation_audits (conversation_id, agent_name, verdict, reason) \
             VALUES (?1, ?2, ?3, ?4)",
            params![conversation_id, agent_name, verdict, reason],
        )?;
    }
    Ok(())
}

/// 讀回該對話最新一輪審查（agent_name, verdict, reason）。
pub fn get_conversation_audits(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT agent_name, verdict, reason FROM conversation_audits \
         WHERE conversation_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect()
}

pub fn get_audit_logs_for_task(conn: &Connection, task_id: &str) -> Result<Vec<AuditLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, agent_name, verdict, reason, audited_at \
         FROM audit_logs \
         WHERE task_id = ?1 \
         ORDER BY audited_at ASC"
    )?;
    query_audit_logs(&mut stmt, task_id)
}

fn query_audit_logs(stmt: &mut rusqlite::Statement, task_id: &str) -> Result<Vec<AuditLog>> {
    let log_iter = stmt.query_map(params![task_id], |row| {
        Ok(AuditLog {
            id: row.get(0)?,
            task_id: row.get(1)?,
            agent_name: row.get(2)?,
            verdict: row.get(3)?,
            reason: row.get(4)?,
            audited_at: row.get(5)?,
        })
    })?;
    log_iter.collect()
}

// ─── Conversation persistence ─────────────────────────────────────────────────

pub fn create_conversation(
    conn: &Connection,
    title: &str,
    project_id: Option<&str>,
) -> Result<String> {
    let conv_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO conversations (id, title, project_id) VALUES (?1, ?2, ?3)",
        params![conv_id, title, project_id],
    )?;
    Ok(conv_id)
}

pub fn add_conversation_message(
    conn: &Connection,
    conversation_id: &str,
    role: &str,
    content: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO conversation_messages (conversation_id, role, content) \
         VALUES (?1, ?2, ?3)",
        params![conversation_id, role, content],
    )?;
    // Also update the conversation's updated_at
    conn.execute(
        "UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![conversation_id],
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub project_id: Option<String>,
}

pub fn get_conversations(conn: &Connection) -> Result<Vec<ConversationSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, updated_at, project_id FROM conversations ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ConversationSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            updated_at: row.get(2)?,
            project_id: row.get(3)?,
        })
    })?;
    let mut results = Vec::new();
    for row_result in rows {
        results.push(row_result?);
    }
    Ok(results)
}

/// 歷史資料補綁：把尚無歸屬的舊對話掛到指定專案下（升級既有資料庫用）。
pub fn assign_orphan_conversations(conn: &Connection, project_id: &str) -> Result<usize> {
    let n = conn.execute(
        "UPDATE conversations SET project_id = ?1 WHERE project_id IS NULL",
        params![project_id],
    )?;
    Ok(n)
}

pub fn get_messages_for_conversation(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<ConversationMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, created_at \
         FROM conversation_messages \
         WHERE conversation_id = ?1 \
         ORDER BY created_at ASC"
    )?;
    query_conv_messages(&mut stmt, conversation_id)
}

fn query_conv_messages(stmt: &mut rusqlite::Statement, conversation_id: &str) -> Result<Vec<ConversationMessage>> {
    let msg_iter = stmt.query_map(params![conversation_id], |row| {
        Ok(ConversationMessage {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    msg_iter.collect()
}

// ─── File change tracking（write_file before/after 快照，供 GUI diff 面板）────

/// 超過 max_bytes 時截斷至 UTF-8 字元邊界（向下對齊）並附截斷標記；
/// 未超過時原樣借用，零拷貝。
fn truncate_file_change_content(content: &str, max_bytes: usize) -> std::borrow::Cow<'_, str> {
    if content.len() <= max_bytes {
        return std::borrow::Cow::Borrowed(content);
    }
    let mut cut = max_bytes;
    while cut > 0 && !content.is_char_boundary(cut) {
        cut -= 1;
    }
    std::borrow::Cow::Owned(format!(
        "{}{}",
        &content[..cut],
        crate::config::FILE_CHANGE_TRUNCATION_MARKER
    ))
}

/// 記錄一次檔案寫入的 before/after 快照，並套用保留策略（FileChangesConfig）：
/// 單筆內容超限截斷、該對話超出筆數上限時刪最舊。
/// written_at 由 SQLite CURRENT_TIMESTAMP 生成，與其他表的時間戳慣例一致。
/// 接收 db_path 而非 Connection：呼叫點（agent.rs execute_tool）不持有連線。
pub fn add_file_change(
    db_path: &Path,
    conversation_id: &str,
    file_path: &str,
    before: &str,
    after: &str,
    limits: &crate::config::FileChangesConfig,
) -> Result<()> {
    let conn = open_connection(db_path)?;
    let before = truncate_file_change_content(before, limits.content_max_bytes);
    let after = truncate_file_change_content(after, limits.content_max_bytes);
    conn.execute(
        "INSERT INTO file_changes (conversation_id, file_path, before_content, after_content, written_at) \
         VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
        params![conversation_id, file_path, before.as_ref(), after.as_ref()],
    )?;
    // 每對話保留上限：只留 id 最大（最新）的 keep_per_conversation 筆
    conn.execute(
        "DELETE FROM file_changes \
         WHERE conversation_id = ?1 \
           AND id NOT IN ( \
               SELECT id FROM file_changes \
               WHERE conversation_id = ?1 \
               ORDER BY id DESC LIMIT ?2)",
        params![conversation_id, limits.keep_per_conversation as i64],
    )?;
    Ok(())
}

/// 讀回該對話的全部檔案變更，按寫入順序（id ASC）。
pub fn get_file_changes(db_path: &Path, conversation_id: &str) -> Result<Vec<FileChangeRecord>> {
    let conn = open_connection(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, file_path, before_content, after_content, written_at \
         FROM file_changes \
         WHERE conversation_id = ?1 \
         ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok(FileChangeRecord {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            file_path: row.get(2)?,
            before_content: row.get(3)?,
            after_content: row.get(4)?,
            written_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn delete_conversation(conn: &Connection, conversation_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM conversation_messages WHERE conversation_id = ?1",
        params![conversation_id],
    )?;
    // 級聯清理：file_changes 對話亡則快照亡，否則孤兒列永久佔空間
    conn.execute(
        "DELETE FROM file_changes WHERE conversation_id = ?1",
        params![conversation_id],
    )?;
    // 同理級聯 conversation_audits：每對話最多一輪 22 筆審查列，漏刪即成孤兒
    conn.execute(
        "DELETE FROM conversation_audits WHERE conversation_id = ?1",
        params![conversation_id],
    )?;
    conn.execute(
        "DELETE FROM conversations WHERE id = ?1",
        params![conversation_id],
    )?;
    Ok(())
}

// ─── Project CRUD ─────────────────────────────────────────────────────────────

pub fn create_project(
    conn: &Connection,
    name: &str,
    folders_json: &str,
) -> Result<String> {
    let project_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO projects (id, name, folders) VALUES (?1, ?2, ?3)",
        params![project_id, name, folders_json],
    )?;
    Ok(project_id)
}

pub fn get_all_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, folders, work_mode, shell, language, \
                require_approval, default_permissions, auto_review, full_access, \
                created_at, updated_at \
         FROM projects \
         ORDER BY created_at ASC"
    )?;
    query_all_projects(&mut stmt)
}

fn query_all_projects(stmt: &mut rusqlite::Statement) -> Result<Vec<Project>> {
    let project_iter = stmt.query_map([], |row| {
        let req_app: Option<i32> = row.get(6)?;
        let def_perm: Option<i32> = row.get(7)?;
        let auto_rev: Option<i32> = row.get(8)?;
        let full_acc: Option<i32> = row.get(9)?;
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            folders: row.get(2)?,
            work_mode: row.get(3)?,
            shell: row.get(4)?,
            language: row.get(5)?,
            require_approval: req_app.map(|v| v != 0),
            default_permissions: def_perm.map(|v| v != 0),
            auto_review: auto_rev.map(|v| v != 0),
            full_access: full_acc.map(|v| v != 0),
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    project_iter.collect()
}

pub fn get_project(conn: &Connection, project_id: &str) -> Result<Project> {
    let mut stmt = conn.prepare(
        "SELECT id, name, folders, work_mode, shell, language, \
                require_approval, default_permissions, auto_review, full_access, \
                created_at, updated_at \
         FROM projects \
         WHERE id = ?1"
    )?;
    let req_app: Option<i32> = stmt.query_row(params![project_id], |row| row.get(6))?;
    let def_perm: Option<i32> = stmt.query_row(params![project_id], |row| row.get(7))?;
    let auto_rev: Option<i32> = stmt.query_row(params![project_id], |row| row.get(8))?;
    let full_acc: Option<i32> = stmt.query_row(params![project_id], |row| row.get(9))?;
    Ok(Project {
        id: stmt.query_row(params![project_id], |row| row.get(0))?,
        name: stmt.query_row(params![project_id], |row| row.get(1))?,
        folders: stmt.query_row(params![project_id], |row| row.get(2))?,
        work_mode: stmt.query_row(params![project_id], |row| row.get(3))?,
        shell: stmt.query_row(params![project_id], |row| row.get(4))?,
        language: stmt.query_row(params![project_id], |row| row.get(5))?,
        require_approval: req_app.map(|v| v != 0),
        default_permissions: def_perm.map(|v| v != 0),
        auto_review: auto_rev.map(|v| v != 0),
        full_access: full_acc.map(|v| v != 0),
        created_at: stmt.query_row(params![project_id], |row| row.get(10))?,
        updated_at: stmt.query_row(params![project_id], |row| row.get(11))?,
    })
}

pub fn delete_project(conn: &Connection, project_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM projects WHERE id = ?1",
        params![project_id],
    )?;
    conn.execute(
        "UPDATE tasks SET project_id = NULL WHERE project_id = ?1",
        params![project_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_project_settings(
    conn: &Connection,
    project_id: &str,
    work_mode: Option<String>,
    shell: Option<String>,
    language: Option<String>,
    require_approval: Option<bool>,
    default_permissions: Option<bool>,
    auto_review: Option<bool>,
    full_access: Option<bool>,
) -> Result<()> {
    conn.execute(
        "UPDATE projects SET \
            work_mode = ?1, \
            shell = ?2, \
            language = ?3, \
            require_approval = ?4, \
            default_permissions = ?5, \
            auto_review = ?6, \
            full_access = ?7, \
            updated_at = CURRENT_TIMESTAMP \
         WHERE id = ?8",
        params![
            work_mode,
            shell,
            language,
            require_approval.map(|v| if v { 1 } else { 0 }),
            default_permissions.map(|v| if v { 1 } else { 0 }),
            auto_review.map(|v| if v { 1 } else { 0 }),
            full_access.map(|v| if v { 1 } else { 0 }),
            project_id
        ],
    )?;
    Ok(())
}

pub fn update_project_folders(
    conn: &Connection,
    project_id: &str,
    folders_json: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE projects SET folders = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![folders_json, project_id],
    )?;
    Ok(())
}

pub fn update_project_name(
    conn: &Connection,
    project_id: &str,
    name: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE projects SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![name, project_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn add_token_log(
    conn: &Connection,
    task_id: Option<&str>,
    model_name: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost_usd: f64,
    warning_triggered: i32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO token_ledger (task_id, model_name, prompt_tokens, completion_tokens, total_tokens, cost_usd, warning_triggered) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![task_id, model_name, prompt_tokens, completion_tokens, total_tokens, cost_usd, warning_triggered],
    )?;
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod file_change_tests {
    use super::*;
    use crate::config::{FileChangesConfig, FILE_CHANGE_TRUNCATION_MARKER};
    use std::path::PathBuf;

    /// 每測試獨立暫存 DB（pid + 奈秒時戳），避免並行測試互踩同一檔案。
    fn temp_db(tag: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "agnes_test_fc_{}_{}_{}.db",
            tag,
            std::process::id(),
            unique
        ))
    }

    /// 測試專用的小額度保留策略。
    fn limits(content_max_bytes: usize, keep_per_conversation: usize) -> FileChangesConfig {
        FileChangesConfig { content_max_bytes, keep_per_conversation }
    }

    #[test]
    fn file_change_roundtrip_ordered_by_id() {
        let db = temp_db("roundtrip");
        let lim = FileChangesConfig::default();
        add_file_change(&db, "conv-1", "src/a.rs", "old a", "new a", &lim).unwrap();
        add_file_change(&db, "conv-1", "src/b.rs", "", "fresh file", &lim).unwrap();

        let changes = get_file_changes(&db, "conv-1").unwrap();
        assert_eq!(changes.len(), 2);
        // id ASC ＝ 寫入順序
        assert!(changes[0].id < changes[1].id);
        assert_eq!(changes[0].file_path, "src/a.rs");
        assert_eq!(changes[0].before_content, "old a");
        assert_eq!(changes[0].after_content, "new a");
        assert_eq!(changes[1].before_content, "");
        assert!(!changes[0].written_at.is_empty());

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn file_changes_isolated_per_conversation() {
        let db = temp_db("isolated");
        add_file_change(&db, "conv-a", "x.txt", "1", "2", &FileChangesConfig::default()).unwrap();

        // 無記錄的對話 → 空集合（非錯誤）
        assert!(get_file_changes(&db, "conv-b").unwrap().is_empty());
        assert_eq!(get_file_changes(&db, "conv-a").unwrap().len(), 1);

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn oversized_content_truncated_with_marker() {
        let db = temp_db("truncate");
        // before 超限（6 > 4）截斷；after 未超限原樣保存
        add_file_change(&db, "conv-1", "big.txt", "abcdef", "tiny", &limits(4, 10)).unwrap();

        let changes = get_file_changes(&db, "conv-1").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].before_content,
            format!("abcd{}", FILE_CHANGE_TRUNCATION_MARKER)
        );
        assert_eq!(changes[0].after_content, "tiny");

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn content_at_exact_limit_not_truncated() {
        let db = temp_db("exact");
        add_file_change(&db, "conv-1", "x.txt", "abcd", "abcd", &limits(4, 10)).unwrap();

        let changes = get_file_changes(&db, "conv-1").unwrap();
        assert_eq!(changes[0].before_content, "abcd");
        assert!(!changes[0].after_content.contains(FILE_CHANGE_TRUNCATION_MARKER));

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn truncation_respects_utf8_boundary() {
        let db = temp_db("utf8");
        // 「中文字」共 9 bytes；上限 4 落在第二字中間 → 向下對齊到 3（「中」）
        add_file_change(&db, "conv-1", "cjk.txt", "中文字", "", &limits(4, 10)).unwrap();

        let changes = get_file_changes(&db, "conv-1").unwrap();
        assert_eq!(
            changes[0].before_content,
            format!("中{}", FILE_CHANGE_TRUNCATION_MARKER)
        );

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn per_conversation_retention_keeps_newest() {
        let db = temp_db("retention");
        let lim = limits(1024, 3);
        for i in 0..5 {
            let path = format!("f{}.txt", i);
            add_file_change(&db, "conv-1", &path, "old", "new", &lim).unwrap();
        }
        // 不同對話不受 conv-1 修剪影響
        add_file_change(&db, "conv-2", "other.txt", "a", "b", &lim).unwrap();

        let changes = get_file_changes(&db, "conv-1").unwrap();
        assert_eq!(changes.len(), 3);
        // 留下的必須是最新三筆（f2/f3/f4），最舊兩筆已刪
        let paths: Vec<&str> = changes.iter().map(|c| c.file_path.as_str()).collect();
        assert_eq!(paths, vec!["f2.txt", "f3.txt", "f4.txt"]);
        assert_eq!(get_file_changes(&db, "conv-2").unwrap().len(), 1);

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn delete_conversation_cascades_file_changes() {
        let db = temp_db("cascade");
        let lim = FileChangesConfig::default();
        add_file_change(&db, "conv-del", "a.txt", "1", "2", &lim).unwrap();
        add_file_change(&db, "conv-del", "b.txt", "3", "4", &lim).unwrap();
        add_file_change(&db, "conv-keep", "c.txt", "5", "6", &lim).unwrap();

        let conn = open_connection(&db).unwrap();
        delete_conversation(&conn, "conv-del").unwrap();

        assert!(get_file_changes(&db, "conv-del").unwrap().is_empty());
        // 其他對話的快照不受級聯影響
        assert_eq!(get_file_changes(&db, "conv-keep").unwrap().len(), 1);

        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn delete_conversation_cascades_conversation_audits() {
        let db = temp_db("cascade_audits");
        let conn = open_connection(&db).unwrap();
        let audits = vec![
            ("G1".to_string(), "[PASS]".to_string(), "ok".to_string()),
            (
                "G2".to_string(),
                "[REJECT: db.rs:1 | 範例".to_string(),
                "ng".to_string(),
            ),
        ];
        replace_conversation_audits(&conn, "conv-del", &audits).unwrap();
        replace_conversation_audits(&conn, "conv-keep", &audits).unwrap();

        delete_conversation(&conn, "conv-del").unwrap();

        assert!(get_conversation_audits(&conn, "conv-del").unwrap().is_empty());
        // 其他對話的審查列不受級聯影響
        assert_eq!(get_conversation_audits(&conn, "conv-keep").unwrap().len(), 2);

        let _ = std::fs::remove_file(&db);
    }
}
