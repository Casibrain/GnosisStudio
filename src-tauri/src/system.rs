//! 系统能力命令 —— M3
//!
//! 这 22 个 IPC 通道必须有宿主才能做，Electron 版里由 index.cjs 提供。
//! sidecar 是 headless 的，所以改由 Rust 直接实现（见 docs/ipc-contract.md §3.6）。
//!
//!   electron.paths.*      × 7  → 路径解析
//!   electron.windows.*    × 8  → 窗口操作
//!   electron.clipboard.*  × 2  → 剪贴板
//!   electron.system.*     × 2  → macOS 键盘权限
//!   electron.platform / version / quit × 3

use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// 与 Electron `app.getPath("appData")` 对齐的平台 base 目录。
///
/// 这里是**平台 base**，不含应用名——Electron 版代码自己拼 "psychopy4"。
/// 拼错会让用户已有的 venv / 设置全部"消失"。
pub fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(p) = std::env::var("APPDATA") {
            return PathBuf::from(p);
        }
        if let Some(home) = home_dir() {
            return home.join("AppData").join("Roaming");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            return home.join("Library").join("Application Support");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(p);
        }
        if let Some(home) = home_dir() {
            return home.join(".config");
        }
    }
    home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Electron 版固定用 "psychopy4"（index.cjs:258）
const APP_FOLDER: &str = "psychopy4";

fn psychopy_dir() -> PathBuf {
    app_data_dir().join(APP_FOLDER)
}

fn s(p: PathBuf) -> String {
    p.to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// electron.paths.*
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn paths_documents() -> String {
    home_dir().map(s).unwrap_or_default()
}

#[tauri::command]
pub fn paths_user() -> String {
    s(psychopy_dir())
}

#[tauri::command]
pub fn paths_devices() -> String {
    s(psychopy_dir().join("devices.json"))
}

#[tauri::command]
pub fn paths_prefs() -> String {
    s(psychopy_dir().join("preferences.json"))
}

#[tauri::command]
pub fn paths_pavlovia() -> String {
    s(psychopy_dir().join("pavlovia"))
}

#[tauri::command]
pub fn paths_pavlovia_users() -> String {
    s(psychopy_dir().join("pavlovia").join("users.json"))
}

#[tauri::command]
pub fn paths_pavlovia_projects() -> String {
    s(psychopy_dir().join("pavlovia").join("projects.json"))
}

// ---------------------------------------------------------------------------
// electron.platform / version / quit
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn platform() -> &'static str {
    // Electron 的 process.platform 用小写："darwin" / "win32" / "linux"
    // std::env::consts::OS 是 "macos"，必须映射，否则前端判断会失效
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

#[tauri::command]
pub fn app_version<R: Runtime>(app: AppHandle<R>) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn quit<R: Runtime>(app: AppHandle<R>) {
    app.exit(0);
}

// ---------------------------------------------------------------------------
// electron.windows.*
// ---------------------------------------------------------------------------

fn find_window<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<tauri::WebviewWindow<R>, String> {
    app.get_webview_window(id)
        .ok_or_else(|| format!("no window: {id}"))
}

/// 新窗口。Electron 版 `newWindow(target)` 用 target 决定加载哪个路由
/// （"builder" / "coder" / "runner"，null 则加载首页）。
///
/// 窗口 label 用 `target` 本身，与前端 `electron.windows.get(target)` 的
/// "url 包含 target" 语义天然对齐。
#[tauri::command]
pub async fn windows_new<R: Runtime>(app: AppHandle<R>, target: Option<String>) -> Result<(), String> {
    let target = target.filter(|t| !t.is_empty());
    let label = window_label(target.as_deref());

    // 已存在则只聚焦，避免重复开窗（Electron 版会开新的，但重复 label 在 Tauri 是硬错误）
    if let Some(w) = app.get_webview_window(&label) {
        return w.set_focus().map_err(|e| e.to_string());
    }

    let url = window_url(target.as_deref());

    tauri::WebviewWindowBuilder::new(&app, &label, url)
        .title("Gnosis Studio")
        .inner_size(1600.0, 900.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 由 target 生成合法窗口 label。
///
/// Tauri 的 label 只允许 alphanumeric、`-`、`/`、`:`、`_`。
/// target 形如 `builder?fileOpen=/a/b.psyexp`，其中的 `?` `=` 都不能出现在 label 里，
/// 所以只取 query 之前的部分并把非法字符换成 `_`。
///
/// 注意 label 同时是 `windows_get` 的去重依据：同一个 target 前缀复用同一窗口，
/// 与 Electron 版"url 包含 target"的语义保持一致。
fn window_label(target: Option<&str>) -> String {
    let Some(t) = target else { return "main".to_string() };
    let path = t.split('?').next().unwrap_or(t);
    let cleaned: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | ':' | '_') { c } else { '_' })
        .collect();
    if cleaned.is_empty() { "main".to_string() } else { cleaned }
}

/// 由 target 生成要加载的 URL。
///
/// 前半段是预渲染出来的目录（`builder` -> `builder/index.html`），
/// query 必须留在 `/index.html` **之后**，否则会拼成
/// `builder?fileOpen=x/index.html` 这种无效路径。
fn window_url(target: Option<&str>) -> tauri::WebviewUrl {
    let Some(t) = target else {
        return tauri::WebviewUrl::App("index.html".into());
    };
    let (path, query) = match t.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (t, None),
    };
    let path = path.trim_matches('/');
    let mut url = if path.is_empty() {
        "index.html".to_string()
    } else {
        format!("{path}/index.html")
    };
    if let Some(q) = query {
        url.push('?');
        url.push_str(q);
    }
    tauri::WebviewUrl::App(url.into())
}

