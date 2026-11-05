/**
 * Tauri 版 preload —— 提供 `window.electron / window.git / window.python`。
 *
 * 与 Electron 版（preload.js）的差别只有一件事：**怎么发消息**。
 *   Electron:  ipcRenderer.invoke(channel, ...args)
 *   Tauri:     invoke("node_call", { channel, params })
 *
 * 两者返回的都是 Promise<result>，而且 result 的形状完全一致，
 * 所以前端所有代码都不用改 —— 这正是 preload 这层存在的意义。
 *
 * 注意参数命名：invoke 的 params 会被 tauri-plugin-fs 等按 camelCase 处理，
 * channel 必须写成 { channel, params } 两个字段，Rust 侧 node_call(channel, params)。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * 调一个 sidecar 通道。
 * @param {string} channel 形如 "files.list" / "python.uv.list"
 * @param {any[]} params 与 Electron 版 invoke 一致的数组参数
 */
export function nodeCall(channel, params = []) {
  return invoke("node_call", { channel, params });
}

/** sidecar 是否就绪（可用于显示"正在启动"状态） */
export function nodeReady() {
  return invoke("node_ready");
}

/** 订阅一个事件 tag，返回 unlisten */
export function nodeListen(tag, cb) {
  return listen(tag, (e) => cb(e.payload));
}

/**
 * 把一组 API 对象挂到 window 上。
 * 在 Tauri 下 main window 创建时调用一次即可。
 */
export async function installGlobals() {
  // 事件监听器：前端代码可以直接 window.electron.listen("tag", cb)
  const listener = (tag, cb) => nodeListen(tag, cb);

  window.electron = {
    invoke: nodeCall,
    listen: listener,
    ready: nodeReady,
  };
  window.git = { invoke: nodeCall, listen: listener };
  window.python = { invoke: nodeCall, listen: listener };
}
