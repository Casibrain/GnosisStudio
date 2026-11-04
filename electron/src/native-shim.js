/**
 * 原生能力 shim —— sidecar 侧的 dialog / shell 实现。
 *
 * 这些能力本质上需要系统 GUI，sidecar 是 headless 的，所以两种做法：
 *   1. 转给 Rust（Rust 有 tauri-plugin-dialog / opener）—— 最终形态
 *   2. 用 Node 尽力而为 —— 过渡形态，保证 sidecar 能独立跑通测试
 *
 * 当前阶段（M3）用 2，并通过 `setNativeBridge` 预留 1 的接口。
 * M8 接 Rust 时只需在 sidecar 启动时注入 bridge，业务代码零改动。
 */

import { exec } from "node:child_process";
import os from "node:os";

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
 * 同步对话框 —— Electron 的 showOpenDialogSync / showSaveDialogSync。
 *
 * sidecar 是异步进程，真正的同步对话框只能由宿主提供。
 * 过渡期直接抛错，避免"静默返回 null 让用户以为取消了"这种更糟的失败。
 */
const dialog = {
    showOpenDialogSync(options) {
        if (bridge.openDialog) return bridge.openDialog(options);
        throw new Error(
            "dialog.showOpenDialogSync requires the host (Rust) bridge; " +
            "inject via setNativeBridge() at sidecar startup"
        );
    },
    showSaveDialogSync(options) {
        if (bridge.saveDialog) return bridge.saveDialog(options);
        throw new Error(
            "dialog.showSaveDialogSync requires the host (Rust) bridge; " +
            "inject via setNativeBridge() at sidecar startup"
        );
    },
};

/**
 * shell —— 打开外部链接 / 定位文件。这些用 Node 就能做，不必转 Rust。
 */
function run(cmd) {
    return new Promise((resolve, reject) => {
        exec(cmd, err => (err ? reject(err) : resolve()));
    });
}

/** 按平台返回"用默认程序打开"的命令 */
function openerFor(target) {
    if (process.platform === "darwin") return `open "${target}"`;
    if (process.platform === "win32") return `start "" "${target}"`;
    return `xdg-open "${target}"`;
}

/** 按平台返回"在文件管理器中显示"的命令 */
function revealFor(target) {
    if (process.platform === "darwin") return `open -R "${target}"`;
    if (process.platform === "win32") return `explorer /select,"${target}"`;
    return `xdg-open "$(dirname "${target}")"`;
}

const shell = {
    async openExternal(url) {
        if (bridge.openExternal) return bridge.openExternal(url);
        await run(openerFor(url));
    },
    async openPath(target) {
        if (bridge.openPath) return bridge.openPath(target);
        await run(openerFor(target));
    },
    async showItemInFolder(target) {
        if (bridge.showItemInFolder) return bridge.showItemInFolder(target);
        await run(revealFor(target));
    },
};

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