/// Electron 版语义：返回 url 里包含 target 的窗口 id 列表
#[tauri::command]
pub async fn windows_get<R: Runtime>(app: AppHandle<R>, target: String) -> Vec<String> {
    app.webview_windows()
        .into_iter()
        .filter(|(_, w)| {
            w.url()
                .map(|u| u.as_str().contains(&target))
                .unwrap_or(false)
        })
        .map(|(label, _)| label)
        .collect()
}

#[tauri::command]
pub async fn windows_send<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    tag: String,
    data: serde_json::Value,
) -> Result<(), String> {
    find_window(&app, &id)?
        .emit(&tag, data)
        .map_err(|e| e.to_string())
}

// Electron 用 `windows[id || evt.sender.id]` —— id 为空时回退到来源窗口
fn resolve_key(id: Option<String>, sender: Option<String>) -> Result<String, String> {
    id.filter(|s| !s.is_empty())
        .or(sender)
        .ok_or_else(|| "no window id".to_string())
}

#[tauri::command]
pub async fn windows_focus<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    sender: Option<String>,
) -> Result<(), String> {
    find_window(&app, &resolve_key(id, sender)?)?
        .set_focus()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn windows_close<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    sender: Option<String>,
) -> Result<(), String> {
    find_window(&app, &resolve_key(id, sender)?)?
        .close()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn windows_devtools<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    sender: Option<String>,
) -> Result<(), String> {
    let win = find_window(&app, &resolve_key(id, sender)?)?;
    #[cfg(debug_assertions)]
    win.open_devtools();
    Ok(())
}

/// Electron 的 removeMenu()
#[tauri::command]
pub async fn windows_hide_menu<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    sender: Option<String>,
) -> Result<(), String> {
    find_window(&app, &resolve_key(id, sender)?)?
        .hide_menu()
        .map_err(|e| e.to_string())
}

