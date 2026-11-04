# GnosisStudio IPC 契约

> **本文件是迁移的唯一接口规范。** 所有 Tauri 侧实现必须逐字节保真此契约：通道名、参数顺序、返回结构、事件 tag。
>
> 生成方式：从 `electron/src/**` 机械提取（`ipcMain.handle` / `ipcMain.on` / `webContents.send`），非手抄。
> 源仓库：`generated/GnosisStudio@feat/tauri-migration`
> 生成日期：2026-11-04

---

## 1. 总览

| 类别 | 数量 |
|---|---|
| `ipcMain.handle`（请求-响应，异步） | **91** |
| `ipcMain.on`（请求-响应，**同步**） | **1** |
| 事件 tag（后端 → 前端） | 见 §4 |
| **合计通道** | **92** |

### 1.1 分组

| 前缀 | 通道数 | 归属里程碑 |
|---|---|---|
| `electron.*` | 37 | M3 |
| `python.*` | 33 | M4 / M7 |
| `git.*` | 22 | M6 |

---

## 2. 通道清单

#### `electron.*` — 37 个通道

| 通道 | 类型 | 实现位置 |
|---|---|---|
| `electron.clipboard.get` | handle | `index.cjs:266` |
| `electron.clipboard.set` | handle | `index.cjs:267` |
| `electron.files.downloadFolder` | handle | `files.js:260` |
| `electron.files.exists` | handle | `files.js:251` |
| `electron.files.load` | handle | `files.js:249` |
| `electron.files.mkdir` | handle | `files.js:253` |
| `electron.files.openDialog` | handle | `files.js:254` |
| `electron.files.openExternal` | handle | `files.js:259` |
| `electron.files.openPath` | handle | `files.js:258` |
| `electron.files.save` | handle | `files.js:250` |
| `electron.files.saveDialog` | handle | `files.js:255` |
| `electron.files.scandir` | handle | `files.js:256` |
| `electron.files.showItemInFolder` | handle | `files.js:257` |
| `electron.files.stat` | handle | `files.js:252` |
| `electron.paths.devices` | handle | `index.cjs:256` |
| `electron.paths.documents` | handle | `index.cjs:254` |
| `electron.paths.pavlovia` | handle | `index.cjs:259` |
| `electron.paths.pavlovia.projects` | handle | `index.cjs:261` |
| `electron.paths.pavlovia.users` | handle | `index.cjs:260` |
| `electron.paths.prefs` | handle | `index.cjs:257` |
| `electron.paths.user` | handle | `index.cjs:255` |
| `electron.platform` | handle | `index.cjs:274` |
| `electron.quit` | handle | `index.cjs:275` |
| `electron.state.getProfiles` | **on（同步）** | `tools/state.js:112` |
| `electron.state.setProfiles` | handle | `tools/state.js:110` |
| `electron.state.updateFrame` | handle | `tools/state.js:109` |
| `electron.system.hasKeyboardAccess` | handle | `index.cjs:271` |
| `electron.system.requestKeyboardAccess` | handle | `index.cjs:270` |
| `electron.version` | handle | `index.cjs:273` |
| `electron.windows.close` | handle | `index.cjs:249` |
| `electron.windows.devtools` | handle | `index.cjs:248` |
| `electron.windows.focus` | handle | `index.cjs:247` |
| `electron.windows.get` | handle | `index.cjs:241` |
| `electron.windows.hideMenu` | handle | `index.cjs:250` |
| `electron.windows.new` | handle | `index.cjs:240` |
| `electron.windows.send` | handle | `index.cjs:242` |
| `electron.windows.setMenu` | handle | `index.cjs:251` |

#### `python.*` — 33 个通道

