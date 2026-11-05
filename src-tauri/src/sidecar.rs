//! Node sidecar 进程管理与桥接。
//!
//! 职责分层（见 docs/ipc-contract.md §3）：
//!   Rust   —— 窗口、菜单、对话框、路径、打包
//!   Node   —— 业务逻辑（files / git / python / state），以子进程运行
//!   Python —— psychopy 环境，由 Node 调度
//!
//! 所以这里的核心是双向转发：
//!   前端 invoke("node_call", {channel, params})
//!     -> 写到 sidecar stdin（JSON-RPC）
//!     -> sidecar 处理后从 stdout 回 {"id", "result"|"error"}
//!     -> 这里按 id 匹配，resolve 给前端
//!
//! 反向通道（sidecar 需要宿主能力，如 OAuth 窗口）：
//!   sidecar 往 stdout 写 {"id":"host_N","method":"psychojs_run",...}
//!     -> 这里识别出没有 "sender" 且 id 以 host_ 开头的帧
//!     -> 在 Rust 里执行对应能力
//!     -> 把结果写回 stdin
//!
//! 事件（Node -> 前端）：
//!   sidecar stdout {"method":"event","params":{"tag","data"}}
//!     -> emit 到对应窗口的 `tag`（保留层级广播语义）

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// sidecar 进程 + 通信状态
/// stdin 和 child 都用锁包起来：write 可能发生在任意线程，drop 时也要跨线程。
pub struct Sidecar {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    /// 等待响应的前端请求：id -> oneshot sender
    pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, String>>>>>,
    next_id: Arc<Mutex<u64>>,
}

static SIDECAR: Mutex<Option<Arc<Sidecar>>> = Mutex::new(None);

/// 启动 sidecar。在 setup() 里调一次。
///
/// 用 `node <script>` 而不是打包的可执行文件：sidecar 是纯 JS + node_modules，
/// 打包成单文件反而是给 esbuild 找麻烦。生产包里 script 路径来自 resources。
pub fn start<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let script = sidecar_script(app)?;
    let node = std::env::var("GNOSTIS_NODE").unwrap_or_else(|_| "node".to_string());

    // sidecar 的 stderr 落到文件：GUI 应用下 inherit 到不了终端，
    // 出问题时这是唯一能看到 Node 侧报错的地方。
    let log_path = std::env::temp_dir().join("gnosis-sidecar.log");
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("cannot open sidecar log {}: {e}", log_path.display()))?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;

    eprintln!("[sidecar] spawning {node} {script} (log: {})", log_path.display());

    let mut child = Command::new(&node)
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log_err))
        .env("GNOSTIS_HOST", "1")
        .env("GNOSTIS_APP_NAME", "psychopy4")
        .spawn()
        .map_err(|e| format!("failed to spawn sidecar ({node} {script}): {e}"))?;

    let stdin = child.stdin.take().ok_or("sidecar stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("sidecar stdout unavailable")?;

    let pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, String>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let sidecar = Arc::new(Sidecar {
        child: Mutex::new(Some(child)),
        stdin: Mutex::new(Some(stdin)),
        pending: pending.clone(),
        next_id: Arc::new(Mutex::new(1)),
    });

    // 读线程
    let app_for_reader = app.clone();
    std::thread::spawn(move || read_loop(app_for_reader, stdout, pending));

    *SIDECAR.lock().unwrap() = Some(sidecar);
    Ok(())
}

/// sidecar 脚本路径。
///
/// 开发态在仓库里；生产态由打包放进 resources（见 tauri.conf.json 的 resources）。
fn sidecar_script<R: Runtime>(app: &AppHandle<R>) -> Result<String, String> {
    if let Ok(p) = std::env::var("GNOSTIS_SIDECAR") {
        return Ok(p);
    }
    // 生产：resources/sidecar/sidecar.mjs
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join("sidecar").join("sidecar.mjs");
        if p.exists() {
            return Ok(p.to_string_lossy().to_string());
        }
    }
    // 开发：仓库内的 electron/src/sidecar.mjs
    //
    // Rust 与 Node 现在同一棵树（M9 把 src-tauri 归位到 GnosisStudio/），
    // 所以从 src-tauri 往上一层就是仓库根。打包后走上面的 resource_dir，
    // 不依赖这个相对路径。
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("electron")
        .join("src")
        .join("sidecar.mjs");
    if dev.exists() {
        return Ok(dev.to_string_lossy().to_string());
    }
    Err(format!(
        "sidecar script not found (looked in resource_dir/sidecar and {})",
        dev.display()
    ))
}

fn read_loop<R: Runtime>(
    app: AppHandle<R>,
    stdout: std::process::ChildStdout,
    pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, String>>>>>,
) {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        // 1) 反向请求：sidecar 要宿主能力（id 形如 "host_N"）
        if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
            if id.starts_with("host_") {
                handle_host_request(&app, id, &msg);
                continue;
            }
        }

        // 2) 事件：Node -> 前端
        if msg.get("method").and_then(|v| v.as_str()) == Some("event") {
            if let Some(params) = msg.get("params") {
                let tag = params.get("tag").and_then(|v| v.as_str()).unwrap_or("");
                let data = params.get("data").cloned().unwrap_or(Value::Null);
                // 层级广播，与 transport 的 sendEvent 语义一致：a:b:c -> a:b:c, a:b, a
                for t in tag_chain(tag) {
                    let _ = app.emit(&t, data.clone());
                }
            }
            continue;
        }

        // 3) 普通响应：交给等待中的前端请求
        if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
            let sender = pending.lock().unwrap().remove(&id);
            if let Some(tx) = sender {
                let out = if let Some(err) = msg.get("error") {
                    Err(err.get("message").and_then(|v| v.as_str()).unwrap_or("sidecar error").to_string())
                } else {
                    Ok(msg.get("result").cloned().unwrap_or(Value::Null))
                };
                let _ = tx.send(out);
            }
        }
    }
}

