/**
 * 原生能力 shim —— sidecar 侧的 dialog / shell 实现。
 *
 * 这些能力需要系统 GUI，而 sidecar 是 headless 的，所以一律转给 Rust：
 * 启动时由 sidecar.mjs 用 setNativeBridge() 注入 tauri-plugin-dialog /
 * tauri-plugin-opener 的实现，本文件只保留形状（签名与 Electron 对齐）
 * 和"没有宿主就显式报错"的兜底。
 *
 * 不再保留 Node 侧的 exec 兜底：那条路径拼 shell 命令且未转义，
 * 只在特定目录名下出错，比直接报错更难查。
 */

/**
 * @type {{openDialog?: Function, saveDialog?: Function, openPath?: Function,
 *         openExternal?: Function, showItemInFolder?: Function, authWindow?: Function}}
 */
let bridge = {};

/**
 * 注入 Rust 侧实现（M8 调用）
 * @param {object} b
 */
export function setNativeBridge(b) {
    bridge = { ...bridge, ...b };
}

/**
 * 对话框 —— 对应 Electron 的 showOpenDialogSync / showSaveDialogSync。
 *
 * 签名必须与 Electron 一致：`(win, options)`。
 * files.js:74 就是这么调的（`dialog.showOpenDialogSync(win, options)`），
 * 早先只写 `(options)` 会把 win 当成 options —— title/filters/defaultPath
 * 全部丢失，且不报错。
 *
 * 注意 **返回的是 Promise，不是同步值**。名字里的 Sync 只是为了与 Electron
 * 的调用形状对齐。这能成立是因为唯一的调用链是
 *   files.js:265 handle(...) -> transport/jsonrpc.js:73 `await fn(...)`
 * 中间没有任何地方把它当同步值用。若将来出现同步调用点（如 Electron 下
 * 直接调 files.js 的 openDialog），必须改成 await —— 否则拿到的是 Promise
 * 对象，真值判断恒为真，用户取消反而会被当成选了路径。
 */
const dialog = {
    showOpenDialogSync(win, options) {
        // 兼容只传 options 的调用（Electron 允许省略 win）
        if (options === undefined) {
            options = win;
        }
        if (bridge.openDialog) return bridge.openDialog(options);
        throw new Error(
            "dialog.showOpenDialogSync requires the host (Rust) bridge; " +
            "inject via setNativeBridge() at sidecar startup"
        );
    },
    showSaveDialogSync(win, options) {
        if (options === undefined) {
            options = win;
        }
        if (bridge.saveDialog) return bridge.saveDialog(options);
        throw new Error(
            "dialog.showSaveDialogSync requires the host (Rust) bridge; " +
            "inject via setNativeBridge() at sidecar startup"
        );
    },
};

/**
 * shell —— 打开外部链接 / 定位文件。
 *
 * 这几个能力都需要宿主，由 tauri-plugin-opener 实现（见 sidecar.mjs）。
 * 早先这里用 exec(`open "${target}"`) 兜底，但未转义的插值在路径含空格、
 * 引号或 $() 时就会坏，Windows 的 `start ""` 更脆。宁可显式报错，
 * 也不要留一条"有时能用"的路径 —— 那种 bug 只在用户的目录名带空格时出现。
 */
const shell = {
    async openExternal(url) {
        if (bridge.openExternal) return bridge.openExternal(url);
        throw missingHost("shell.openExternal");
    },
    async openPath(target) {
        if (bridge.openPath) return bridge.openPath(target);
        throw missingHost("shell.openPath");
    },
    async showItemInFolder(target) {
        if (bridge.showItemInFolder) return bridge.showItemInFolder(target);
        throw missingHost("shell.showItemInFolder");
    },
};

function missingHost(what) {
    return new Error(
        `${what} needs the host (Rust) bridge; inject via setNativeBridge() at sidecar startup`
    );
}

export { dialog, shell };


/**
 * 打开一个 OAuth 登录窗口并等用户登录完成。
 *
 * git.js 的 login() 需要真实的 BrowserWindow + session（清 cookie），
 * Node sidecar 造不出来，所以必须由 Rust 宿主提供。
 *
 * @param {string} url 要加载的授权页地址
 * @param {string} hostname 回调判定用的 host（拿到 ?code= 即完成）
 * @returns {Promise<string>} 最终的 callback URL
 */
export async function authWindow(url, hostname) {
    if (bridge.authWindow) {
        return await bridge.authWindow(url, hostname);
    }
    throw new Error(
        "OAuth login needs the host (Rust) bridge; inject via setNativeBridge({ authWindow })"
    );
}
