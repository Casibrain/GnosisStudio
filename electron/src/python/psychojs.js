/**
 * PsychoJS 本地运行 —— 转发到 Rust 宿主。
 *
 * Electron 版：express.static(cwd) 起服务 + 专用 BrowserWindow 加载
 * http://localhost:<port>?<params>。
 *
 * Tauri 版：见 src-tauri/src/system.rs 的 `psychojs_run` / `psychojs_stop`。
 *
 * 必须是真 HTTP —— 自定义协议不行：实验页面里 psychojs 会发 XHR 取资源，
 * 自定义协议的 origin 会让这些请求全部跨域失败。
 *
 * 端口从 8002 起找第一个空闲的（与 Electron 版一致），返回 "127.0.0.1:<port>"。
 */

import { hostRequest } from "../sidecar-host.mjs";

export async function run(cwd, params = {}) {
    return await hostRequest("psychojs_run", [cwd, params]);
}

export async function stop(address) {
    return await hostRequest("psychojs_stop", [address]);
}

export function getPsychoJSServer(address) {
    return { stop: () => stop(address) };
}