| 通道 | 类型 | 实现位置 |
|---|---|---|
| `python.liaison.ready` | handle | `python/index.js:28` |
| `python.liaison.send` | handle | `python/index.js:23` |
| `python.liaison.start` | handle | `python/index.js:21` |
| `python.liaison.started` | handle | `python/index.js:26` |
| `python.liaison.stop` | handle | `python/index.js:22` |
| `python.psychojs.run` | handle | `python/index.js:70` |
| `python.psychojs.stop` | handle | `python/index.js:71` |
| `python.scripts.run` | handle | `python/index.js:66` |
| `python.scripts.stop` | handle | `python/index.js:67` |
| `python.scripts.wait` | handle | `python/index.js:68` |
| `python.shell.close` | handle | `python/index.js:62` |
| `python.shell.list` | handle | `python/index.js:64` |
| `python.shell.open` | handle | `python/index.js:60` |
| `python.shell.send` | handle | `python/index.js:52` |
| `python.uv.executable` | handle | `python/index.js:36` |
| `python.uv.exists` | handle | `python/index.js:32` |
| `python.uv.findDirectory` | handle | `python/index.js:34` |
| `python.uv.findPython` | handle | `python/index.js:38` |
| `python.uv.folder` | handle | `python/index.js:31` |
| `python.uv.getEnvironments` | handle | `python/index.js:45` |
| `python.uv.install` | handle | `python/index.js:33` |
| `python.uv.makeExecutable` | handle | `python/index.js:46` |
| `python.uv.needsUpdate` | handle | `python/index.js:30` |
| `python.uv.resolvePackageVersion` | handle | `python/index.js:48` |
| `python.uv.setDirectory` | handle | `python/index.js:35` |
| `python.uv.systemInfo` | handle | `python/index.js:49` |
| `python.venv.executable` | handle | `python/index.js:41` |
| `python.venv.getPackageDetails` | handle | `python/index.js:44` |
| `python.venv.getPackages` | handle | `python/index.js:43` |
| `python.venv.hasGIL` | handle | `python/index.js:47` |
| `python.venv.installPackage` | handle | `python/index.js:40` |
| `python.venv.setup` | handle | `python/index.js:39` |
| `python.venv.uninstallPackage` | handle | `python/index.js:41` |

#### `git.*` — 22 个通道

| 通道 | 类型 | 实现位置 |
|---|---|---|
| `git.authenticateURL` | handle | `tools/git.js:971` |
| `git.clearUsers` | handle | `tools/git.js:961` |
| `git.clone` | handle | `tools/git.js:960` |
| `git.commit` | handle | `tools/git.js:964` |
| `git.fork` | handle | `tools/git.js:962` |
| `git.getProjectInfo` | handle | `tools/git.js:968` |
| `git.getRemote` | handle | `tools/git.js:954` |
| `git.getUserInfo` | handle | `tools/git.js:959` |
| `git.linkProject` | handle | `tools/git.js:963` |
| `git.listGroups` | handle | `tools/git.js:967` |
| `git.listProjectForks` | handle | `tools/git.js:969` |
| `git.listSurveys` | handle | `tools/git.js:970` |
| `git.listUsers` | handle | `tools/git.js:957` |
| `git.loadProjects` | handle | `tools/git.js:958` |
| `git.loadUsers` | handle | `tools/git.js:956` |
| `git.login` | handle | `tools/git.js:955` |
| `git.newProject` | handle | `tools/git.js:966` |
| `git.output` | handle | `tools/git.js:953` |
| `git.pull` | handle | `tools/git.js:965` |
| `git.push` | handle | `tools/git.js:972` |
| `git.server` | handle | `tools/git.js:973` |
| `git.stage` | handle | `tools/git.js:974` |

---

## 3. 关键语义

### 3.1 错误语义

Electron：handler 抛异常 → 前端 `reject(err)`，**err 通常是 Error 对象**。

Tauri：`Result::Err(String)` → 前端 reject 得到**裸字符串**。

**M0 实测结论**：`src/lib/python/functions.svelte.js:handleError` 的 `"error" in err` 会在字符串上抛 TypeError。已加 `typeof` 守卫修复。
其余 17 处 `catch(` 均不访问 `err.error`，无需改动。

**M2 约束**：传输层**不做**错误对象包装，保持 Tauri 裸字符串语义。若未来需要恢复 `{error: ...}` 结构，只需改 `handleError` 一处。

### 3.2 同步通道（唯一）

`electron.state.getProfiles` 是 `ipcMain.on` + `evt.returnValue` 的**同步**调用，前端：

```js
getProfiles: () => ipcRenderer.sendSync("electron.state.getProfiles")
```

**决策 D6**：改为异步。Tauri 无 `sendSync` 等价物。
影响面：前端仅 `preload.js:145` 一处调用点，改为 `await`。启动时序随之调整。

### 3.3 事件广播模型（重要，易漏）

`electron/src/python/utils.js:88` 的 `output(tag, message)` 是**所有事件的唯一出口**，含一个关键行为：

```js
// 层级广播：tag 为 "uv:psychopy-cedrus" 时，会依次发到
//   "uv:psychopy-cedrus" → "uv:psychopy" → "uv"
while (tag?.includes?.(":")) {
    tag = tag.substring(0, tag.lastIndexOf(":"))
    channels.push(tag)
}
// 广播到所有窗口
BrowserWindow.getAllWindows().forEach(win => win.webContents.send(channel, message))
```

**Tauri 迁移必须保留两点**：
1. **层级广播**：`a:b:c` → 同时发 `a:b:c`、`a:b`、`a`
2. **全窗口广播**：不只是发起调用的窗口

### 3.4 事件 tag 全集

