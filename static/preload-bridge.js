/**
 * Tauri preload 桥 —— 在页面模块图求值前把 window.electron / git / python 装上。
 *
 * ## 为什么是普通 <script> 而不是 import
 *
 * src/lib/globals.svelte.js 第二行就是
 *     export var electron = $state(window.electron)
 * 模块求值时立刻读取。等 layout 的 onMount 或顶层 await 再装就晚了，
 * $state(undefined) 之后不再变，所有 {#if electron} 分支不渲染 —— 白屏且无报错。
 * 所以由 app.html 用 <script src> 引入，加载早于模块图。
 *
 * ## 为什么必须有完整的 API 结构，而不只是一个 invoke
 *
 * 前端调用的是 electron.windows.new()、electron.platform()、git.server() 这种
 * 结构化 API，而不是 electron.invoke("...")。早先这里只挂了裸 invoke，
 * 于是每个调用点都 TypeError: xxx is not a function，页面渲染一半就停。
 *
 * 所以这里照着 electron/src/preload.js 的形状逐个方法搭出来。
 * 保持形状一致比抽公共函数更重要 —— 它是契约。
 */
(function () {
  "use strict";

  if (window.electron) return; // 已被注入（Electron 外壳），不覆盖

  var internals = window.__TAURI_INTERNALS__;

  // 不是 Tauri 环境（vite dev 在浏览器里直接打开）：
  // 不抛错，挂空实现让 UI 能渲染出来，否则开发时整页挂掉。
  if (!internals || typeof internals.invoke !== "function") {
    console.warn("[preload-bridge] Tauri internals not found; using no-op backend");
    var reject = function () {
      return Promise.reject(new Error("Tauri backend unavailable"));
    };
    var noopApi = { invoke: reject, listen: function () { return Promise.resolve(function () {}); } };
    window.electron = noopApi;
    window.git = noopApi;
    window.python = noopApi;
    return;
  }

  /** 调一个 Rust 命令 */
  function call(cmd, args) {
    return internals.invoke(cmd, args || {});
  }

  /**
   * 调一个 sidecar 通道。
   *
   * Electron 的 ipcRenderer.invoke(channel, ...args) 是**位置参数**，
   * sidecar 的 handler 按 (evt, ...args) 接收。所以这里必须把 ...args
   * 原样塞进 params 数组，不能改成一个对象 —— 否则所有 handler 取参全错。
   */
  function nodeCall(channel) {
    var args = Array.prototype.slice.call(arguments, 1);
    return call("node_call", { channel: channel, params: args });
  }

  /** 订阅一个事件 tag，返回 Promise<unlisten> */
  function listenTag(tag, lsnr) {
    return call("plugin:event|listen", {
      event: tag,
      target: { kind: "Any" },
      handler: internals.transformCallback(function (e) {
        var payload = e && e.payload !== undefined ? e.payload : e;
        lsnr(payload);
      })
    }).then(function (id) {
      return function () {
        return call("plugin:event|unlisten", { event: tag, eventId: id });
      };
    });
  }

  // Electron 的 ipcRenderer.on(tag, (evt, ...args) => {}) 把 evt 作为第一个参数；
  // 前端代码里普遍写成 (evt, data) => ...，所以这里补一个假的 evt。
  function onTag(tag, lsnr) {
    listenTag(tag, function (payload) {
      lsnr({}, payload);
    });
  }

  // ---- electron ----

  var electronApi = {
    // 平台与版本：Electron preload 里这些是同步值/方法
    platform: function () {
      // 前端有 `await electron.platform()`，所以这里返回 Promise 也能用
      return call("platform");
    },
    version: function () {
      return call("app_version");
    },
    windows: {
      new: function (target) { return call("windows_new", { target: target }); },
      get: function (target) { return call("windows_get", { target: target }); },
      send: function (id, tag, data) { return call("windows_send", { id: id, tag: tag, data: data }); },
      emit: function (tag, data) { return call("windows_emit", { tag: tag, data: data }); },
      listen: onTag,
      focus: function (id) { return call("windows_focus", { id: id }); },
      devtools: function (id) { return call("windows_devtools", { id: id }); },
      close: function (id) { return call("windows_close", { id: id }); },
      hideMenu: function () { return call("windows_hide_menu"); },
      setMenu: function (template) { return call("windows_set_menu", { template: template }); }
    },
    paths: {
      // webUtils.getPathForFile 在 Tauri 里拿不到 —— 用 webkitRelativePath/name 兜底
      getPathForFile: function (file) {
        return (file && (file.path || file.webkitRelativePath)) || (file && file.name) || "";
      },
      documents: function () { return call("paths_documents"); },
      user: function () { return call("paths_user"); },
      devices: function () { return call("paths_devices"); },
      prefs: function () { return call("paths_prefs"); },
      pavlovia: {
        dir: function () { return call("paths_pavlovia"); },
        users: function () { return call("paths_pavlovia_users"); },
        projects: function () { return call("paths_pavlovia_projects"); }
      }
    },
    files: {
      load: function (f) { return nodeCall("electron.files.load", f); },
      save: function (f, c) { return nodeCall("electron.files.save", f, c); },
      exists: function (f) { return nodeCall("electron.files.exists", f); },
      stat: function (f) { return nodeCall("electron.files.stat", f); },
      mkdir: function (f) { return nodeCall("electron.files.mkdir", f); },
      openDialog: function (o) { return nodeCall("electron.files.openDialog", o); },
      saveDialog: function (o) { return nodeCall("electron.files.saveDialog", o); },
      scandir: function (d) { return nodeCall("electron.files.scandir", d); },
      showItemInFolder: function (f) { return nodeCall("electron.files.showItemInFolder", f); },
      openPath: function (p) { return nodeCall("electron.files.openPath", p); },
      openExternal: function (u) { return nodeCall("electron.files.openExternal", u); },
      downloadFolder: function () { return nodeCall("electron.files.downloadFolder"); }
    },
    clipboard: {
      get: function () { return call("clipboard_get"); },
      set: function (v) { return call("clipboard_set", { value: v }); }
    },
    state: {
      updateFrame: function (frame) { return nodeCall("electron.state.updateFrame", frame); },
      setProfiles: function (p) { return nodeCall("electron.state.setProfiles", p); },
      getProfiles: function () { return nodeCall("electron.state.getProfiles"); }
    },
    system: {
      requestKeyboardAccess: function () { return call("system_request_keyboard_access"); },
      hasKeyboardAccess: function () { return call("system_has_keyboard_access"); }
    }
  };

  // ---- python ----

  var pythonApi = {
    liaison: {
      start: function () { return nodeCall("python.liaison.start"); },
      stop: function () { return nodeCall("python.liaison.stop"); },
      send: function (m) { return nodeCall("python.liaison.send", m); },
      listen: function (l) { onTag("python.liaison.response", l); },
      started: function () { return nodeCall("python.liaison.started"); },
      ready: function () { return nodeCall("python.liaison.ready"); }
    },
    venv: {
      setup: function (v) { return nodeCall("python.venv.setup", v); },
      executable: function (v) { return nodeCall("python.venv.executable", v); },
      installPackage: function (v, n, ver) { return nodeCall("python.venv.installPackage", v, n, ver); },
      uninstallPackage: function (v, n) { return nodeCall("python.venv.uninstallPackage", v, n); },
      getPackages: function (v) { return nodeCall("python.venv.getPackages", v); },
      getPackageDetails: function (v, n) { return nodeCall("python.venv.getPackageDetails", v, n); },
      hasGIL: function (v) { return nodeCall("python.venv.hasGIL", v); }
    },
    uv: {
      folder: function () { return nodeCall("python.uv.folder"); },
      executable: function () { return nodeCall("python.uv.executable"); },
      systemInfo: function () { return nodeCall("python.uv.systemInfo"); },
      findDirectory: function (o) { return nodeCall("python.uv.findDirectory", o); },
      setDirectory: function (o) { return nodeCall("python.uv.setDirectory", o); },
      exists: function () { return nodeCall("python.uv.exists"); },
      install: function () { return nodeCall("python.uv.install"); },
      ensure: function () { return nodeCall("python.uv.ensure"); }
    },
    output: {
      stdout: { send: function (m) { return call("python_stdout_send", { message: m }); }, listen: function (l) { onTag("stdout", l); } },
      stderr: { send: function (m) { return call("python_stderr_send", { message: m }); }, listen: function (l) { onTag("stderr", l); } },
      app: { stderr: { listen: function (l) { onTag("stderr:app", l); } } }
    },
    shell: {
      list: function (v) { return nodeCall("python.shell.list", v); },
      send: function (v, id, m) { return nodeCall("python.shell.send", v, id, m); },
      open: function (v) { return nodeCall("python.shell.open", v); },
      close: function (v, id) { return nodeCall("python.shell.close", v, id); }
    },
    scripts: {
      run: function (v, f) {
        var args = Array.prototype.slice.call(arguments, 2);
        return nodeCall.apply(null, ["python.scripts.run", v, f].concat(args));
      },
      wait: function (v, id) { return nodeCall("python.scripts.wait", v, id); },
      stop: function (v, id) { return nodeCall("python.scripts.stop", v, id); }
    },
    psychojs: {
      run: function (cwd, params) { return nodeCall("python.psychojs.run", cwd, params || {}); },
      stop: function (addr) { return nodeCall("python.psychojs.stop", addr); }
    }
  };

  // ---- git ----

  var gitApi = {
    listen: function (l) { onTag("git", l); },
    output: function (m) { return nodeCall("git.output", m); },
    server: function () { return nodeCall("git.server"); },
    login: function () { return nodeCall("git.login"); },
    loadUsers: function () { return nodeCall("git.loadUsers"); },
    clearUsers: function () { return nodeCall("git.clearUsers"); },
    listUsers: function () { return nodeCall("git.listUsers"); },
    listGroups: function (u) { return nodeCall("git.listGroups", u); },
    listSurveys: function (u) { return nodeCall("git.listSurveys", u); },
    getUserInfo: function (u) { return nodeCall("git.getUserInfo", u); },
    authenticateURL: function (u) { return nodeCall("git.authenticateURL", u); },
    getRemote: function (p) { return nodeCall("git.getRemote", p); },
    getProjectInfo: function (p) { return nodeCall("git.getProjectInfo", p); },
    clone: function (url, dir) { return nodeCall("git.clone", url, dir); },
    fork: function (p) { return nodeCall("git.fork", p); },
    listProjectForks: function (p) { return nodeCall("git.listProjectForks", p); },
    pull: function (dir) { return nodeCall("git.pull", dir); },
    stage: function (dir, files) { return nodeCall("git.stage", dir, files); },
    commit: function (dir, msg) { return nodeCall("git.commit", dir, msg); },
    push: function (dir) { return nodeCall("git.push", dir); },
    newProject: function (o) { return nodeCall("git.newProject", o); },
    loadProjects: function () { return nodeCall("git.loadProjects"); },
    linkProject: function (p) { return nodeCall("git.linkProject", p); }
  };

  window.electron = electronApi;
  window.git = gitApi;
  window.python = pythonApi;

})();
