//! 深链与单实例。
//!
//! 对应 Electron 版 electron/src/index.cjs 的两段：
//!   `app.requestSingleInstanceLock()` —— 第二实例退出，argv 交给第一实例
//!   `app.setAsDefaultProtocolClient('psychopy-studio')` + `open-url` / `open-file`
//!
//! 入参分发语义（必须与 Electron 版一致，前端路由依赖它）：
//!   psychopy-studio://<group>/<name>  ->  runner?projectOpen=<group>/<name>
//!   *.psyexp                          ->  builder?fileOpen=<path>
//!   *.psyrun                          ->  runner?fileOpen=<path>
//!   其他                              ->  忽略并记日志（不要开空窗口）
//!
//! 与 Electron 的差别：Electron 的 argv[1] 直接是文件路径；
//! Tauri 的 argv[0] 是可执行文件，从 argv[1..] 里找。

use serde_json::json;
use tauri::{AppHandle, Emitter, Runtime, Url};

/// 协议 scheme。与 Electron 版 `psychopy-studio` 保持一字不差——
/// 改了用户机器上已注册的默认打开方式就失效了。
pub const SCHEME: &str = "psychopy-studio";

/// 把一条入参（URL 或文件路径）转成前端路由。
///
/// 抽成纯函数是为了能单测：这是唯一有真分支的逻辑，
/// 而它错了的表现是"双击文件没反应"，排查成本很高。
pub fn route_for(target: &str) -> Option<String> {
    if target.is_empty() {
        return None;
    }

    if let Some(rest) = target.strip_prefix("psychopy-studio://") {
        // Electron 版正则：/(?<group>.*?)\/(?<name>.*?)$/
        // 只按第一个 '/' 切开，group 里可能含 ':' 之类
        let (group, name) = rest.split_once('/')?;
        if group.is_empty() || name.is_empty() {
            return None;
        }
        return Some(format!("runner?projectOpen={group}/{name}"));
    }

    if target.ends_with(".psyexp") {
        return Some(format!("builder?fileOpen={target}"));
    }
    if target.ends_with(".psyrun") {
        return Some(format!("runner?fileOpen={target}"));
    }

    None
}

/// 开一个窗口加载给定路由。复用 system::new_window 的语义（按路由前缀去重）。
///
/// Electron 版在 macOS 上对 fileOpen 不开新窗口而是复用；这里统一交给
/// new_window 的 `windows_get` 去重逻辑，行为已对齐。
fn open_route<R: Runtime>(app: &AppHandle<R>, route: &str) {
    let app = app.clone();
    let route = route.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::system::windows_new(app, Some(route)).await {
            eprintln!("[deeplink] failed to open window: {e}");
        }
    });
}

/// 处理一条入参。返回是否被识别。
pub fn handle_target<R: Runtime>(app: &AppHandle<R>, target: &str) -> bool {
    match route_for(target) {
        Some(route) => {
            println!("[deeplink] {target} -> {route}");
            open_route(app, &route);
            true
        }
        None => {
            if !target.is_empty() {
                // 与 Electron 版一致：记日志，不开窗口
                eprintln!("[deeplink] not a PsychoPy file or link: {target}");
            }
            false
        }
    }
}

/// 启动时从命令行取文件参数。
///
/// 只认第一个像样的候选：Electron 版读的是 `process.argv[1]`，
/// 但 Tauri 下 argv[0] 是可执行文件，且 macOS 的 `-psn_xxx` 之类的系统参数
/// 也会混进来，所以这里过滤掉以 '-' 开头的项。
pub fn handle_argv<R: Runtime>(app: &AppHandle<R>, argv: &[String]) {
    for arg in argv.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        if route_for(arg).is_some() {
            handle_target(app, arg);
            return; // 一次只开一个，和 Electron 的 argv[1] 语义一致
        }
    }
}

/// 注册深链插件与事件回调。
///
/// macOS 用 open-url（kAEGetURL），Windows/Linux 用 argv 传递给第二实例。
pub fn register<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    use tauri_plugin_deep_link::DeepLinkExt;

    // OpenUrlEvent 只暴露 urls()，不携带 AppHandle，
    // 所以在注册时就把它捕获进闭包。
    let handle = app.handle().clone();
    app.deep_link().on_open_url(move |event| {
        let urls: Vec<Url> = event.urls();
        let Some(url) = urls.first() else { return };
        handle_target(&handle, url.as_str());
    });
    Ok(())
}

/// 把当前文件/链接转发给前端（前端也可以主动监听）。
///
/// 前端路由是硬跳转（new_window 直接加载 X/index.html），
/// 这个事件是给"已经开着主窗口、只想要通知"的场景用的补充；
/// 目前没有调用方，先留着以免以后要加时改 Rust。
#[allow(dead_code)]
pub fn emit_open<R: Runtime>(app: &AppHandle<R>, target: &str) {
    let route = route_for(target).unwrap_or_default();
    let _ = app.emit("open-target", json!({ "target": target, "route": route }));
}

#[cfg(test)]
mod tests {
    use super::route_for;

    #[test]
    fn deep_link_maps_to_runner_project() {
        // Electron 版正则 (?<group>.*?)/(?<name>.*?)$ 的等价行为
        assert_eq!(
            route_for("psychopy-studio://mygroup/myproject"),
            Some("runner?projectOpen=mygroup/myproject".into())
        );
    }

    #[test]
    fn deep_link_with_nested_name_keeps_everything_after_first_slash() {
        // group 之后的 '/' 都归属 name —— 与 .*?$ 对 name 的贪婪到结尾一致
        assert_eq!(
            route_for("psychopy-studio://group/sub/name"),
            Some("runner?projectOpen=group/sub/name".into())
        );
    }

    #[test]
    fn deep_link_without_name_is_ignored() {
        assert_eq!(route_for("psychopy-studio://grouponly"), None);
        assert_eq!(route_for("psychopy-studio://group/"), None);
        assert_eq!(route_for("psychopy-studio://"), None);
    }

    #[test]
    fn psyexp_opens_builder_psyrun_opens_runner() {
        assert_eq!(
            route_for("/Users/x/my.psyexp"),
            Some("builder?fileOpen=/Users/x/my.psyexp".into())
        );
        assert_eq!(
            route_for("/Users/x/run.psyrun"),
            Some("runner?fileOpen=/Users/x/run.psyrun".into())
        );
    }

    #[test]
    fn other_files_are_ignored() {
        // 与 Electron 版一致：非 PsychoPy 文件记日志，不开窗口
        assert_eq!(route_for("/Users/x/notes.txt"), None);
        assert_eq!(route_for(""), None);
    }

    #[test]
    fn similar_prefix_does_not_match() {
        // 防止 "psychopy-studio2://" 或把 .psyexp 当前缀的错误匹配
        assert_eq!(route_for("psychopy-studio2://a/b"), None);
        assert_eq!(route_for("/tmp/.psyexp.bak"), None);
    }
}
