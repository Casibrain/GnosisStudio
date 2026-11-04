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

**关键：`liaison` 的前缀由 preload 加，不由后端加。** `preload.js:68`：

```js
listen: (tag, lsnr) => ipcRenderer.on(`liaison:${tag}`, lsnr)
```

Python 侧 `liaison.js:119` 发的是 `output("liaison:" + msg.tag, msg)`，配合
层级广播，会同时发 `liaison:alert`、`liaison`。前端 `.listen("alert")` 收到的正是
`liaison:alert`。**这解释了为什么 alert 能工作**——读码时可验证，不必运行时确认。

> Tauri 侧注意：Tauri 的 `listen()` 没有 preload 自动加前缀这层。桥接实现时
> 前端若仍写 `.listen("alert")`，Rust 侧要么按约定映射到 `liaison:alert`，
> 要么在桥接层补齐这层前缀转换。**不要两边都加或都不加**。
>
> 其余 `listen` 均**不**加前缀：`python.output.stdout/stderr`、`python.uv`
> （`preload.js:97-111`）和 `git`（`:134`）直接监听裸 tag。

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

## 6. M4：Python 栈实测结论

以下全部来自**真实 Python 会话**（psychopy dev / Python 3.11，本机已有环境），不是读码推断。

### 6.1 走通的全链路

```
setDirectory("user") → getEnvironments → venv.executable("dev") → venv.getPackages("dev")
liaison.start → liaison.started → liaison.ready → liaison.send(echo) → liaison.stop
scripts.run("dev", "/tmp/gstest/hello.py") → scripts.wait → stdout/stderr 事件送达
```

实测输出：`venv.executable` = `~/Library/Application Support/psychopy4/.python/dev/bin/python3`，
`getPackages` 返回真实包表，`scripts.wait` 返回 `[0, null]`（退出码 0）。

### 6.2 致命缺陷：`output()` 绕过了 transport（已修）

**这是 M4 最有价值的发现。** `python/utils.js` 的 `output()` 是**所有**后端→前端事件的唯一出口，
但原实现直接调 `BrowserWindow.getAllWindows()`：

```js
// 修复前
for (let channel of channels) {
    BrowserWindow.getAllWindows().forEach(win => win.webContents.send(channel, message))
}
```

sidecar 里 `getAllWindows()` 返回 `[]`（`electron-shim.js`），导致
**stdout / stderr / uv / git 全部事件静默丢失**——脚本能跑完，但前端一片空白，
且**不报任何错**。`tools/git.js:947` 是第二个相同缺陷。

修复：两者都改为 `sendEvent(tag, message)`，层级广播交给 transport。

| 文件 | 修复 |
|---|---|
| `python/utils.js` `output()` | `BrowserWindow.getAllWindows()...` → `sendEvent(tag, message)` |
| `tools/git.js` `output()` | 同上 |

**教训**：迁移事件总线时，"注册"和"发送"是两条独立路径。
M2 只改了注册侧（`ipcMain.handle`），发送侧漏了。任何事件型迁移都要单独核对发送侧。

### 6.3 stdout 污染 JSON-RPC（已修）

业务代码 `logging.log()` 默认 `echo=true` 走 `console.log`，`execSync` 经
`input() → output() → logging.log` 会把 `UV\t>> "..."` 写进 stdout，
Rust 侧 `JSON.parse` 直接崩：`Unexpected token 'U', "UV\t>> "/Us"...`。

修复：`sidecar.mjs` 在加载业务代码**之前**把 `console.log/info/debug` 全部改道 stderr。
不要逐个改业务调用点——那要改几十处，且以后新增的 `console.log` 还会漏。

### 6.4 各通道参数形态（易错，实测记录）

| 通道 | 正确参数 | 错误写法 |
|---|---|---|
| `python.venv.*` | `["dev"]` —— **版本号字符串** | 传 `getEnvironments` 返回的对象 → `version.includes is not a function` |
| `python.scripts.run` | `["dev", "/path/hello.py"]` —— **位置参数** | 传 `[{venv,file}]` 对象 → `version.includes is not a function` |
| `python.liaison.start` | `["dev"]`，**非幂等** | 连续调两次 → 第二次 `Sent before connected.` |

### 6.5 liaison 事件语义（读码可确认，无需运行时）

- `liaison.send()` 的响应用 **promise 返回**（`pending` 数组），**不产生事件**
- 只有 Python 主动推送带 `tag` 的消息，才走 `output("liaison:"+tag)` → 广播
- 所以 echo 测试"事件数为 0"是**正确行为**，不是 bug

