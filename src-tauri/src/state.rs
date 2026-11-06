//! 应用状态管理。
//! 负责处理窗口布局 (frames) 和 配置文件 (profiles) 的持久化。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FrameState {
    pub pos: Option<[f64; 2]>,
    pub size: Option<[f64; 2]>,
    pub maximized: bool,
    pub files: Vec<String>,
    pub view: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppState {
    pub frames: HashMap<String, FrameState>,
    pub profiles: HashMap<String, serde_json::Value>,
}

pub struct StateManager {
    pub current: Mutex<AppState>,
    path: PathBuf,
}

impl StateManager {
    pub fn new(app: &AppHandle) -> Self {
        let mut path = app.path().app_data_dir().expect("failed to get app data dir");
        path.push("psychopy4");
        fs::create_dir_all(&path).expect("failed to create app data dir");
        path.push("last_app_state.json");

        let current = if path.exists() {
            let content = fs::read_to_string(&path).expect("failed to read state file");
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            AppState::default()
        };

        Self {
            current: Mutex::new(current),
            path,
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let state = self.current.lock().unwrap();
        let content = serde_json::to_string_pretty(&*state).map_err(|e| e.to_string())?;
        fs::write(&self.path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_frame(&self, id: String, details: serde_json::Value) -> Result<(), String> {
        let mut state = self.current.lock().unwrap();
        let frame = state.frames.entry(id.clone()).or_insert_with(|| FrameState {
            view: id, // 默认 view 为 id (与 Node 版逻辑一致)
            ..Default::default()
        });

        // 简单实现：将 JSON 字段合并到 FrameState
        // 在实际生产中建议定义具体更新结构体
        if let Some(pos) = details.get("pos").and_then(|v| v.as_array()) {
            if pos.len() == 2 {
                frame.pos = Some([pos[0].as_f64().unwrap_or(0.0), pos[1].as_f64().unwrap_or(0.0)]);
            }
        }
        if let Some(size) = details.get("size").and_then(|v| v.as_array()) {
            if size.len() == 2 {
                frame.size = Some([size[0].as_f64().unwrap_or(0.0), size[1].as_f64().unwrap_or(0.0)]);
            }
        }
        if let Some(maximized) = details.get("maximized").and_then(|v| v.as_bool()) {
            frame.maximized = maximized;
        }
        if let Some(files) = details.get("files").and_then(|v| v.as_array()) {
            frame.files = files.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
        }
        if let Some(view) = details.get("view").and_then(|v| v.as_str()) {
            frame.view = view.to_string();
        }

        Ok(())
    }

    pub fn set_profiles(&self, key: String, value: serde_json::Value) -> Result<(), String> {
        {
            let mut state = self.current.lock().unwrap();
            state.profiles.insert(key, value);
        }
        self.save()
    }

    pub fn get_profiles(&self) -> serde_json::Value {
        let state = self.current.lock().unwrap();
        serde_json::to_value(&state.profiles).unwrap_or(serde_json::Value::Null)
    }
}