/// `a:b:c` -> ["a:b:c", "a:b", "a"]
fn tag_chain(tag: &str) -> Vec<String> {
    if tag.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = tag.split(':').collect();
    let mut out = Vec::with_capacity(parts.len());
    while !parts.is_empty() {
        out.push(parts.join(":"));
        parts.pop();
    }
    out
}

/// sidecar 发来的宿主能力请求
fn handle_host_request<R: Runtime>(app: &AppHandle<R>, id: &str, msg: &Value) {
    let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = msg.get("params").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let app = app.clone();
    let id = id.to_string();
    let method_owned = method.to_string();

    // 有些能力需要在主线程/异步上下文里跑，统一丢给 tokio
    tauri::async_runtime::spawn(async move {
        let result: Result<Value, String> = match method_owned.as_str() {
            "psychojs_run" => {
                let cwd = params.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let p = params.get(1).cloned();
                crate::system::psychojs_run(app, cwd, p).await.map(Value::String)
            }
            "psychojs_stop" => {
                let addr = params.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                crate::system::psychojs_stop(addr).await.map(|_| Value::Null)
            }
            "auth_window" => {
                let url = params.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                let host = params.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                crate::system::auth_window(app, url, host).await.map(Value::String)
            }
            other => Err(format!("unknownown host method: {other}")),
        };

        reply_to_sidecar(&id, result);
    });
}

fn reply_to_sidecar(id: &str, result: Result<Value, String>) {
    let frame = match result {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": e } }),
    };
    if let Some(sc) = SIDECAR.lock().unwrap().as_ref() {
        if let Some(stdin) = sc.stdin.lock().unwrap().as_ref() {
            let mut w = stdin;
            let _ = writeln!(w, "{frame}");
        }
    }
}

/// 前端 -> sidecar 的通用调用入口。
///
/// 参数顺序与 preload 完全一致：`invoke(channel, ...args)`。
#[tauri::command]
pub async fn node_call(channel: String, params: Value) -> Result<Value, String> {
    eprintln!("[node_call] {channel}");
    let sc = {
        let guard = SIDECAR.lock().unwrap();
        guard.as_ref().cloned().ok_or("sidecar is not running")?
    };

    let args = match params {
        Value::Array(a) => a,
        Value::Null => Vec::new(),
        other => vec![other],
    };

    let id = {
        let mut n = sc.next_id.lock().unwrap();
        let id = *n;
        *n += 1;
        id
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    sc.pending.lock().unwrap().insert(id, tx);

    let frame = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": channel,
        "params": args,
    });
    {
        let stdin = sc.stdin.lock().unwrap();
        let stdin = stdin.as_ref().ok_or("sidecar stdin closed")?;
        let mut writer = stdin;
        writeln!(writer, "{frame}").map_err(|e| format!("write to sidecar failed: {e}"))?;
    }

    let out = match rx.await {
        Ok(res) => res,
        Err(_) => Err("sidecar closed before responding".to_string()),
    };
    eprintln!("[node_call] {channel} -> {}", match &out { Ok(v) => format!("ok {}", v.to_string().chars().take(60).collect::<String>()), Err(e) => format!("err {e}") });
    out
}

/// sidecar 是否已就绪（前端可据此显示"正在启动"）
#[tauri::command]
pub fn node_ready() -> bool {
    SIDECAR.lock().unwrap().is_some()
}

/// 关掉 sidecar。退出前调用。
///
/// 顺序很重要：先关 stdin，sidecar 的 `process.stdin.on("end")` 会触发
/// shutdown() -> saveState() -> process.exit(0)。窗口布局就是靠这条路径落盘的。
/// 只 kill 不关 stdin 会丢掉状态。
pub fn stop() {
    let sc = SIDECAR.lock().unwrap().take();
    let Some(sc) = sc else { return };

    // 关 stdin：触发 sidecar 的 'end' -> saveState() -> exit。
    // 只 kill 不关 stdin 会丢掉窗口布局（sidecar 的落盘走这条路径）。
    sc.stdin.lock().unwrap().take();

    // 等它自己退，最多 2 秒；超时再 kill，避免卡住整个应用退出
    let taken = sc.child.lock().unwrap().take();
    if let Some(mut child) = taken {
        for _ in 0..20 {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(_) => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}


#[cfg(test)]
mod tag_chain_tests {
    use super::*;

    #[test]
    fn chain_expands_hierarchically() {
        // 与 transport 的 sendEvent 语义必须一致，否则前端订阅 a:b 收不到 a:b:c
        assert_eq!(tag_chain("a:b:c"), vec!["a:b:c", "a:b", "a"]);
        assert_eq!(tag_chain("a"), vec!["a"]);
        assert_eq!(tag_chain(""), Vec::<String>::new());
    }
}
