/**
 * Tauri preload 桥 —— 在页面模块图求值前把 window.electron / git / python 装上。
 *
 * 为什么必须是普通 <script> 而不是 import：
 *   src/lib/globals.svelte.js 第一行就是 `export var electron = $state(window.electron)`，
 *   模块求值时立刻读取 window.electron。等到 layout 的 onMount 或顶层 await 再装，
 *   拿到的是 undefined，$state 之后不会再变，于是所有 {#if electron} 分支
 *   都不渲染 —— 表现为纯白屏且没有报错。
 *
 * 所以这个文件由 app.html 用普通 <script src> 引入（加载在模块图之前），
 * 并且它必须在被引入时就完成赋值，不能是 module 形式的 export。
 *
 * 与 Electron 版的差别只有"怎么发消息"：
 *   Electron:  ipcRenderer.invoke(channel, ...args)
 *   Tauri:     invoke("node_call", { channel, params })
 * 两者都返回 Promise<result>，result 形状一致，业务代码不用改。
 */
(function () {
  "use strict";

  if (window.electron) return; // 已被注入（例如 Electron 外壳），不覆盖

  var internals = window.__TAURI_INTERNALS__;
  if (!internals || typeof internals.invoke !== "function") {
    // 不是 Tauri 环境（例如 vite dev 在浏览器里直接打开）。
    // 不抛错 —— 抛错会让页面在开发时直接挂掉；留一个空实现让 UI 能渲染出来。
    console.warn("[preload-bridge] Tauri internals not found; using no-op backend");
    var noop = function () {
      return Promise.reject(new Error("Tauri backend unavailable"));
    };
    window.electron = { invoke: noop, listen: function () { return Promise.resolve(function () {}); } };
    window.git = { invoke: noop };
    window.python = { invoke: noop };
    return;
  }

  function invoke(cmd, args) {
    return internals.invoke(cmd, args);
  }

  /** sidecar 通道调用：channel 如 "git.server" */
  function nodeCall(channel, params) {
    return invoke("node_call", { channel: channel, params: params || [] });
  }

  /** 订阅事件 tag；返回 Promise<unlisten> */
  function listen(tag, cb) {
    return invoke("plugin:event|listen", {
      event: tag,
      target: { kind: "Any" },
      handler: internals.transformCallback(function (e) {
        cb(e && e.payload !== undefined ? e.payload : e);
      }),
    }).then(function (id) {
      return function () {
        return invoke("plugin:event|unlisten", { event: tag, eventId: id });
      };
    });
  }

  var api = { invoke: nodeCall, listen: listen };
  window.electron = api;
  window.git = { invoke: nodeCall, listen: listen };
  window.python = { invoke: nodeCall, listen: listen };
})();