/// Electron 的 setMenu(win, template)
///
/// 前端（`src/lib/ui/menu/frameMenu/Item.svelte:68`）给每个菜单项的 `click`
/// 赋一个字符串 id，然后 `listen("menu:<id>")`。原实现把这个 id 换成 shell 回调，
/// 触发时 `webContents.send("menu:<id>", true)` 回给前端。
///
/// Tauri 下需要：
///   1. 把 Electron 模板翻译成 `tauri::menu::Menu`（字段名基本一致：label/enabled/submenu）
///   2. 每个菜单项的 `click` 挂一个闭包，emit 回 `menu:<id>`
///   3. macOS 设成应用菜单，其他平台设成窗口菜单
///
/// 注意：emit 的 tag 是 `menu:<id>`，前端用 `listen("menu:<id>")` 收
/// （preload 对 windows.listen 不加前缀，见 ipc-contract.md §3.3）。
#[tauri::command]
pub async fn windows_set_menu<R: Runtime>(
    app: AppHandle<R>,
    template: serde_json::Value,
    id: Option<String>,
    sender: Option<String>,
) -> Result<(), String> {
    let key = resolve_key(id, sender)?;
    let win = find_window(&app, &key)?;
    let menu = build_menu(&app, &template)?;

    // macOS 的菜单是应用级的，不能挂到单个窗口上（tauri 文档明确写 Unsupported）
    if cfg!(target_os = "macos") {
        app.set_menu(menu).map_err(|e| e.to_string())?;
    } else {
        win.set_menu(menu).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 把 Electron 的菜单模板翻译成 Tauri 菜单。
///
/// 只处理前端实际用到的字段（见 `Item.svelte` 的 getProfile）：label、enabled、
/// click、role、submenu。其余字段（accelerator 等）忽略而不是报错——
/// 菜单渲染失败不该让整个窗口卡住。
fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    template: &serde_json::Value,
) -> Result<tauri::menu::Menu<R>, String> {
    use tauri::menu::{Menu, PredefinedMenuItem, Submenu};

    let entries = template.as_array().ok_or("menu template must be an array")?;
    let menu = Menu::new(app).map_err(|e| e.to_string())?;

    for entry in entries {
        let label = entry.get("label").and_then(|v| v.as_str()).unwrap_or("");

        // 有 submenu 时递归成子菜单
        if let Some(sub) = entry.get("submenu").filter(|v| v.is_array()) {
            let submenu = Submenu::new(app, label, true).map_err(|e| e.to_string())?;
            for item in sub.as_array().unwrap() {
                add_submenu_entry(app, &submenu, item)?;
            }
            menu.append(&submenu).map_err(|e| e.to_string())?;
            continue;
        }

        // 预定义角色（copy/paste/quit 等）直接映射
        if let Some(role) = entry.get("role").and_then(|v| v.as_str()) {
            let item = match role {
                "quit" => PredefinedMenuItem::quit(app, Some(label)),
                "copy" => PredefinedMenuItem::copy(app, Some(label)),
                "cut" => PredefinedMenuItem::cut(app, Some(label)),
                "paste" => PredefinedMenuItem::paste(app, Some(label)),
                "undo" => PredefinedMenuItem::undo(app, Some(label)),
                "redo" => PredefinedMenuItem::redo(app, Some(label)),
                "selectAll" => PredefinedMenuItem::select_all(app, Some(label)),
                "minimize" => PredefinedMenuItem::minimize(app, Some(label)),
                "close" | "quit-window" => PredefinedMenuItem::close_window(app, Some(label)),
                "togglefullscreen" => PredefinedMenuItem::fullscreen(app, Some(label)),
                _ => PredefinedMenuItem::separator(app),
            };
            menu.append(&item.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            continue;
        }

        if label.is_empty() {
            menu.append(&PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            continue;
        }

        add_menu_entry(app, &menu, entry)?;
    }

    Ok(menu)
}

/// 把一个普通菜单项（带 click 回调）加到 menu 或 submenu 上
///
/// 用 MenuItem 自身的 id 作为事件标识 —— Tauri 的 on_menu_event 回调会带出这个 id，
/// 所以不需要额外的 "click -> tag" 注册表。
/// 把单个菜单项（label/enabled/click）构建出来。Menu 和 Submenu 都能 append 它。
fn build_menu_item<R: Runtime>(
    app: &AppHandle<R>,
    entry: &serde_json::Value,
) -> Result<tauri::menu::MenuItem<R>, String> {
    use tauri::menu::MenuItem;

    let label = entry.get("label").and_then(|v| v.as_str()).unwrap_or("");
    let enabled = entry.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    // click 字段前端放的是字符串 id，直接当 Tauri 菜单项 id 用；
    // on_menu_event 会带出这个 id，前端 `listen("menu:<id>")` 收到。
    let id = entry
        .get("click")
        .and_then(|v| v.as_str())
        .unwrap_or(label);

    MenuItem::with_id(app, id, label, enabled, None::<&str>).map_err(|e| e.to_string())
}

fn add_menu_entry<R: Runtime>(
    app: &AppHandle<R>,
    menu: &tauri::menu::Menu<R>,
    entry: &serde_json::Value,
) -> Result<(), String> {
    let item = build_menu_item(app, entry)?;
    menu.append(&item).map_err(|e| e.to_string())
}

fn add_submenu_entry<R: Runtime>(
    app: &AppHandle<R>,
    submenu: &tauri::menu::Submenu<R>,
    entry: &serde_json::Value,
) -> Result<(), String> {
    let item = build_menu_item(app, entry)?;
    submenu.append(&item).map_err(|e| e.to_string())
}

/// 供 lib.rs 的 on_menu_event 调用：把菜单 id 翻译成 `menu:<id>` 事件发给窗口
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, window: Option<&tauri::WebviewWindow<R>>, id: &str) {
    let tag = format!("menu:{id}");
    if let Some(win) = window {
        let _ = win.emit(&tag, true);
    } else {
        let _ = app.emit(&tag, true);
    }
}

// ---------------------------------------------------------------------------
// OAuth 登录窗口（Pavlovia GitLab）
//
// electron/src/tools/git.js 的 login() 需要：开一个能登录的窗口、清掉 cookie
// 强制重新登录、监听导航并在拿到 ?code= 时关闭窗口把 URL 返回。
// 这三件事 Node sidecar 做不到（没有 BrowserWindow / session），只能放 Rust。
//
// 契约：前端（经 sidecar 的 git.login）调用 `auth_window(url, hostname)`，
// 等到用户登录完成，返回最终的 callback URL；用户直接关窗则返回 Err。
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn auth_window<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    hostname: String,
) -> Result<String, String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let label = format!("oauth-{}", std::process::id());
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

    // on_navigation 只能在 builder 上挂（WebviewWindow 事后没有这个方法），
    // 所以回调判断写在这里，命中后 send 到 channel 并关窗。
    let tx_nav = tx.clone();
    let host = hostname.clone();
    let win = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(url.parse().map_err(|e| format!("bad url: {e}"))?),
    )
    .title("Pavlovia Login")
    .inner_size(980.0, 720.0)
    .on_navigation(move |target| {
        // callback = 同 host + 路径 "/" + 带 code 参数
        let is_callback = target
            .host_str()
            .map(|h| host.ends_with(h))
            .unwrap_or(false)
            && target.path() == "/"
            && target.query_pairs().any(|(k, _)| k == "code");
        if is_callback {
            let _ = tx_nav.send(Ok(target.to_string()));
        }
        // 拿到 code 后不再导航，避免把回调页真的渲染出来
        !is_callback
    })
    .build()
    .map_err(|e| e.to_string())?;

    // 强制重新登录：清掉这个 webview 的 cookie / 存储
    let _ = win.clear_all_browsing_data();

    // 用户直接关窗 = 取消
    let tx_close = tx.clone();
    let win_close = win.clone();
    win.on_window_event(move |evt| {
        if matches!(evt, tauri::WindowEvent::Destroyed) {
            let _ = tx_close.send(Err("Login window closed before completing".to_string()));
        }
    });

    // 等回调，带超时，避免窗口卡住时一直挂着
    let outcome = rx.recv_timeout(std::time::Duration::from_secs(300));
    let _ = win_close.close();

    match outcome {
        Ok(Ok(callback)) => Ok(callback),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Login timed out after 5 minutes".to_string()),
    }
}

