//! SQLite-based state management for workflow executions

use crate::error::Result;
use crate::models::{TaskExecution, TaskStatus, WorkflowExecution};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// State manager for workflow and task execution tracking
#[derive(Clone)]
pub struct StateManager {
    conn: Arc<Mutex<Connection>>,
}

impl StateManager {
    /// Create a new state manager with SQLite database
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Configure SQLite for edge devices
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -2000;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 0;
            PRAGMA foreign_keys = ON;
            ",
        )?;

        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        manager.init_schema()?;
        Ok(manager)
    }

    /// Create in-memory database (for testing)
    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        manager.init_schema()?;
        Ok(manager)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS workflows (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS executions (
                id INTEGER PRIMARY KEY,
                workflow_id INTEGER NOT NULL,
                started_at TIMESTAMP NOT NULL,
                completed_at TIMESTAMP,
                status TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflows(id)
            );

            CREATE TABLE IF NOT EXISTS task_executions (
                id INTEGER PRIMARY KEY,
                execution_id INTEGER NOT NULL,
                task_name TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMP NOT NULL,
                completed_at TIMESTAMP,
                exit_code INTEGER,
                stdout TEXT,
                stderr TEXT,
                attempt INTEGER DEFAULT 1,
                retry_count INTEGER DEFAULT 0,
                next_retry_at TIMESTAMP,
                FOREIGN KEY (execution_id) REFERENCES executions(id)
            );

            CREATE TABLE IF NOT EXISTS retention_policy (
                workflow_name TEXT PRIMARY KEY,
                max_executions INTEGER DEFAULT 100,
                max_age_days INTEGER DEFAULT 30
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_workflows_name ON workflows(name);
            CREATE INDEX IF NOT EXISTS idx_executions_workflow_started ON executions(workflow_id, started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_task_executions_status ON task_executions(status);
            CREATE INDEX IF NOT EXISTS idx_task_executions_execution ON task_executions(execution_id);
            CREATE INDEX IF NOT EXISTS idx_task_executions_started ON task_executions(started_at);
            ",
        )?;

        Ok(())
    }

    /// Get or create workflow ID
    pub fn get_or_create_workflow(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        // Try to get existing workflow
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM workflows WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            return Ok(id);
        }

        // Create new workflow
        conn.execute("INSERT INTO workflows (name) VALUES (?1)", params![name])?;

        Ok(conn.last_insert_rowid())
    }

    /// Start a new workflow execution
    pub fn start_execution(&self, workflow_id: i64) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO executions (workflow_id, started_at, status) VALUES (?1, ?2, ?3)",
            params![workflow_id, Utc::now(), TaskStatus::Running.to_string()],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Update execution status
    pub fn update_execution_status(&self, execution_id: i64, status: TaskStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE executions SET status = ?1, completed_at = ?2 WHERE id = ?3",
            params![
                status.to_string(),
                if matches!(
                    status,
                    TaskStatus::Success | TaskStatus::Failed | TaskStatus::Timeout
                ) {
                    Some(Utc::now())
                } else {
                    None
                },
                execution_id
            ],
        )?;

        Ok(())
    }

    /// Start a task execution
    pub fn start_task(&self, execution_id: i64, task_name: &str, attempt: i32) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO task_executions (execution_id, task_name, status, started_at, attempt) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                execution_id,
                task_name,
                TaskStatus::Running.to_string(),
                Utc::now(),
                attempt
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Update task execution status
    pub fn update_task_status(
        &self,
        task_execution_id: i64,
        status: TaskStatus,
        exit_code: Option<i32>,
        stdout: Option<&str>,
        stderr: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE task_executions SET status = ?1, completed_at = ?2, exit_code = ?3, stdout = ?4, stderr = ?5 WHERE id = ?6",
            params![
                status.to_string(),
                if matches!(status, TaskStatus::Success | TaskStatus::Failed | TaskStatus::Timeout) {
                    Some(Utc::now())
                } else {
                    None
                },
                exit_code,
                stdout,
                stderr,
                task_execution_id
            ],
        )?;

        Ok(())
    }

    /// Set task retry information
    pub fn set_task_retry(
        &self,
        task_execution_id: i64,
        retry_count: i32,
        next_retry_at: DateTime<Utc>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE task_executions SET status = ?1, retry_count = ?2, next_retry_at = ?3 WHERE id = ?4",
            params![
                TaskStatus::Retrying.to_string(),
                retry_count,
                next_retry_at,
                task_execution_id
            ],
        )?;

        Ok(())
    }

    /// Get execution by ID
    pub fn get_execution(&self, execution_id: i64) -> Result<Option<WorkflowExecution>> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                "SELECT id, workflow_id, started_at, completed_at, status FROM executions WHERE id = ?1",
                params![execution_id],
                |row| {
                    Ok(WorkflowExecution {
                        id: row.get(0)?,
                        workflow_id: row.get(1)?,
                        started_at: row.get(2)?,
                        completed_at: row.get(3)?,
                        status: parse_task_status(&row.get::<_, String>(4)?),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Get task executions for a workflow execution
    pub fn get_task_executions(&self, execution_id: i64) -> Result<Vec<TaskExecution>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, execution_id, task_name, status, started_at, completed_at, exit_code, stdout, stderr, attempt, retry_count, next_retry_at
             FROM task_executions WHERE execution_id = ?1 ORDER BY started_at",
        )?;

        let rows = stmt.query_map(params![execution_id], |row| {
            Ok(TaskExecution {
                id: row.get(0)?,
                execution_id: row.get(1)?,
                task_name: row.get(2)?,
                status: parse_task_status(&row.get::<_, String>(3)?),
                started_at: row.get(4)?,
                completed_at: row.get(5)?,
                exit_code: row.get(6)?,
                stdout: row.get(7)?,
                stderr: row.get(8)?,
                attempt: row.get(9)?,
                retry_count: row.get(10)?,
                next_retry_at: row.get(11)?,
            })
        })?;

        let mut executions = Vec::new();
        for row in rows {
            executions.push(row?);
        }

        Ok(executions)
    }

    /// Recover from crash - find and mark incomplete executions as failed
    pub fn recover_from_crash(&self) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();

        // Find executions that were running when process crashed
        let mut stmt = conn.prepare("SELECT id FROM executions WHERE status = ?1")?;

        let crashed_ids: Vec<i64> = stmt
            .query_map(params![TaskStatus::Running.to_string()], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Mark them as failed
        for id in &crashed_ids {
            conn.execute(
                "UPDATE executions SET status = ?1, completed_at = ?2 WHERE id = ?3",
                params![TaskStatus::Failed.to_string(), Utc::now(), id],
            )?;
        }

        Ok(crashed_ids)
    }

    /// Get execution history for a workflow
    pub fn get_execution_history(
        &self,
        workflow_name: &str,
        limit: usize,
    ) -> Result<Vec<WorkflowExecution>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT e.id, e.workflow_id, e.started_at, e.completed_at, e.status
             FROM executions e
             JOIN workflows w ON e.workflow_id = w.id
             WHERE w.name = ?1
             ORDER BY e.started_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![workflow_name, limit], |row| {
            Ok(WorkflowExecution {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                started_at: row.get(2)?,
                completed_at: row.get(3)?,
                status: parse_task_status(&row.get::<_, String>(4)?),
            })
        })?;

        let mut executions = Vec::new();
        for row in rows {
            executions.push(row?);
        }

        Ok(executions)
    }
}

fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "pending" => TaskStatus::Pending,
        "running" => TaskStatus::Running,
        "success" => TaskStatus::Success,
        "failed" => TaskStatus::Failed,
        "retrying" => TaskStatus::Retrying,
        "timeout" => TaskStatus::Timeout,
        _ => TaskStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_state_manager() {
        let manager = StateManager::in_memory().unwrap();
        assert!(manager.conn.lock().is_ok());
    }

    #[test]
    fn test_workflow_operations() {
        let manager = StateManager::in_memory().unwrap();

        // Create workflow
        let workflow_id = manager.get_or_create_workflow("test-workflow").unwrap();
        assert!(workflow_id > 0);

        // Get existing workflow
        let workflow_id2 = manager.get_or_create_workflow("test-workflow").unwrap();
        assert_eq!(workflow_id, workflow_id2);
    }

    #[test]
    fn test_execution_lifecycle() {
        let manager = StateManager::in_memory().unwrap();

        let workflow_id = manager.get_or_create_workflow("test").unwrap();
        let execution_id = manager.start_execution(workflow_id).unwrap();

        // Check execution status
        let execution = manager.get_execution(execution_id).unwrap().unwrap();
        assert_eq!(execution.status, TaskStatus::Running);
        assert!(execution.completed_at.is_none());

        // Update to success
        manager
            .update_execution_status(execution_id, TaskStatus::Success)
            .unwrap();

        let execution = manager.get_execution(execution_id).unwrap().unwrap();
        assert_eq!(execution.status, TaskStatus::Success);
        assert!(execution.completed_at.is_some());
    }

    #[test]
    fn test_task_execution() {
        let manager = StateManager::in_memory().unwrap();

        let workflow_id = manager.get_or_create_workflow("test").unwrap();
        let execution_id = manager.start_execution(workflow_id).unwrap();

        // Start task
        let task_id = manager.start_task(execution_id, "task1", 1).unwrap();

        // Update task status
        manager
            .update_task_status(
                task_id,
                TaskStatus::Success,
                Some(0),
                Some("output"),
                Some("error"),
            )
            .unwrap();

        // Get task executions
        let tasks = manager.get_task_executions(execution_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_name, "task1");
        assert_eq!(tasks[0].status, TaskStatus::Success);
        assert_eq!(tasks[0].exit_code, Some(0));
        assert_eq!(tasks[0].stdout, Some("output".to_string()));
    }

    #[test]
    fn test_task_retry() {
        let manager = StateManager::in_memory().unwrap();

        let workflow_id = manager.get_or_create_workflow("test").unwrap();
        let execution_id = manager.start_execution(workflow_id).unwrap();
        let task_id = manager.start_task(execution_id, "task1", 1).unwrap();

        // Set retry
        let next_retry = Utc::now() + chrono::Duration::seconds(10);
        manager.set_task_retry(task_id, 1, next_retry).unwrap();

        let tasks = manager.get_task_executions(execution_id).unwrap();
        assert_eq!(tasks[0].status, TaskStatus::Retrying);
        assert_eq!(tasks[0].retry_count, 1);
        assert!(tasks[0].next_retry_at.is_some());
    }

    #[test]
    fn test_crash_recovery() {
        let manager = StateManager::in_memory().unwrap();

        let workflow_id = manager.get_or_create_workflow("test").unwrap();
        let exec1 = manager.start_execution(workflow_id).unwrap();
        let exec2 = manager.start_execution(workflow_id).unwrap();

        // Complete one execution
        manager
            .update_execution_status(exec1, TaskStatus::Success)
            .unwrap();

        // Simulate crash (exec2 still running)
        let crashed = manager.recover_from_crash().unwrap();

        assert_eq!(crashed.len(), 1);
        assert_eq!(crashed[0], exec2);

        // Verify exec2 marked as failed
        let execution = manager.get_execution(exec2).unwrap().unwrap();
        assert_eq!(execution.status, TaskStatus::Failed);
    }

    #[test]
    fn test_execution_history() {
        let manager = StateManager::in_memory().unwrap();

        let workflow_id = manager.get_or_create_workflow("test").unwrap();

        // Create multiple executions
        for _ in 0..5 {
            let exec_id = manager.start_execution(workflow_id).unwrap();
            manager
                .update_execution_status(exec_id, TaskStatus::Success)
                .unwrap();
        }

        let history = manager.get_execution_history("test", 3).unwrap();
        assert_eq!(history.len(), 3);
    }
}