| tag | 来源 | 载荷 | 前端订阅 |
|---|---|---|---|
| `stdout` | `python/script.js`、`utils.js` output | string | ✅ |
| `stderr` | 同上 | string | ✅ |
| `uv` | `utils.js` output（含 `uv:*` 子通道） | string | ✅ |
| `git` | `tools/git.js:947` | string | — |
| `import sys` | `python/shell.js:55`（初始握手） | string | — |
| `showTips` | `index.cjs:195` | boolean | ✅ `TipsDialog.svelte:19` |
| `alert` | `liaison:${tag}` 转发而来 | object | ✅ `runner/+page.svelte:45` |
| `error` | 同上 | object | ✅ `PythonErrors.svelte:27` |
| `fileOpen` | `views.svelte.js:37` → `electron.windows.send` | file 对象 | ✅ 三个 route |
| `menu:${id}` | `frames.js:127` | 事件详情 | ✅ `Item.svelte:68` |
| `liaison:${tag}` | `liaison.js:293` `output(\`liaison:${tag}\`)` | Python 回传 msg | ✅ 前缀订阅 |

**动态前缀共 2 个**：`liaison:${tag}`、`menu:${id}`。迁移时不能写死 tag 表。

**注意 `alert` / `error` 的来源**：不是独立 tag，而是 Python 侧 liaison 回传的 `msg.tag`（`liaison.js:119` `this.output(msg.tag, msg)`）→ `output("liaison:" + tag)` → 层级广播同时发 `liaison:alert`、`liaison`。前端 `.listen("alert")` 之所以能收到，是因为 liaison 的 tag 本身叫 `alert`（无冒号），走的是 `output("liaison:alert")` 还是 `output(tag)` 需以运行时为准——**M4 移植时用真实 Python 会话验证一次，不要凭读码推断**。

### 3.5 参数顺序约定

`ipcMain.handle(channel, (evt, a, b, c) => ...)` 中 `evt` 是 Electron 注入的，**不属于业务参数**。
Tauri 侧 `invoke(cmd, {a, b, c})` 用命名参数，需按位置映射回原顺序。

例：
```js
// Electron
ipcMain.handle("python.venv.installPackage", (evt, venv, name, version) => ...)
// Tauri
invoke("venv_install_package", { venv, name, version })
```

### 3.6 来源窗口（`evt.sender.id`）—— 易漏且会静默失效

业务代码有 **9 处**依赖 `evt.sender.id` 判断"哪个窗口发起的调用"：

| 通道 | 用途 |
|---|---|
| `electron.files.openDialog` | `windows[evt.sender.id]` 作为对话框父窗口 |
| `electron.files.saveDialog` | 同上 |
| `electron.state.updateFrame` | 更新对应窗口的状态 |
| `electron.windows.focus` | `windows[id \|\| evt.sender.id]` |
| `electron.windows.devtools` | 同上 |
| `electron.windows.close` | 同上 |
| `electron.windows.hideMenu` | `windows[evt.sender.id]` |
| `electron.windows.setMenu` | 同上 |

**迁移约束**：transport 必须保证第一个参数上有 `sender.id`。
- Electron：数字 webContents id
- JSON-RPC：前端 `invoke` 时显式带上的窗口 label 字符串

业务代码统一用 `senderId(evt)` 取值（`transport/index.js`），**不要直接摸 `evt.sender.id`**，否则切换后端时会静默拿到 undefined。

> 注意 `evt.sender.id` 在 Electron 是数字、在 Tauri 是字符串，`windows[id]` 这类字典的 key 类型随之改变。M5 移植窗口管理时需统一改为 label。

---

## 4. 传输层实现（M2）

已落地 `electron/src/transport/`，把 handler 注册与 Electron 解耦：

| 文件 | 作用 |
|---|---|
| `transport/index.js` | 门面：`handle` / `on` / `sendEvent` / `useTransport` / `senderId` |
| `transport/electron.js` | 默认后端，行为与直接调 `ipcMain` 完全一致 |
| `transport/jsonrpc.js` | sidecar 后端，JSON-RPC 2.0 over NDJSON |

**改动范围**：5 个文件、97 处引用，仅替换注册函数，**业务逻辑零改动**：
`files.js`、`tools/state.js`、`tools/git.js`、`python/index.js`、`index.cjs`

**已验证**（`/tmp/rpc_test.mjs`，真实往返）：
- 正常返回 → `result`
- handler 抛错 → `error.message` 为**裸字符串**
- `sender` 透传到 `evt.sender.id`
- 未注册方法 → `-32601`
- 层级广播：`uv:psychopy-cedrus` 同时发出 `uv:psychopy-cedrus` 与 `uv`

**注意**：`index.cjs` 是 CommonJS，通过 `require()` 引入 ESM 的 transport。已实测 Node 22 的 `require(ESM)` 与 `import` **共享同一模块实例**，因此后端切换是全局生效的，不存在双份 registry。

---

