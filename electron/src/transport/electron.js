/**
 * Electron 传输后端 —— 行为与直接调用 ipcMain 完全一致。
 *
 * 这个文件是迁移期间 Electron 的默认路径，确保现有功能零回归。
 */

import { ipcMain, BrowserWindow } from "electron";

const registered = {
    handle: new Set(),
    on: new Set(),
};

export function handle(channel, fn) {
    // 原样转发，保持 (evt, ...args) 签名
    ipcMain.handle(channel, fn);
    registered.handle.add(channel);
}

export function on(channel, fn) {
    ipcMain.on(channel, fn);
    registered.on.add(channel);
}

/**
 * 广播事件。必须保留两个行为（见 docs/ipc-contract.md §3.3）：
 *   1. 层级广播：a:b:c -> 也发 a:b、a
 *   2. 全窗口广播
 */
export function sendEvent(tag, message) {
    const channels = [];
    if (tag) {
        channels.push(tag);
        let t = tag;
        while (t?.includes?.(":")) {
            t = t.substring(0, t.lastIndexOf(":"));
            channels.push(t);
        }
    }
    for (const channel of channels) {
        BrowserWindow.getAllWindows().forEach(
            win => win.webContents.send(channel, message)
        );
    }
}

export function listHandlers() {
    return {
        handle: [...registered.handle],
        on: [...registered.on],
    };
}
