// GnosisStudio — Tauri 后端入口
//
// M0：验证 $state + invoke 可行（结论见 docs/tauri-migration-plan.md §10）
// M3：系统能力命令落地（system.rs），sidecar 通过 stdio 承接业务逻辑
//
// 分层：
//   Rust   —— 窗口、菜单、文件对话框、剪贴板、路径、单实例、打包
//   Node   —— 业务逻辑（files / git / python / state），以 sidecar 子进程运行
//   Python —— psychopy 环境，由 sidecar 调度
#![allow(dead_code)]

use serde_json::json;

mod deeplink;
mod sidecar;
mod system;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例必须第一个注册：拿不到锁的实例在这里直接退出，
        // 它的 argv 会回调给已运行的实例（对应 Electron 的 requestSingleInstanceLock）。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // 第二实例带着文件/链接启动时，交给第一实例处理
            deeplink::handle_argv(app, &argv);
        }))
        .plugin(tauri_plugin_opener::init())
        // psychopy-studio:// 深链。macOS 走 open-url，其余平台走第二实例的 argv。
        .plugin(tauri_plugin_deep_link::init())
        // 菜单点击：把菜单项 id 变成 `menu:<id>` 事件发回前端
        // （前端 src/lib/ui/menu/frameMenu/Item.svelte:68 用 listen("menu:<id>") 收）
        .on_menu_event(|app, event| {
            system::handle_menu_event(app, None, event.id().as_ref());
        })
        .setup(|_app| {
            // M5 自检：路径解析是迁移中最容易静默出错的部分（拼错 appName，
            // 用户已有的 venv/设置就"消失"了）。启动时打一遍，便于和 Electron 版比对。
            if std::env::var("GNOSTIS_SELFCHECK").is_ok() {
                println!("[selfcheck] platform        = {}", system::platform());
                println!("[selfcheck] paths_user      = {}", system::paths_user());
                println!("[selfcheck] paths_documents = {}", system::paths_documents());
                println!("[selfcheck] paths_devices   = {}", system::paths_devices());
                println!("[selfcheck] paths_prefs     = {}", system::paths_prefs());
                println!("[selfcheck] paths_pavlovia  = {}", system::paths_pavlovia());
                println!("[selfcheck] pavlovia.users  = {}", system::paths_pavlovia_users());
                println!("[selfcheck] pavlovia.projs  = {}", system::paths_pavlovia_projects());
            }
            // 深链回调：Windows/Linux 需要显式注册，macOS 在打包后由 Info.plist 声明
            if let Err(e) = deeplink::register(_app) {
                eprintln!("[deeplink] register failed: {e}");
            }
            // 首次启动就带文件参数（双击 .psyexp、或 open 命令）时立刻打开
            let argv: Vec<String> = std::env::args().collect();
            deeplink::handle_argv(_app.handle(), &argv);

            // 启动 Node sidecar：业务逻辑（files / git / python / state）都在里面。
            // 失败不 panic —— 前端可以调 node_ready() 判断并显示"后端未就绪"，
            // 总比整个窗口打不开好。
            if let Err(e) = sidecar::start(_app.handle()) {
                eprintln!("[sidecar] failed to start: {e}");
            }

            // 自检：前端要等 webview 加载、模块求值，中间任何一环出错都会白屏，
            // 而且我们在 Rust 侧完全看不到。所以自己先打一遍全链路，
            // 确认 Rust -> sidecar -> 回包 这条主干是通的。
            if std::env::var("GNOSTIS_SELFCHECK").is_ok() {
                tauri::async_runtime::spawn(async {
                    for channel in ["git.server", "git.loadUsers", "python.uv.folder"] {
                        match sidecar::node_call(channel.into(), json!([])).await {
                            Ok(v) => println!("[selfcheck] node_call {channel} = {v}"),
                            Err(e) => println!("[selfcheck] node_call {channel} FAILED: {e}"),
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // electron.paths.*
            system::paths_documents,
            system::paths_user,
            system::paths_devices,
            system::paths_prefs,
            system::paths_pavlovia,
            system::paths_pavlovia_users,
            system::paths_pavlovia_projects,
            // electron.platform / version / quit
            system::platform,
            system::app_version,
            system::quit,
            // electron.windows.*
            system::windows_new,
            system::windows_get,
            system::windows_send,
            system::windows_focus,
            system::windows_close,
            system::windows_devtools,
            system::windows_hide_menu,
            system::windows_set_menu,
            // electron.clipboard.*
            system::clipboard_get,
            system::clipboard_set,
            // electron.system.*
            system::system_request_keyboard_access,
            system::system_has_keyboard_access,
            system::auth_window,
            system::psychojs_run,
            system::psychojs_stop,
            // Node sidecar 桥接
            sidecar::node_call,
            sidecar::node_ready,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // 退出时先让 sidecar 落盘再放手（它靠 stdin 关闭触发 saveState）
            if let tauri::RunEvent::Exit = event {
                sidecar::stop();
            }
        });
}