// ---------------------------------------------------------------------------
// PsychoJS 本地运行（electron/src/python/psychojs.js）
//
// Electron 版：express.static(cwd) 起静态服务 + 一个专用 BrowserWindow 加载
// http://localhost:<port>?<params>，窗口带 reload/devtools 菜单。
//
// Tauri 侧必须是**真 HTTP**，不能用自定义协议：实验页面里 psychojs 会发
// XHR 拉取资源，自定义协议的 origin 会让这些请求全部跨域失败。
//
// 端口策略与 Electron 一致：从 8002 起找第一个空闲端口。
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;

/// 一个运行中的 PsychoJS 服务
struct PsychoJsServer {
    address: String,
    // 持有 server 句柄：stop 时 unblock() 让 incoming_requests() 结束，线程退出
    shutdown: Option<Arc<tiny_http::Server>>,
    root: std::path::PathBuf,
}

/// 全局服务表，key 是 "host:port"，与 Electron 版 `servers[address]` 对应
static PSYCHOJS_SERVERS: Mutex<Option<HashMap<String, Arc<PsychoJsServer>>>> = Mutex::new(None);

fn psychojs_servers() -> &'static Mutex<Option<HashMap<String, Arc<PsychoJsServer>>>> {
    &PSYCHOJS_SERVERS
}

