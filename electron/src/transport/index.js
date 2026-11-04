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
 * 关键约束（见 docs/ipc-contract.md §3）：
 *   - 错误语义：Tauri 侧 reject 是裸字符串，不要包装成 {error: ...}
 *   - 同步通道：Electron 的 sendSync 无等价物，getProfiles 已决定改异步（D6）
 *   - 事件广播：必须保留层级广播 + 全窗口广播
 *   - 来源窗口：业务代码里 8 处用了 `evt.sender.id`，用 senderId(evt) 取值
 *
 * 后端**按需静态导入**：electron 后端只在真正 useTransport("electron") 时加载，
 * 否则 sidecar 会因为 transport/electron.js 顶层 import ipcMain 而崩。
 * 为避免异步，这里用同步 require（require(ESM) 在 Node 22 可用且共享实例）。
 */

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

/**
 * 传输后端。默认 electron。
 * @type {{handle: Function, on: Function, sendEvent: Function, listHandlers: Function}}
 */
let backend = null;

/** 已注册的通道（与后端无关，便于在切换后端前后做契约校验） */
const registry = { handle: new Set(), on: new Set() };

/**
 * 切换传输后端
 * @param {"electron"|"jsonrpc"} name
 * @param {object} [options] jsonrpc 后端的配置
 */
export function useTransport(name, options = {}) {
    if (name === "electron") {
        // 惰性加载，避免 sidecar 环境下触碰 ipcMain
        backend = require("./electron.js");
    } else if (name === "jsonrpc") {
        backend = require("./jsonrpc.js").create(options);
        // jsonrpc 后端是新实例，把已注册的通道重放一遍
        for (const [channel, fn] of registered.handle) backend.handle(channel, fn);
        for (const [channel, fn] of registered.on) backend.on(channel, fn);
    } else {
        throw new Error(`Unknown transport: ${name}`);
    }
    return backend;
}

/** 当前后端（测试用） */
export function currentTransport() {
    return backend;
}

/** 记录一份，供后端切换时重放 */
const registered = { handle: new Map(), on: new Map() };

/**
 * 注册一个请求-响应通道
 * @param {string} channel 通道名（如 "python.uv.install"）
 * @param {Function} fn handler，签名与原先 ipcMain.handle 一致：(evt, ...args)
 * @returns {Function} 原样返回 fn，保持 `handlers.xxx = handle(...)` 的赋值形式不变
 */
export function handle(channel, fn) {
    registered.handle.set(channel, fn);
    registry.handle.add(channel);
    backend?.handle(channel, fn);
    return fn;
}

/**
 * 注册一个同步通道（仅 Electron 支持；jsonrpc 下按异步处理）
 * @param {string} channel
 * @param {Function} fn
 * @returns {Function}
 */
export function on(channel, fn) {
    registered.on.set(channel, fn);
    registry.on.add(channel);
    backend?.on(channel, fn);
    return fn;
}

/**
 * 向后端 -> 前端广播事件
 * @param {string} tag 支持 ":" 层级（a:b:c 会同时发 a:b:c、a:b、a）
 * @param {*} message
 */
export function sendEvent(tag, message) {
    backend?.sendEvent(tag, message);
}

/**
 * 列出已注册通道（用于契约校验）
 */
export function listHandlers() {
    return {
        handle: [...registry.handle],
        on: [...registry.on],
    };
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

/**
 * 窗口查找表。Electron 主进程启动后调用 `setWindowRegistry(windows)` 注入；
 * sidecar 下保持 null（没有窗口对象，对话框由 Rust 侧处理）。
 *
 * 这样 files.js 既能拿到父窗口，又不用静态 import frames.js
 * （frames.js → svelte.js → Electron GUI 依赖链，sidecar 下会崩）。
 */
let windowRegistry = null;

export function setWindowRegistry(windows) {
    windowRegistry = windows;
}

/**
 * @param {string|number} id
 * @returns {object|undefined} Electron BrowserWindow 或 undefined
 */
export function getWindowById(id) {
    if (!windowRegistry) return undefined;
    return windowRegistry[id];
}