## 5. M3：进程分层与通道归属

### 5.1 三层分工

| 层 | 职责 | 通道数 |
|---|---|---|
| **Rust**（Tauri 主进程） | 窗口、菜单、对话框、剪贴板、路径、单实例、打包、协议 | **22** |
| **Node sidecar**（`electron/src/sidecar.mjs`） | files、state、git、python 全部业务逻辑 | **70** |
| **Python**（由 sidecar 拉起） | psychopy 环境、实验编译、liaison 长连接 | — |

合计 92，与 §1 契约完全对齐。

### 5.2 通道归属明细

**Rust 提供（22 个）** —— 实现于 `demo/GnosisStudio/src-tauri/src/system.rs`

```
electron.paths.*        × 7   documents / user / devices / prefs / pavlovia{,.users,.projects}
electron.windows.*      × 8   new / get / send / focus / close / devtools / hideMenu / setMenu
electron.clipboard.*    × 2   get / set
electron.system.*       × 2   hasKeyboardAccess / requestKeyboardAccess
electron.platform       × 1
electron.version        × 1
electron.quit           × 1
```

**sidecar 提供（70 个）**

```
electron.files.*        × 12
electron.state.*        × 3   （含唯一的同步通道 getProfiles）
git.*                   × 22
python.*                × 33
```

### 5.3 sidecar 如何复用 Electron 业务代码

`sidecar.mjs` 用 `module.registerHooks` 把模块名 `"electron"` 重定向到 `electron-shim.js`，
因此 **14 个业务文件一行不改**就能在 sidecar 下运行。

`electron-shim.js` 提供：
- `app.getPath("appData"/"documents")` —— 平台目录，**必须与 Electron 布局一致**，否则用户已有 venv/设置会"消失"
- `app.on/whenReady/quit/isPackaged`
- `safeStorage` —— 过渡实现（本地密钥 + XOR），M6 换 keychain（D7 已定不兼容旧密文）
- `BrowserWindow.getAllWindows` —— 返回空数组，广播由 transport 接管

`native-shim.js` 提供 `dialog`/`shell`：
- `shell.*` 用 Node `exec` 调平台命令，可直接工作
- `dialog.*Sync` 需要宿主 GUI，**故意抛错**而不是静默返回 null（后者会让用户以为"取消了"）
- 通过 `setNativeBridge()` 预留 Rust 注入点，M8 接线

### 5.4 M3 完成时发现并修掉的耦合

| 问题 | 修法 |
|---|---|
| `files.js` 静态 `import { windows } from "./frames.js"`，会把 frames → svelte → Electron GUI 整条链拖进 sidecar | 改为 `transport.getWindowById(senderId(evt))`，Electron 启动时 `setWindowRegistry(windows)` 注入 |
| `transport/index.js` 顶层 import 两个后端，sidecar 加载即触 `ipcMain` | 后端改惰性 `require`；`handle()` 在注册表里缓存并支持切换后重放 |

### 5.5 行为差异（已知且有意）

| 项 | Electron | Tauri/M3 | 说明 |
|---|---|---|---|
| `electron.platform` | `"darwin"/"win32"/"linux"` | 同左（Rust 侧映射 `macos`→`darwin`） | 前端判断依赖此值，必须映射 |
| `electron.clipboard.*` | 模块级变量，**非系统剪贴板** | `Mutex<Option<Value>>` | 前端 `clipboard.js` 只当内存存储用 |
| `postinstall` | 无 | `pnpm install` 会看到 `dmg-builder` 可选依赖警告 | 无害 |

---

## 6. 前端调用点统计

| 前缀 | 调用点数 | 主要文件 |
|---|---|---|
| `python.liaison.send` | 39 | `src/lib/experiment/experiment.svelte.js`、`src/lib/python/` |
| `electron.files.*` | 37 | `src/lib/utils/callbacks.js`、`src/lib/dialogs/` |
| `electron.windows.*` | 20 | `src/routes/*/ribbon/` |
| `electron.paths.*` | 11 | 各 route |
| `electron.version` | 5 | `src/lib/utils/versions.js` |
| `electron.state.*` | 3 | `src/lib/` |
| `electron.quit` | 3 | ribbon menu |
| `electron.system.*` | 2 | 权限请求 |
| `electron.platform` | 2 | — |
| `electron.clipboard.*` | 2 | — |

---

## 7. 迁移检查清单

每个通道迁移时必须确认：

- [ ] 通道名映射（snake_case 转换规则统一）
- [ ] 参数顺序与命名
- [ ] 返回结构逐字段一致
- [ ] 错误路径行为一致（见 §3.1）
- [ ] 若涉及事件广播，保留层级广播 + 全窗口广播（见 §3.3）
- [ ] 前端调用点无需改动（除非是 D6 的同步→异步）
