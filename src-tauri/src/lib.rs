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
use tauri::Manager;

mod deeplink;
mod sidecar;
mod system;

/// 诊断用：让页面把状态回传给 Rust 打印。
/// webview 的 console 在 GUI 应用下拿不到，这是唯一可靠的回传通道。
#[tauri::command]
fn diag_report(key: String, value: String) {
    println!("[diag] {key} = {value}");
    // 同时写文件：GUI 下 stdout 可能被重定向到 /dev/null
    let _ = std::fs::write(format!("/tmp/gnosis-diag-{key}.txt"), &value);
}

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
            // 白屏排查：GNOSTIS_DUMP_DOM=1 时把页面真实状态画到左上角，截图即可读。
            // webview 的 console 在 GUI 下看不到；截图只能看出"白"，看不出
            // "白是因为没渲染"还是"因为 CSS 变量没加载"。
            if std::env::var("GNOSTIS_DUMP_DOM").is_ok() {
                let h = _app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    println!("[dom] dumping after 10s");
                    match h.get_webview_window("main") {
                        Some(_) => println!("[dom] found main window"),
                        None => println!("[dom] NO main window, have: {:?}", h.webview_windows().keys().collect::<Vec<_>>()),
                    }
                    if let Some(w) = h.get_webview_window("main") {
                        let js = r#"
                          (function(){
                            var out = [];
                            try {
                              out.push('electron=' + (typeof window.electron));
                              var c = document.querySelector('.container');
                              var cs = c ? getComputedStyle(c) : null;
                              out.push('bg=' + (cs ? cs.backgroundColor : 'no-container'));
                              out.push('mantle=' + (cs ? ('[' + cs.getPropertyValue('--mantle') + ']') : 'nil'));
                              out.push('views=' + document.querySelectorAll('.view').length);
                              out.push('links=' + document.querySelectorAll('link[rel=stylesheet]').length);
                            } catch(e) { out.push('ERR:' + e.message); }
                            var box = document.createElement('pre');
                            box.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:2147483647;background:#000;color:#0f0;font:16px monospace;padding:12px;margin:0;white-space:pre-wrap';
                            box.textContent = 'DIAG\n' + out.join('\n');
                            document.body.appendChild(box);
                            document.title = 'DIAG-' + out.join(';');
                          })()
                        "#;
                        match w.eval(js) {
                            Ok(()) => println!("[dom] eval ok"),
                            Err(e) => println!("[dom] eval failed: {e}"),
                        }
                        // 强制置前并移到主屏可见区域，否则截图拍到的是别的窗口
                        let _ = w.unminimize();
                        let _ = w.show();
                        let _ = w.set_always_on_top(true);
                        let _ = w.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(50, 50)));
                        let _ = w.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(1500, 900)));
                        let _ = w.set_focus();
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
            system::windows_emit,
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
            diag_report,
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
