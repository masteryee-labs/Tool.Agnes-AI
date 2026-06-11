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
            logged_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_index USING fts5(
            file_path UNINDEXED,
            tag,
            content
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
        ",
    )?;

    // 既有資料庫遷移：舊版 conversations 無 project_id 欄。
    // ALTER 對已有該欄的表會失敗——該錯誤即「無需遷移」，安全忽略。
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN project_id TEXT", []);

    Ok(())
}

/// 全域模式 Session 的 project_id 哨兵值（不對應任何 projects 列）。
pub const GLOBAL_PROJECT_ID: &str = "global";

/// Open a new Connection at the given path. Returns an error if the DB cannot
/// be created / opened. Calls `init_db` automatically.
pub fn open_connection(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
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

pub fn delete_conversation(conn: &Connection, conversation_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM conversation_messages WHERE conversation_id = ?1",
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

pub fn add_token_log(
    conn: &Connection,
    task_id: Option<&str>,
    model_name: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost_usd: f64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO token_ledger (task_id, model_name, prompt_tokens, completion_tokens, total_tokens, cost_usd) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![task_id, model_name, prompt_tokens, completion_tokens, total_tokens, cost_usd],
    )?;
    Ok(())
}