层级广播实测（`/tmp/py_hier.mjs`，三组全部通过）：

```
输入 uv:psychopy-cedrus → 发出 uv:psychopy-cedrus, uv
输入 liaison:alert      → 发出 liaison:alert, liaison
输入 menu:abc           → 发出 menu:abc, menu
无重复发送 ✓
```

### 6.6 未验证项（留到 M7）

- `python.psychojs.run/stop` —— 依赖 `express` + `BrowserWindow`（`psychojs.js:54`），
  要等 M7 用 Tauri protocol 替代 express 后再测
- `python.shell.*` —— 交互式 shell，本轮未测

---

## 7. 前端调用点统计

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

## 7. M5：窗口、菜单、状态持久化

### 7.1 同步通道改异步（D6）

`electron.state.getProfiles` 是**唯一**的同步通道（`ipcRenderer.sendSync` + `evt.returnValue`）。
Tauri 没有同步 invoke，改异步后影响**两个**前端调用点：

| 位置 | 原写法 | 新写法 |
|---|---|---|
| `src/lib/experiment/profiles.svelte.js:92` | `electron?.state.getProfiles()?.[key]` | `(await electron?.state.getProfiles())?.[key]`（已在 async 函数内，直接加 await） |
| `src/lib/experiment/profiles.svelte.js:117` | 模块顶层同步循环 | 顶层 `const cachedProfiles = await ...`（ESM 支持） |

第 2 处必须改：顶层 await 在 ESM/SvelteKit 下可用，且只读一个本地 JSON，
不阻塞渲染关键路径。**不要**为此引入 Rust 预注入机制——那比改两行代码复杂得多。

### 7.2 窗口 id：数字 → 字符串（重要）

Electron 用 `webContents.id`（数字，如 `2`）作为 frames 字典 key；
Tauri 用 label（字符串，如 `"builder"`）。

| 位置 | Electron | Tauri |
|---|---|---|
| `state.js` `updateFrame(evt.sender.id)` | 数字 | label |
| `frames.js` `windows[win.webContents.id]` | 数字 | label |
| `git.js` `oauthWindows` | 数字 | label |

**实测后果**：直接迁移会 `Cannot convert undefined or null to object`
（`Object.assign(undefined, details)` 抛错），窗口状态**全部丢失**且不报明显错误。

**修法**：`updateFrame` 在 key 不存在时自动 `newFrame(id, id)`。
因为窗口由 Rust 管理，业务侧收不到开窗通知，只会在第一次 `updateFrame` 时见到这个 key。

### 7.3 退出落盘

Electron 版依赖 `app.on("before-quit") → saveState()`（`index.cjs:233`）。
sidecar 没有这个事件，必须自己在退出前落盘，否则**每次重启都丢窗口布局**。

`sidecar.mjs` 的 `shutdown()` 统一处理：`SIGTERM`/`SIGINT`/stdin 结束/uncaughtException 都先 `saveState()`。
用 `exiting` 标志防重入。

### 7.4 菜单系统

| 项 | Electron | Tauri |
|---|---|---|
| 回调机制 | `click: "<id>"` → `win.webContents.send("menu:<id>", true)` | `MenuItem::with_id(id, ...)` → `on_menu_event` → emit `menu:<id>` |
| 菜单构建 | `Menu.buildFromTemplate` | `tauri::menu::{Menu, Submenu, MenuItem, PredefinedMenuItem}` |
| macOS | `Menu.setApplicationMenu` | **`app.set_menu`**（`window.set_menu` 在 macOS 明确 Unsupported） |
| 其他平台 | `win.setMenu` | `win.set_menu` |

前端 `Item.svelte` 的 `getProfile()` 字段基本通用：`label`/`enabled`/`click`/`submenu`/`role`。
`role` 需映射到 `PredefinedMenuItem`（copy/cut/paste/quit/undo/redo/selectAll/minimize/close/fullscreen）。

### 7.5 capabilities 授权（M5 埋下的坑）

原 `capabilities/default.json` 写死 `windows: ["main"]`。
`windows_new` 动态创建的窗口（label 为 `builder`/`runner`）**拿不到任何权限**，
所有 invoke 会被拒且报错在前端控制台，不易发现。

**修法**：`"windows": ["*"]`。

### 7.6 启动自检

