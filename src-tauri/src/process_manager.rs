//! Python 进程生命周期管理
//! 负责所有 Python 交互式 Shell 和 脚本进程的注册、控制与清理。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use std::process::Stdio;

/// Python 进程的统一包装
pub struct ManagedProcess {
    pub id: String,
    pub child: Mutex<Option<tokio::process::Child>>,
    pub venv_version: String,
}

/// 全局进程注册表
pub struct ProcessRegistry {
    pub processes: Mutex<HashMap<String, Arc<ManagedProcess>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, id: String, process: tokio::process::Child, venv: String) {
        let managed = Arc::new(ManagedProcess {
            id: id.clone(),
            child: Mutex::new(Some(process)),
            venv_version: venv,
        });
        self.processes.lock().unwrap().insert(id, managed);
    }

    pub fn unregister(&self, id: &str) {
        self.processes.lock().unwrap().remove(id);
    }

    pub fn get(&self, id: &str) -> Option<Arc<ManagedProcess>> {
        self.processes.lock().unwrap().get(id).cloned()
    }

    pub fn kill_all(&self) {
        let mut procs = self.processes.lock().unwrap();
        for (_, proc) in procs.drain() {
            if let Some(child) = proc.child.lock().unwrap().take() {
                let _ = child.kill();
            }
        }
    }

    /// 启动一个交互式 Python Shell
    pub async fn spawn_shell(
        &self, 
        app: AppHandle, 
        executable: String, 
        venv: String
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        
        let mut child = Command::new(&executable)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn python shell: {e}"))?;

        let stdout = child.stdout.take().ok_or("stdout not available")?;
        let stdin = child.stdin.take().ok_or("stdin not available")?;

        self.register(id.clone(), child, venv);

        let app_clone = app.clone();
        let id_clone = id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = app_clone.emit("python.shell.stdout", json!({ "id": id_clone, "data": line }));
            }
        });

        let mut stdin = stdin;
        let _ = stdin.write_all(b"import sys\n").await;

        Ok(id)
    }

    /// 启动一个 Python 脚本
    pub async fn spawn_script(
        &self,
        app: AppHandle,
        executable: String,
        file: String,
        args: Vec<String>,
        venv: String,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let folder = std::path::Path::new(&file).parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
        
        let mut child = Command::new(&executable)
            .arg("-u")
            .arg(&file)
            .args(&args)
            .current_dir(folder)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn python script: {e}"))?;

        let stdout = child.stdout.take().ok_or("stdout not available")?;
        let stderr = child.stderr.take().ok_//error not available")?;

        self.register(id.clone(), child, venv);

        let app_stdout = app.clone();
        let id_stdout = id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = app_stdout.emit("python.shell.stdout", json!({ "id": id_stdout, "data": line }));
            }
        });

        let app_stderr = app.clone();
        let id_stderr = id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = app_stderr.emit("python.shell.stderr", json!({ "id": id_stderr, "data": line }));
            }
        });

        Ok(id)
    }
}