/// 从 8002 起找一个能绑上的端口。
///
/// Electron 版用 tcp-port-used 预检再 listen（有 TOCTOU 空窗）；这里直接尝试 bind，
/// 拿到 handle 再用，没有竞态。
fn bind_safe_port() -> Result<(tiny_http::Server, u16), String> {
    for port in 8002..8100u16 {
        match tiny_http::Server::http(("127.0.0.1", port)) {
            Ok(server) => return Ok((server, port)),
            Err(_) => continue,
        }
    }
    Err("no free port in 8002..8100 for the PsychoJS server".to_string())
}

/// 把 URL 路径解析成磁盘路径，并挡掉路径穿越。
///
/// tiny_http 给的 url 已经包含了 query string，需要先切掉；
/// 再 percent-decode，最后确认规范化后的路径仍在 root 之内。
fn resolve_static_path(root: &std::path::Path, url: &str) -> Option<std::path::PathBuf> {
    let path_only = url.split(['?', '#']).next().unwrap_or("/");
    let decoded = percent_decode(path_only);
    // 去掉前导 '/'，并拒绝任何 ".." 片段
    let rel = decoded.trim_start_matches('/');
    if rel.split('/').any(|seg| seg == "..") {
        return None;
    }
    let full = if rel.is_empty() { root.join("index.html") } else { root.join(rel) };
    // 目录请求补 index.html
    let full = if full.is_dir() { full.join("index.html") } else { full };
    // 规范化后仍在 root 内才放行（挡符号链接逃逸）
    let canon_root = root.canonicalize().ok()?;
    let canon = full.canonicalize().ok()?;
    if !canon.starts_with(&canon_root) {
        return None;
    }
    Some(canon)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[tauri::command]
pub async fn psychojs_run<R: Runtime>(
    app: AppHandle<R>,
    cwd: String,
    params: Option<serde_json::Value>,
) -> Result<String, String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let root = std::path::PathBuf::from(&cwd);
    if !root.is_dir() {
        return Err(format!("PsychoJS cwd is not a directory: {cwd}"));
    }

    let (server, port) = bind_safe_port()?;
    let address = format!("127.0.0.1:{port}");
    let server = Arc::new(server);
    let root_for_thread = root.clone();
    let root_for_entry = root.clone();

    // 服务线程：一直跑，直到 server 被 unblock
    let server_thread = server.clone();
    std::thread::spawn(move || {
        for req in server_thread.incoming_requests() {
            serve_one(&root_for_thread, req);
        }
    });

    // 记到全局表，供 psychojs_stop 找到
    {
        let mut guard = psychojs_servers().lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(
            address.clone(),
            Arc::new(PsychoJsServer {
                address: address.clone(),
                shutdown: Some(server.clone()),
                root: root_for_entry,
            }),
        );
    }

    // 打开专用窗口（对应 Electron 版 server.win）
    let label = format!("psychojs-{port}");
    let url = format!("http://{address}{}", query_string(params));
    let win = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(
        url.parse().map_err(|e| format!("bad url: {e}"))?,
    ))
    .title("PsychoJS Experiment")
    .maximized(true)
    .build()
    .map_err(|e| e.to_string())?;

    // 窗口关闭 = 实验结束 = 停服务（与 Electron 版 "close" -> server.stop() 一致）
    let addr_for_close = address.clone();
    win.on_window_event(move |evt| {
        if matches!(evt, tauri::WindowEvent::Destroyed) {
            stop_psychojs(&addr_for_close);
        }
    });

    Ok(address)
}

/// 处理单个请求。抽成独立函数是为了能单测（不需要 Tauri 运行时）。
fn serve_one(root: &std::path::Path, req: tiny_http::Request) {
    if req.method() != &tiny_http::Method::Get && req.method() != &tiny_http::Method::Head {
        let _ = req.respond(tiny_http::Response::empty(405));
        return;
    }
    let url = req.url().to_string();
    let _ = req.respond(build_response(root, &url));
}

/// 把 URL 变成响应体。纯函数，无副作用，可单测。
///
/// 返回 boxed response 是为了让 200 和 404 两个分支类型一致
/// （tiny_http 的 Response<Cursor<Vec<u8>>> 与 Response<Empty> 不是同一类型）。
fn build_response(
    root: &std::path::Path,
    url: &str,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    if let Some(file) = resolve_static_path(root, url) {
        if let Ok(body) = std::fs::read(&file) {
            let mime = mime_guess::from_path(&file).first_or_octet_stream();
            let header = tiny_http::Header::from_bytes(
                &b"Content-Type"[..],
                mime.essence_str().as_bytes(),
            )
            .expect("valid header");
            return tiny_http::Response::from_data(body).with_header(header);
        }
    }
    // 404 用空 body 的 from_data，保持泛型参数一致
    tiny_http::Response::from_data(Vec::new()).with_status_code(404)
}

