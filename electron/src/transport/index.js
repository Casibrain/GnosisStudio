/**
 * 传输层抽象 —— M2
 *
 * 目标：让所有 `electron/src/**` 里的 handler 注册与 Electron 解耦，
 * 使同一份业务代码既能在 Electron 下跑，也能在 Node sidecar 下跑。
 *
 * 用法（业务代码只需改 import）：
 *   import { handle, on } from "<path>/transport/index.js";
 *   export const handlers = {
 *     foo: handle("python.foo.bar", async (evt, a, b) => ...)
 *   };
 *
 * Electron 实现保持原有语义（evt 首参、handler 抛异常 → reject）。
 * JSON-RPC 实现会去掉 evt 首参，并把异常转成 JSON-RPC error。
 *
 * 关键约束（见 docs/ipc-contract.md §3）：
 *   - 错误语义：Tauri 侧 reject 是裸字符串，不要包装成 {error: ...}
 *   - 同步通道：Electron 的 sendSync 无等价物，getProfiles 已决定改异步（D6）
 *   - 事件广播：必须保留层级广播 + 全窗口广播
 *   - **来源窗口**：业务代码里 9 处用了 `evt.sender.id`（openDialog/saveDialog/
 *     updateFrame/windows.focus|devtools|close|hideMenu|setMenu）。transport 必须
 *     保证第一个参数上有可用的来源标识，否则这些通道会在 sidecar 下静默失效。
 *     Electron: evt.sender.id 是数字；JSON-RPC: evt.sender.id 是窗口 label 字符串。
 *     调用方用 `senderId(evt)` 取值，不要直接摸 evt.sender.id。
 */

import * as electron from "./electron.js";
import * as jsonrpc from "./jsonrpc.js";

/**
 * 传输后端注册表。默认 electron，sidecar 启动时切到 jsonrpc。
 * @type {{handle: Function, on: Function, sendEvent: Function, listHandlers: Function}}
 */
let backend = electron;

/**
 * 切换传输后端
 * @param {"electron"|"jsonrpc"} name
 * @param {object} [options] jsonrpc 后端的配置（端口/管道名等）
 */
export function useTransport(name, options = {}) {
    if (name === "electron") {
        backend = electron;
    } else if (name === "jsonrpc") {
        backend = jsonrpc.create(options);
    } else {
        throw new Error(`Unknown transport: ${name}`);
    }
    return backend;
}

/** 当前后端（测试用） */
export function currentTransport() {
    return backend;
}

/**
 * 注册一个请求-响应通道
 * @param {string} channel 通道名（如 "python.uv.install"）
 * @param {Function} fn handler，签名与原先 ipcMain.handle 一致：(evt, ...args)
 * @returns {Function} 原样返回 fn，保持 `handlers.xxx = handle(...)` 的赋值形式不变
 */
export function handle(channel, fn) {
    backend.handle(channel, fn);
    return fn;
}

/**
 * 注册一个同步通道（仅 Electron 支持；jsonrpc 下抛错，因为已决定改异步）
 * @param {string} channel
 * @param {Function} fn
 * @returns {Function}
 */
export function on(channel, fn) {
    backend.on(channel, fn);
    return fn;
}

/**
 * 向后端 -> 前端广播事件
 * @param {string} tag 支持 ":" 层级（a:b:c 会同时发 a:b:c、a:b、a）
 * @param {*} message
 */
export function sendEvent(tag, message) {
    backend.sendEvent(tag, message);
}

/**
 * 列出已注册通道（用于生成契约校验）
 */
export function listHandlers() {
    return backend.listHandlers();
}

/**
 * 取发起来源的窗口标识（替代直接读 `evt.sender.id`）。
 *
 * - Electron: 数字 webContents id
 * - JSON-RPC: 前端 invoke 时显式带上的窗口 label 字符串
 *
 * @param {{sender?: {id: number|string}}} evt
 * @returns {number|string|undefined}
 */
export function senderId(evt) {
    return evt?.sender?.id;
}