`GNOSTIS_SELFCHECK=1` 环境变量触发 `setup()` 里打印所有路径，便于与 Electron 版逐项比对：

```
[selfcheck] platform        = darwin
[selfcheck] paths_user      = /Users/huangdp/Library/Application Support/psychopy4
...
```

已实测与 Electron 布局一致。

---

## 8. M6：Git / Pavlovia

### 8.1 safeStorage：必须兼容旧密文（D7 修正）

M3 时的 D7 写的是"不兼容旧密文，用 XOR + 本地 key 文件占位"。**这个判断是错的**，M6 实测推翻：

```
~/Library/Application Support/psychopy4/pavlovia/users.json
hex: 7631 30 f5c06be9a04336ce30cdf86b89 65e0bc
     └"v10"┘ └──────── 16 字节 IV ────────┘ └ 0 字节 ┘
```

这是 Electron safeStorage 真实写下的 **Chromium OSCrypt** 密文，里面是用户的 Pavlovia 登录态。
换 key 就等于让所有用户的登录失效，且不会有任何提示。

**正确实现**（`electron-shim.js`）：

| 项 | 值 |
|---|---|
| 格式 | `"v10"` (3B) + IV (16B) + AES-128-CBC 密文 |
| 密钥 | macOS Keychain 条目 `"Electron Safe Storage"`，值是 **base64 的 16 字节** |
| 读取 | `security find-generic-password -s "Electron Safe Storage" -w` |
| 返回类型 | `encryptString` 返回 **Buffer**（不是 base64 字符串） |

实测四项全通过：

```
加密后头 v10 / 长度 99 / 往返 PASS
旧明文兼容 PASS（无 v10 头则原样返回 utf8）
空明文（只有头+IV）返回 ""
解密 Electron 真实写下的 users.json → ""（空用户表）✓
```

**注意返回类型**：`git.js` 把 `encryptString` 的结果直接 `fs.writeFileSync`，
把 `fs.readFileSync` 的 Buffer 直接喂 `decryptString`。返回 base64 字符串会让
每次读写都多一层 base64，且和 Electron 写的文件不兼容。

Windows 未移植：Electron 用 DPAPI 保护一个 32 字节密钥存在 `Local State` 的
`os_crypt.encrypted_key`。这里**明确抛错**而不是降级——
降级会"能加密但解不开旧数据"，比直接失败更难排查。

### 8.2 OAuth 登录窗口必须放 Rust

`git.js` 的 `login()` 需要三件 Node 做不到的事：

1. `new BrowserWindow(...)` —— 真实窗口
2. `win.webContents.session.clearStorageData(...)` —— 清 cookie 强制重新登录
3. `win.webContents.on("did-navigate", ...)` —— 监听回调 URL 拿 `?code=`

已实现 `system::auth_window(url, hostname) -> callback_url`：

- `WebviewWindowBuilder::on_navigation` 在 **builder 上**挂（`WebviewWindow` 事后没有这个方法）
- 回调判定：同 host + 路径 `/` + 带 `code` 参数
- 命中后 `on_navigation` 返回 `false`，避免把回调页真的渲染出来
- `clear_all_browsing_data()` 对应 `clearStorageData`
- 用户关窗 → `WindowEvent::Destroyed` → channel 收到 Err，`login()` 静默返回（与 Electron 一致）
- 5 分钟超时，避免窗口卡住时一直挂着

Node 侧改成 `authWindow(url, hostname)`（`native-shim.js` 的 bridge），
`BrowserWindow` 已完全移出 `git.js`。

### 8.3 实测

```
git.server        => https://gitlab.pavlovia.org
git.loadUsers     => null            （读取加密文件成功）
git.loadProjects  => {}
git.listGroups    => [真实 Pavlovia 组...]   ← 网络+认证链路通
git.stage         => ["a.txt"]
git.getRemote     => null
git.getProjectInfo => null
git 事件通道      => ["git"]        ← 走 sendEvent，正常
```

`git.login` 在纯 sidecar 下静默返回 null（拿不到宿主窗口），符合设计。

---

## 9. 迁移检查清单

每个通道迁移时必须确认：

- [ ] 通道名映射（snake_case 转换规则统一）
- [ ] 参数顺序与命名
- [ ] 返回结构逐字段一致
- [ ] 错误路径行为一致（见 §3.1）
- [ ] 若涉及事件广播，保留层级广播 + 全窗口广播（见 §3.3）
- [ ] 前端调用点无需改动（除非是 D6 的同步→异步）