fn query_string(params: Option<serde_json::Value>) -> String {
    let Some(serde_json::Value::Object(map)) = params else {
        return String::new();
    };
    let pairs: Vec<String> = map
        .into_iter()
        .map(|(k, v)| {
            let val = match v {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            format!("{}={}", urlencode(&k), urlencode(&val))
        })
        .collect();
    if pairs.is_empty() {
        String::new()
    } else {
        format!("?{}", pairs.join("&"))
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn stop_psychojs(address: &str) {
    let mut guard = psychojs_servers().lock().unwrap();
    let Some(map) = guard.as_mut() else { return };
    if let Some(server) = map.remove(address) {
        if let Some(s) = &server.shutdown {
            s.unblock();
        }
    }
}

#[tauri::command]
pub async fn psychojs_stop(address: String) -> Result<(), String> {
    stop_psychojs(&address);
    Ok(())
}

// ---------------------------------------------------------------------------
// electron.clipboard.*
//
// 注意：这**不是**系统剪贴板。Electron 版里 `clipboard` 只是一个模块级变量
// （index.cjs:37 `clipboard = undefined`），用于在应用内复制粘贴实验节点。
// 前端 src/lib/utils/clipboard.js 也只是把它当内存存储用。
// 所以这里用 Mutex<Value> 即可，不要接 arboard/system clipboard。
// ---------------------------------------------------------------------------

use std::sync::Mutex;

static APP_CLIPBOARD: Mutex<Option<serde_json::Value>> = Mutex::new(None);

#[tauri::command]
pub fn clipboard_get() -> Option<serde_json::Value> {
    APP_CLIPBOARD.lock().ok().and_then(|g| g.clone())
}

#[tauri::command]
pub fn clipboard_set(value: Option<serde_json::Value>) {
    if let Ok(mut g) = APP_CLIPBOARD.lock() {
        *g = value;
    }
}

// ---------------------------------------------------------------------------
// electron.system.*（仅 macOS 有意义）
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn system_request_keyboard_access() -> Result<(), String> {
    // 非 macOS 直接成功（与 Electron 版一致）
    // TODO(M8): macOS 走 AXIsProcessTrustedWithOptions，需 objc2。
    // 现阶段不阻塞：绝大多数用户不会走到这个分支。
    Ok(())
}

#[tauri::command]
pub async fn system_has_keyboard_access() -> Result<bool, String> {
    // TODO(M8): macOS 查 AXIsProcessTrusted()，需 objc2。
    // 过渡期返回 true，避免前端误判为"需要授权"而骚扰用户。
    Ok(true)
}

// ---------------------------------------------------------------------------
// PsychoJS 静态服务的单元测试
//
// 路径解析是这里唯一的攻击面（路径穿越 / 符号链接逃逸），
// 且不需要 Tauri 运行时就能测，所以单独覆盖。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod psychojs_tests {
    use super::*;

    /// 每个测试用独立目录：cargo test 并发跑同一进程，
    /// 用 pid 命名会让多个测试共享（并被其中一个 remove_dir_all 删掉）。
    /// 用线程 id + 原子计数保证唯一。
    fn tmp_root() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("psyjs-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("index.html"), b"<html>root</html>").unwrap();
        std::fs::write(dir.join("lib").join("psychojs.js"), b"var x=1;").unwrap();
        dir
    }

    #[test]
    fn root_maps_to_index_html() {
        let root = tmp_root();
        let p = resolve_static_path(&root, "/").expect("root should resolve");
        assert!(p.ends_with("index.html"));
    }

    #[test]
    fn serves_nested_file() {
        let root = tmp_root();
        let p = resolve_static_path(&root, "/lib/psychojs.js").expect("nested file should resolve");
        assert!(p.ends_with("psychojs.js"));
    }

    #[test]
    fn strips_query_string() {
        let root = tmp_root();
        // 前端用 ?__pilotToken=local 之类加载，必须忽略 query 才能找到文件
        let p = resolve_static_path(&root, "/index.html?__pilotToken=local").expect("query must be stripped");
        assert!(p.ends_with("index.html"));
    }

    #[test]
    fn blocks_parent_traversal() {
        let root = tmp_root();
        assert!(resolve_static_path(&root, "/../etc/passwd").is_none(), ".. must be rejected");
        assert!(resolve_static_path(&root, "/lib/../../etc/passwd").is_none(), "nested .. must be rejected");
    }

    #[test]
    fn blocks_percent_encoded_traversal() {
        let root = tmp_root();
        // %2e%2e%2f = "../" —— 解码后必须仍然被拒
        assert!(resolve_static_path(&root, "/%2e%2e%2fetc%2fpasswd").is_none());
    }

    #[test]
    fn missing_file_is_none() {
        let root = tmp_root();
        assert!(resolve_static_path(&root, "/nope.js").is_none());
    }

    #[test]
    fn percent_decode_works() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("%2e%2e"), "..");
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn query_string_builds_from_params() {
        let params = serde_json::json!({"__pilotToken": "local", "runMode": "pilot"});
        let q = query_string(Some(params));
        assert!(q.starts_with('?'));
        assert!(q.contains("__pilotToken=local"));
        assert!(q.contains("runMode=pilot"));
        assert_eq!(query_string(None), "");
    }

    #[test]
    fn build_response_serves_html_with_mime() {
        let root = tmp_root();
        let resp = build_response(&root, "/index.html");
        assert_eq!(resp.status_code().0, 200);
        let ct = resp
            .headers()
            .iter()
            .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("content-type"))
            .map(|h| h.value.as_str().to_string())
            .unwrap_or_default();
        assert!(ct.starts_with("text/html"), "got content-type {ct}");
    }

    #[test]
    fn build_response_serves_js_with_mime() {
        let root = tmp_root();
        let resp = build_response(&root, "/lib/psychojs.js");
        assert_eq!(resp.status_code().0, 200);
    }

    #[test]
    fn build_response_404s_missing_file() {
        let root = tmp_root();
        let resp = build_response(&root, "/missing.js");
        assert_eq!(resp.status_code().0, 404);
    }

    #[test]
    fn build_response_404s_traversal() {
        let root = tmp_root();
        let resp = build_response(&root, "/../secret");
        assert_eq!(resp.status_code().0, 404);
    }

    #[test]
    fn urlencode_escapes_specials() {
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("safe-._~"), "safe-._~");
    }
}

