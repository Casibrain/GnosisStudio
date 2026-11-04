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

---

## 4. 前端调用点统计

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

## 5. 迁移检查清单

每个通道迁移时必须确认：

- [ ] 通道名映射（snake_case 转换规则统一）
- [ ] 参数顺序与命名
- [ ] 返回结构逐字段一致
- [ ] 错误路径行为一致（见 §3.1）
- [ ] 若涉及事件广播，保留层级广播 + 全窗口广播（见 §3.3）
- [ ] 前端调用点无需改动（除非是 D6 的同步→异步）