#[cfg(test)]
mod psychojs_port_tests {
    /// 端口探测：已被占用的端口必须跳过。
    ///
    /// 实测本机上 8002 常被 Python liaison 进程占用，
    /// 所以这里显式占一个端口再验证探测会跳过它。
    #[test]
    fn bind_safe_port_skips_occupied() {
        use super::bind_safe_port;
        // 先占住 8002（如果空闲）——用 std 的 TcpListener，不用 tiny_http
        let holder = std::net::TcpListener::bind("127.0.0.1:8002");
        let Ok(holder) = holder else {
            // 8002 本来就占用，同样说明了场景，直接验证能返回别的端口
            let (server, port) = bind_safe_port().expect("should find a free port");
            assert_ne!(port, 8002);
            drop(server);
            return;
        };

        let (server, port) = bind_safe_port().expect("should find a free port");
        assert_ne!(port, 8002, "must not return the occupied port");
        assert!(port > 8002);
        drop(server);
        drop(holder);
    }
}

// ---------------------------------------------------------------------------
// PsychoJS HTTP 服务的真实往返测试
//
// 前面 13 个测试只验证了"函数返回什么"，没验证"网络请求能得到什么"。
// 服务端口探测 + 真实 HTTP 请求这一层最容易出错（MIME、chunk、keep-alive），
// 所以这里起一个真 server，用真 HTTP client 打一遍。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod http_e2e_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    /// 在指定端口起服务线程，返回地址
    fn serve_on(root: std::path::PathBuf, port: u16) -> String {
        let addr = format!("127.0.0.1:{port}");
        let server = tiny_http::Server::http(&addr).expect("bind");
        std::thread::spawn(move || {
            for req in server.incoming_requests() {
                serve_one(&root, req);
            }
        });
        addr
    }

    /// 发一个 GET，返回 (状态码, content-type, body)
    fn get(addr: &str, path: &str) -> (u16, String, String) {
        let mut stream = TcpStream::connect(addr).expect("connect");
        write!(stream, "GET {path} HTTP/1.0\r\nHost: {addr}\r\n\r\n").unwrap();
        stream.flush().unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let raw = String::from_utf8_lossy(&raw).to_string();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or(("", ""));
        let status = head
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|c| c.parse::<u16>().ok())
            .unwrap_or(0);
        let ct = head
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-type:"))
            .map(|l| l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_default();
        (status, ct, body.to_string())
    }

    /// 同 tmp_root：每测试独立目录，避免并发互相删除
    fn fixture() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("psyjs-e2e-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("index.html"), b"<!doctype html><h1>PSYCHJS_SERVED_OK</h1>").unwrap();
        std::fs::write(
            dir.join("lib").join("psychojs-2024.2.js"),
            b"var PSYCHJS_OK = true;",
        )
        .unwrap();
        dir
    }

    #[test]
    fn serves_index_over_real_http() {
        let root = fixture();
        let addr = serve_on(root, 18002);

        let (status, _ct, body) = get(&addr, "/");
        assert_eq!(status, 200);
        assert!(body.contains("PSYCHJS_SERVED_OK"), "body: {body}");
    }

    #[test]
    fn serves_library_asset_with_js_mime() {
        let root = fixture();
        let addr = serve_on(root, 18003);

        let (status, ct, body) = get(&addr, "/lib/psychojs-2024.2.js");
        assert_eq!(status, 200);
        assert!(ct.contains("javascript"), "content-type: {ct}");
        assert!(body.contains("PSYCHJS_OK"));
    }

    #[test]
    fn ignores_query_string_in_real_http() {
        // 前端加载 index.html 会带 ?__pilotToken=local 之类参数，必须照常命中文件
        let root = fixture();
        let addr = serve_on(root, 18004);

        let (status, _ct, body) = get(&addr, "/index.html?__pilotToken=local");
        assert_eq!(status, 200);
        assert!(body.contains("PSYCHJS_SERVED_OK"));
    }

    #[test]
    fn returns_404_for_missing_asset() {
        let root = fixture();
        let addr = serve_on(root, 18005);

        let (status, _ct, _body) = get(&addr, "/lib/does-not-exist.js");
        assert_eq!(status, 404);
    }

    #[test]
    fn returns_404_for_traversal_over_real_http() {
        let root = fixture();
        let addr = serve_on(root, 18006);

        let (status, _ct, _body) = get(&addr, "/../../../etc/passwd");
        assert_eq!(status, 404);
    }
}

#[cfg(test)]
mod window_target_tests {
    use super::{window_label, window_url};

    #[test]
    fn plain_target_becomes_directory_index() {
        // 预渲染产出 dist/builder/index.html
        match window_url(Some("builder")) {
            tauri::WebviewUrl::App(p) => assert_eq!(p.display().to_string(), "builder/index.html"),
            _ => panic!("expected App url"),
        }
    }

    #[test]
    fn query_is_appended_after_index_html_not_swallowed_by_the_path() {
        // 回归：早先把 target 整体当路径，拼出
        // "builder?fileOpen=x/index.html" —— 窗口直接白屏，且 label 非法。
        match window_url(Some("builder?fileOpen=/tmp/a.psyexp")) {
            tauri::WebviewUrl::App(p) => assert_eq!(p.display().to_string(), "builder/index.html?fileOpen=/tmp/a.psyexp"),
            _ => panic!("expected App url"),
        }
    }

    #[test]
    fn empty_target_loads_root_index() {
        match window_url(None) {
            tauri::WebviewUrl::App(p) => assert_eq!(p.display().to_string(), "index.html"),
            _ => panic!("expected App url"),
        }
    }

    #[test]
    fn label_excludes_illegal_characters() {
        // Tauri 的 label 只允许 alphanumeric 与 - / : _
        let label = window_label(Some("builder?fileOpen=/tmp/a.psyexp"));
        assert_eq!(label, "builder");
        assert!(label.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | ':' | '_')));
    }

    #[test]
    fn label_is_stable_for_same_target_prefix() {
        // windows_get 用的就是 label 去重，同一视图必须映射到同一 label
        assert_eq!(
            window_label(Some("runner?fileOpen=/a.psyrun")),
            window_label(Some("runner?projectOpen=g/p"))
        );
    }

    #[test]
    fn no_target_uses_main() {
        assert_eq!(window_label(None), "main");
        assert_eq!(window_label(Some("")), "main");
    }
}
