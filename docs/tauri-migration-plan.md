# GnosisStudio：Electron → Tauri 迁移方案

**文档版本**：v0.3
**状态**：全部决策已闭环（§9.1 D1–D9），进入实施阶段
**作者**：ArchExpert
**日期**：2026-11-04

---

## 1. 结论先行

迁移可行，且比想象中干净。核心原因：**前端（`src/`）与后端（`electron/`）之间只有一条 IPC 契约**，后端 90% 的代码是纯 Node（`fs`/`child_process`/`isomorphic-git`/`express`），不依赖 Electron 独有 API。

真正的硬骨头只有 **5 个**，全部集中在窗口与生命周期模型上：

| # | 难点 | 性质 |
|---|---|---|
| 1 | SvelteKit 被当作"常驻 HTTP 服务"用（多窗口加载 `/runner`、`/coder`、`/builder`） | 架构级 |
| 2 | 多窗口状态同步靠 `webContents.send` 广播 + 全局 `state.js` 单例 | 架构级 |
| 3 | 自定义协议 `psychopy-studio://` + 单实例 + 文件关联 | Electron 特有，Tauri 有对应机制但要重接 |
| 4 | `window.electron` 是 contextBridge 代理对象，`$state()` 直接持有它 | 隐性陷阱，不改会静默失效 |
| 5 | PsychoJS 用 express 起本地 HTTP 服务给实验页加载资源 | 可换 Tauri 的 `asset` 协议，反而更轻 |

**建议路线**：不是"删 Electron 换 Tauri"，而是**先在 Tauri 里复刻出 IPC 契约，让 Electron 继续跑，逐个后端模块移植，全部通过后再切窗口系统最后一步**。理由见 §6。

---

## 2. 项目现状

### 2.1 仓库布局

```
generated/GnosisStudio/    ← 原始项目（PsychoPy Studio 2027.1.0），Electron + SvelteKit + JS，Git 分支 dev
├── electron/              ← Node 后端，7677 行
│   ├── src/
│   │   ├── index.cjs          290 行  入口、生命周期、单实例、协议注册
│   │   ├── svelte.js          233 行  常驻 express 服务器，端口 8003
│   │   ├── frames.js          155 行  多窗口管理（BrowserWindow/Menu）
│   │   ├── preload.js         157 行  contextBridge，暴露 window.electron / .git / .python
│   │   ├── files.js           261 行  文件系统 IPC
│   │   ├── python/            ~1400 行  Python 子进程管理（uv/venv/liaison/shell/script/psychojs）
│   │   ├── tools/             git.js(974) / state.js(123) / usage / logging / preferences / system / random / version
│   │   └── resources/         启动画面、favicon、splash.html
├── src/                   ← SvelteKit 前端，279 文件（171 .svelte + 69 .js）
│   ├── routes/           /builder /coder /runner /api(+page)
│   └── lib/              实验、UI、pavlovia、python、i18n
├── web/preload.js         浏览器环境降级用的空实现
├── svelte.config.js       adapter-static，pages: ./dist，fallback 404.html
└── package.json           pnpm@12.16.1，electron + electron-builder 走 NSIS/Windows

generated/demo/GnosisStudio/ ← 新脚手架（tauri create，未动过，无独立 .git，由外层仓库管理）
├── src-tauri/             Rust 侧：Cargo.toml、tauri.conf.json、capabilities/default.json
├── src/                   只有 app.html + +page.svelte
└── package.json           Svelte 5.56 + Vite 8 + @tauri-apps/api 2 + adapter-static
```

> 目录关系：`generated/GnosisStudio/` 是带 `.git` 的原项目仓库（当前在 `dev` 分支），
> `generated/demo/GnosisStudio/` 是放在同一工作区里的 Tauri 骨架，用于验证与最终替换。

### 2.2 技术栈对比

| 层 | 现状（Electron） | 目标（Tauri 2.x） |
|---|---|---|
| 壳 | Electron | Tauri 2.x（Rust 核心） |
| 前端 | SvelteKit + adapter-static | 不变 |
| 运行时 | Node 内置（`node:path` 等） | **Rust 侧无 Node**——这是最大决策点，见 §4.1 |
| 窗口 | `BrowserWindow` × N | `WebviewWindow` × N |
| IPC | `contextBridge` → `ipcMain.handle` | `invoke` → `#[tauri::command]` |
| 事件 | `webContents.send(tag, data)` | `emit` / `listen` |
| 静态服务 | express @ :8003 常驻 | `tauri.conf.json` 直连 `frontendDist`，**服务彻底消失** |
| Python | `child_process` 拉起 uv/venv | **不变**，仍是 Node 子进程（见 §4.1） |
| Git | `isomorphic-git`（纯 JS） | 不变，纯 JS 在 webview 里跑 |
| 打包 | electron-builder + NSIS | Tauri bundler |

### 2.3 当前运行形态（关键）

理解现状是理解迁移代价的前提：

1. Electron 主进程启动 → 拉起一个 **express 服务器监听 :8003**，把 `dist/` 挂上去
2. 主窗口 `BrowserWindow.loadURL("http://localhost:8003/")` —— **前端是被当服务端渲染在用的**
3. 用户切 Builder/Coder/Runner，`frames.js` **新开 `BrowserWindow` 加载 `http://localhost:8003/builder` 等**，每个窗口独立 preload
4. preload 通过 `contextBridge.exposeInMainWorld` 把 `electron` / `git` / `python` 三个对象挂到 `window`
5. `src/lib/globals.svelte.js` 用 `$state(window.electron)` 捕获，全局共享
6. 业务代码调 `python.liaison.send()` → IPC → 主进程 → Python 子进程；Python 输出 → `webContents.send("stdout")` → 前端 `.listen("stdout")`

**这套形态决定了：Tauri 侧必须复刻 `window.electron` / `window.git` / `window.python` 的形状，且方法名、返回结构、事件 tag 全部一致。** 前端 279 个文件里对后端的调用点约 200 处（`python.*` 60+、`electron.files` 37、`electron.windows` 20、`electron.paths` 11），逐一改不现实也不必要。

### 2.4 PsychoJS 与 Python 环境的职责边界（常见误解澄清）

> **结论：PsychoJS 不能替代 Python 环境。两者解决的是完全不同的问题——Python 负责「制作」，PsychoJS 负责「播放」。**
>
> 这也是为什么本迁移方案必须保留 Python 工具链：它属于应用核心业务，与 Electron→Tauri 的壳替换无关。

#### 误解来源

项目里有个文件叫 `electron/src/python/psychojs.js`（126 行），容易被理解成「Python 与 PsychoJS 是同一层」。**实际它完全不碰 Python**——它只是：

- 一个 `express.static(cwd)` 静态服务器
- 加一个 `BrowserWindow` 加载 `http://<addr>?...`

把磁盘上已编译好的 JS 文件喂给浏览器窗口。文件放在 `python/` 目录下只是归类习惯。

#### PsychoJS 是什么

**PsychoJS 是 psychopy 从实验脚本编译出来的 JS 产物**，负责播放实验：绘制刺激、呈现图像与计时、监听按键、记录反应时。

**它只负责播放，不负责制作。** 它本身也不是一个可独立安装的 npm 包——`experiment.svelte.js:838` 显示它由 Python 侧现生成：

```js
await python.liaison.send("app", {
    command: "run",
    args: ["psychopy.tools.servertools:getPsychoJS"],   // ← 从 psychopy 里取出 PsychoJS
    kwargs: { cwd: ..., useVersion: ... }
}, 100000)
```

#### Python 环境实际承担的 6 件事（全部位于 PsychoJS 上游）

| # | 用途 | 代码证据 | 可否用 PsychoJS 替代 |
|---|---|---|---|
| 1 | 把 `.psyexp` 编译成 `.py` / `.js` | `experiment.svelte.js:651` "Cannot compile to Python on an experiment with no psyexp file attached"；`:661` 输出 `.js`/`.py` | ❌ 编译器本身是 Python（`psychopy.scripts.psyexpCompile`） |
| 2 | 生成 PsychoJS 库本体 | `experiment.svelte.js:838` `psychopy.tools.servertools:getPsychoJS` | ❌ 从 psychopy 中取出，非独立包 |
| 3 | 提供组件清单与参数 schema | `liaison.js` 启动时 `init prefs`、`activatePlugins` | ❌ schema 定义在 Python 侧 |
| 4 | 加载插件 / 硬件（如 Cedrus 反应盒） | `liaison.js:185` `psychopy.plugins:activatePlugins` | ❌ 插件是 Python 包 |
| 5 | alert 系统 | `liaison.js:134` `psychopy.alerts.liaison:LiaisonAlertHandler` | ❌ |
| 6 | 本地试跑时提供实验服务 | `getPsychoJS` + `python.psychojs.run` | ❌ 依赖第 2 项 |

**时间轴视角**：`编辑 .psyexp → [Python: 编译+生成] → 产出 .js → [PsychoJS: 浏览器播放]`。Python 在前，PsychoJS 在后，不存在替代关系。

#### 纯 JS 路线的真实状态

`experiment.svelte.js:820` `runJS()` 里已经写明：

```js
if (this.pilotMode) {
    if (!python) {
        console.error("Script running is not available in browser.")
        return                              // ← 没有 Python 就没有本地试跑
    }
    ... await python.psychojs.run(...)
} else {
    // todo: Run in JS on Pavlovia (not pilot)     // ← 纯 JS 路线，原作者未完成
}
```

作者在上游留了 `todo`，说明「纯 JS 制作器」是一个独立产品级目标（需要重造 `.psyexp` 解析器 + 代码生成器 + 插件系统），**不在本次迁移范围内**。

#### 对迁移的意义

| 项 | 结论 |
|---|---|
| Python 工具链的去留 | **必须保留**，与 Tauri 无关；因此选用 Node sidecar 方案（见 §4.1） |
| `psychojs.js` 的 express 服务 | **可简化**——这是全场唯一能真正裁掉的后端代码：M7 用 Tauri 自定义 protocol 替掉，顺带删除 `express` 依赖与 `getSafeAddress()` 端口探测（见 §4.3 难点 5） |
| 前端 `if (python)` / `if (electron)` 降级分支 | 保留，作为 web 部署兜底，不动 |

---

## 3. IPC 契约清单（迁移的"接口规范"）

这是本次迁移唯一必须**逐字节保真**的东西。`ipcMain.handle` 共 **91 个**（electron 36 / python 33 / git 22），另有 **1 个** `ipcMain.on` 同步通道（`electron.state.getProfiles`）。

### 3.1 `electron.*`（18 项）

| 分组 | 方法 | 通道 |
|---|---|---|
| windows | new / get / send / focus / devtools / close / hideMenu / setMenu（8 个 handle）+ emit / listen（`ipcRenderer.send/on`） | `electron.windows.*` |
| paths | getPathForFile / documents / user / devices / prefs / pavlovia{dir,users,projects} | `electron.paths.*` |
| files | load / save / exists / mkdir / scandir / stat / openDialog / saveDialog / downloadFolder / openExternal / openPath / showItemInFolder | `electron.files.*` |
| clipboard | get / set | `electron.clipboard.*` |
| state | getProfiles(**同步**) / setProfiles | `electron.state.*` |
| system | requestKeyboardAccess / hasKeyboardAccess | `electron.system.*` |
| 顶层 | version / platform / quit | `electron.version` / `electron.platform` / `electron.quit` |

### 3.2 `python.*`（38 项）

- `uv.*`（14）：executable / findDirectory / findPython / install / getEnvironments / folder / needsUpdate / setDirectory / makeExecutable / exists / systemInfo / resolvePackageVersion / python
- `venv.*`（7）：setup / executable / installPackage / uninstallPackage / getPackages / getPackageDetails / hasGIL
- `liaison.*`（6）：start / stop / send / started / ready / listen(`liaison:${tag}`)
- `shell.*`（5）：open / send / close / list / listen
- `scripts.*`（4）：run / stop / wait / listen
- `psychojs.*`（2）：run / stop
- `output.*`（3）：stdout / stderr / app（**纯事件，无 handler**）

### 3.3 `git.*`（23 项）

login / logout / listUsers / loadUsers / getUserInfo / clearUsers / newProject / linkProject / listGroups / getProjectInfo / listProjectForks / loadProjects / clone / fork / push / pull / commit / stage / authenticateURL / listSurveys / listProjectForks / getRemote / server

后端是 `tools/git.js`，纯 `isomorphic-git` + `http/node`，无 Electron 依赖（只 import 了 `BrowserWindow` 用于广播事件）。

### 3.4 事件 tag（后端 → 前端，共 9 类）

`stdout`、`stderr`、`uv`、`git`、`import sys`、`fileOpen`、`alert`、`error`、`showTips`
另有 `liaison:${tag}`（动态）、`psychojs:*`、`scripts:*`

前端订阅方式：`.listen(tag, cb)` / `.listen("uv", cb)`。**注意 `liaison:*` 是前缀拼接**，迁移时别写死。

---

## 4. 技术方案

### 4.1 核心决策：Node 侧代码怎么处理

`electron/src/` 里 7677 行，其中绝大部分是**与 Electron 无关的纯 Node 代码**：`files.js`（fs）、`tools/git.js`（isomorphic-git）、`python/*.js`（child_process 调度 uv/venv）、`tools/state.js`、`tools/usage.js`、`tools/logging.js`。

只有 4 个文件真正耦合 Electron：`index.cjs`、`svelte.js`、`frames.js`、`preload.js`。

**三条路线**：

| 方案 | 做法 | 评价 |
|---|---|---|
| A. 全 Rust 重写 | 7677 行 JS 翻成 Rust | ❌ 见下方对比。**否决** |
| B. 全删 Node，只留 Rust（同 A 的严谨版，只重写业务层） | 把 uv/venv/isomorphic-git 全换成 Rust crate | ❌ 见下方对比。**否决** |
| **C. 保留 Node 作为 sidecar 运行时** | Tauri 启动时 spawn 一个精简 Node 进程跑 IPC 服务，Rust 只做壳 | ✅ **已确认采用（2026-11-04）** |

#### 4.1.1 为什么不用纯 Rust：证据

分水岭不在"重写工作量"，而在 **1900 行靠真实用户环境试出来的经验代码，且项目 0 自动化测试**。

**代码规模切分**（`electron/src/` 7677 行）：

```
├── 真正 Electron 专属        ~700 行   index.cjs / svelte.js / frames.js 的壳部分
├── 两种方案成本相当            ~2500 行  files.js(261) logging(99) state(123) usage(71)
│                                       preferences system random version shell(113) script(65)
└── 重写最痛的部分             ~1900 行  ← 分歧点
    ├── python/venv.js  497
    ├── python/uv.js    377
    ├── python/liaison.js 314
    ├── tools/git.js    974
    └── python/utils.js 204
```

**三个 Rust 生态难补的坑**：

| 坑 | 位置 | 纯 Rust 的代价 |
|---|---|---|
| **1. uv/venv 兼容分支** | `venv.js:198` 老版本回退 pip install；`:217` `--inexact`；`:291` `pip list --format json`；`:314` `pip show` 文本解析；`utils.js:188` `execTracked` 带超时+env 合并+实时 stdout/stderr 推送 | `tokio::process` 本身不难，但**这些分支的正确性来自真实用户环境试错**。无测试保护，重写等于经验归零 |
| **2. liaison 动态载荷** | `liaison.js:93` 直连 `new WebSocket(ws://addr)`；消息是 `JSON.parse` 后的动态结构；前端 **39 处** `python.liaison.send` 调用点无类型定义 | 要么 `serde_json::Value` 全动态（等于没写 Rust），要么先**逆向出 schema**。纯新增 300–500 行 |
| **3. isomorphic-git 的 3 个隐形契约** | `git.js:713/845/929` 三处 `onAuth` 回调（异步前端弹窗登录）+ `onMessage` 实时进度；`:4` `isomorphic-git/http/node` 的 OAuth token 注入 | `git2`/`gix` 无对等物。要重写 Pavlovia GitLab OAuth + 跨 IPC 的异步认证往返，约 500–800 行 + 真账号调试 |

外加 **`safeStorage` 旧密文兼容**（`git.js:265/333`）：Rust 侧需用 keychain crate 且能解密用户已存 token，否则**全体用户重登**。

#### 4.1.2 风险对照

| # | Node sidecar 风险 | 严重度 | 缓解 |
|---|---|---|---|
| R1 | **IPC 传输层要新写**：91 个 `ipcMain.handle` → JSON-RPC，约 400 行。真正风险在**错误语义**：前端 `functions.svelte.js:17` 依赖 `"error" in err` 判断 | 🔴 | 纯机械映射，无业务逻辑，出错立刻暴露；M2 做成可插拔传输，Electron 下跑全回归 |
| R2 | Node 运行时随包分发（+50MB，arch/平台匹配、代码签名与 macOS 公证） | 🟡 | `@yao-pkg/pkg` 或直接打包 node 二进制；签名纳入 M8 |
| R3 | 进程孤儿（Tauri 崩溃时 sidecar 残留，占 venv 与端口） | 🟡 | 父进程 PID 心跳 + Rust `RunEvent::Exit` 显式 kill |
| R4 | 调试链路变三层（Rust → Node → Python） | 🟡 | 统一 trace id，`logging.js` 加来源标记 |

| # | 纯 Rust 风险 | 严重度 | 说明 |
|---|---|---|---|
| R5 | **行为等价性无法验证** | 🔴🔴 | 最致命。1900 行无测试覆盖，只能人工回归；而用户环境 psychopy/Python 版本组合极多 |
| R6 | liaison schema 逆向出错不报错 | 🔴 | 只是运行时 undefined；且上游 liaison-py 仍在演进，struct 会跟着碎 |
| R7 | Pavlovia OAuth 重写 | 🔴 | 最易出"登录上了但 push 不了"的诡异问题 |
| R8 | 单一技能依赖 | 🟡 | 重写后维护仓库必须会 Rust，Svelte 开发者不再能看后端 |
| R9 | 无法增量交付 | 🟡 | 必须先重写完一大块才能首次启动，中间态不可演示、不可回滚 |

#### 4.1.3 规模与工期

| | Node sidecar | 纯 Rust |
|---|---|---|
| 业务逻辑改动 | 0 | 全部重写 |
| 新增 Rust | ~800–1200 行 | ~6000–7000 行 |
| 包体积 | ~+50MB | ~10–15MB |
| 冷启动 | +300–600ms | 快 |
| 内存 | +40–80MB | 低 |
| 工期（1 人） | **6–10 周** | **4–7 个月** |

#### 4.1.4 后续可选：分阶段 Rust 化

先 sidecar 全量跑通拿到可用版本，再按风险从低到高逐步裁：

```
M0–M9   Node sidecar 全量跑通         ← 永远有一个能用的版本
后续    files.js / logging.js / state.js  → 直接 Rust（低风险，无业务分支）
        tools/git.js                      → 候选第二
        python/venv.js + liaison.js       → 最后，且必须先补集成测试再动
```

**方案 C 具体形态**：

- Tauri 主进程启动 → 通过 `Command::new("node").arg("backend/main.mjs")` 拉起一个 **headless Node 后端**
- 该后端复用现有 `electron/src/**` 中**除 `index.cjs` / `svelte.js` / `frames.js` / `preload.js` 之外的 100% 代码**
- 去掉 `require("electron")`，把 IPC 传输层从 `ipcMain.handle` 换成 **本地 HTTP/Unix socket + SSE**（或直接 `stdio` JSON-RPC）
- Rust 侧只负责：窗口、菜单、协议、单实例、文件关联、开机自启等**系统能力**，通过 `invoke` 暴露给前端，再代理给 Node 后端
- 前端 `window.electron` / `window.git` / `window.python` **形状完全不变**，由 preload 的 Tauri 版从 `invoke` 构造

**代价**：包体积比纯 Rust 方案大（要随包带 Node 运行时，约 +50MB，现阶段已确认可接受）。**收益**：7677 行里约 6800 行零改动，工期 6–10 周。

> 这是本方案最重要的一句话：**我们不是把 Electron 换成 Tauri，是把 Electron 的"壳"换成 Tauri，把 Electron 的"Node 运行时"留下来。**

### 4.2 前端不改动的部分

- `src/` 全部 279 文件：不动
- `svelte.config.js` / `vite.config.ts`：不动（`adapter-static` 适配 Tauri 的 `frontendDist`）
- `web/preload.js` 降级实现：可保留为 web 部署兜底

需要改的只有 **1 个文件 + 1 处补充**：

- `src/lib/globals.svelte.js`：`window.electron` 在 Tauri 下仍是 contextBridge（Tauri 的 `core` 插件），`$state()` 持有代理对象这个模式**可以继续用**，但要确认 Tauri 的 `invoke` 返回值是普通对象而非惰性代理——见 §5.3
- 新增 `src/lib/tauri-globals.svelte.js` 或直接改 globals：注入方式从"preload 自动挂 window"改为"启动时显式装配"

### 4.3 五个难点的解法

#### 难点 1：SvelteKit 的 HTTP 服务模型 → 直接删除

`svelte.js` 的 express 服务器（:8003）在 Tauri 下**完全不需要**。Tauri 的 `frontendDist` + `devUrl` 已经原生解决静态资源服务，`WebviewWindow` 可以直接加载任意 `http://localhost:1420/<route>`。

- **删除**：`electron/src/svelte.js`（233 行）、`express` 依赖
- **改动**：`frames.js` 的 `newWindow(target)` 从 `loadURL(http://localhost:8003/target)` 改为 `WebviewWindowUrl::App(target)` 或 `WebviewWindowUrl::External`
- **注意**：`python.psychojs.run` 里也用了 express 给**实验页**加载资源（`expressApp.use(express.static(this.cwd))`）。这个不能删——但 Tauri 有原生方案：注册自定义 protocol `tauri://psychojs` 指向任意目录，**比 express 更轻更快，还不用管端口冲突**

#### 难点 2：多窗口状态同步 → 换掉全局 state 单例

现状：`tools/state.js` 在**主进程**维护 `frames: {}` 和 `profiles: {}`，窗口打开/移动/缩放时通过 `webContents.send` 广播，前端 `.listen()` 收。`getProfiles` 甚至用 `sendSync` 同步返回。

Tauri 对应机制：
- `WebviewWindow` 的事件通过 `app.emit` / `WebviewWindow::emit` 广播，`listen()` 订阅——**模型一致，可直接映射**
- **同步 `getProfiles` 是硬伤**：Tauri 的 `invoke` 全是异步的。两个选择：
  1. 改成异步 `getProfiles()`，前端那一处调用（`preload.js:145` `ipcRenderer.sendSync`）改 `await`。**推荐**，前端只有 1 个调用点
  2. 启动时把 profiles 作为初始参数注入窗口 URL 的 query/hash。可行但丑
- `state.js` 整体可以保留在 Node sidecar 里，**零改动**

#### 难点 3：协议 + 单实例 + 文件关联 → Tauri 原生支持，重接即可

| Electron | Tauri 2.x | 工作量 |
|---|---|---|
| `app.requestSingleInstanceLock()` | `single-instance` plugin | 小 |
| `app.setAsDefaultProtocolClient('psychopy-studio')` | `tauri.conf.json` `bundle.deepLinkProtocol` + plugin | 小 |
| `second-instance` 事件 | `single-instance` 的 callback 参数 | 小 |
| `open-file` (macOS) | macOS 上 deep-link 走 `open-url` / `opened` 事件 | 小 |
| electron-builder `fileAssociations` | `tauri.conf.json` `bundle.fileAssociations` | 小 |

`onFileOpen(evt, file)` 里解析 `psychopy-studio://group/name` 再 `newWindow` 的逻辑可 1:1 搬进 Rust 的 single-instance callback。

#### 难点 4：`$state(window.electron)` 的 contextBridge 陷阱 → 最需要先验证的一点

`src/lib/globals.svelte.js`:
```js
export var electron = $state(window.electron)
export var git = $state(window.git)
export var python = $state(window.python)
```

`window.electron` 在 Electron 下是 **contextBridge 代理**，不是普通对象。`$state()` 包它之后，Svelte 5 的 rune 会**深度代理**它。这在 Electron 下"碰巧能跑"，因为 contextBridge 代理本身是惰性的、按属性访问的。

**风险**：Tauri 的 `@tauri-apps/api` 里 `invoke` 返回的是 **Promise**，`window.__TAURI__` 注入的对象结构和 contextBridge 完全不同。如果继续 `$state(window.electron)`，Svelte 的 deep proxy 可能会**包装 Promise 或吞掉 getter**，导致 `electron.files.load()` 静默变成 undefined。

**必须做的第一件事（M0，见 §6）**：写个 20 行的 spike，验证
```js
export var electron = $state(window.electron)  // Tauri 注入版
await electron.files.load("/tmp/x")
```
能不能跑通。跑不通就改成显式构造普通对象：
```js
export const electron = { files: { load: (f) => invoke('files_load', {file: f}) }, ... }
```
**这是整个迁移里唯一可能让前端 200 处调用点都要动的地方**，所以必须最先验证。

#### 难点 5：PsychoJS express 服务 → 换自定义 protocol

`python/psychojs.js` 里：
```js
let expressApp = express();
expressApp.use(express.static(this.cwd));
let address = await getSafeAddress();  // tcp-port-used 找空闲端口
server.win.loadURL(`http://${address}?${params}`)
```

Tauri 方案：注册 `psychojs` protocol handler，指向 `cwd` 目录：
```rust
tauri_builder.register_uri_scheme_protocol("psychojs", |ctx, req| {
    // 从 req.uri() 取路径，读 cwd 下文件返回
});
```
然后 `WebviewWindowUrl::External("psychojs://localhost/?...")`。

**收益**：消灭端口竞争、消灭 `tcp-port-used` 依赖、消灭 `getSafeAddress` 逻辑、实验页加载更快。**代价**：要写约 30 行 Rust。值得。

---

## 5. 目标架构

```
┌─────────────────────────────────────────────────────────┐
│ Tauri 主进程 (Rust)                                       │
│  • WebviewWindow × N  (builder / coder / runner / psychojs实验窗) │
│  • 菜单、托盘、协议、单实例、文件关联                       │
│  • #[tauri::command] 薄封装：系统能力直接实现              │
│    (paths / dialog / clipboard / shell.open / fs 基础操作) │
│  • 代理层：其余 invoke → 转发给 Node sidecar              │
└───────────────┬─────────────────────────────────────────┘
                │ spawn node backend/main.mjs (sidecar)
                │ stdio JSON-RPC / 本地 HTTP+SSE
┌───────────────▼─────────────────────────────────────────┐
│ Node 后端 (复用 electron/src/** 的 6800 行)               │
│  python/uv.js venv.js liaison.js shell.js script.js      │
│  tools/git.js state.js usage.js logging.js               │
│  files.js（纯 fs）                                        │
│  transport/  ← 新增：ipcMain.handle → JSON-RPC 适配层     │
└───────────────┬─────────────────────────────────────────┘
                │ child_process
┌───────────────▼─────────────────────────────────────────┐
│ uv 0.12.12 → venv → Python 子进程（liaison / shell /      │
│ scripts / psychojs runner）—— 完全不变                    │
└─────────────────────────────────────────────────────────┘
```

**关键**：Node sidecar 不是"临时方案"。Python 工具链（uv/venv/psychopy）的调度逻辑是这个应用的**核心价值**，用 JS 写、跑在 Node 上，稳定且已验证。把它留在 sidecar 里，等于**只替换了 Electron 这个壳**——这正是用户要的（"用 tauri 替换掉笨重的 electron"），而不是"顺便把后端也重写一遍"。

---

## 6. 迁移里程碑

**原则：每一步都能独立跑通、独立提交。Electron 在整个迁移期间保持可用，直到最后才切换。**

| 里程碑 | 内容 | 交付判据 |
|---|---|---|
| **M0 Spike** | 在 `GnosisStudio/` 脚手架里手写 1 个 `invoke` command + 前端 1 次调用，验证 `$state(window.electron)` 能否工作；验证 `WebviewWindow` 多开 + `emit/listen` | demo 里点按钮弹框、两个窗口互相收事件 |
| **M1 契约层** | 把 `electron/src/**` 的 92 个 channel 整理成 `CONTRACT.md`（通道名、参数、返回结构、事件 tag），作为唯一规范 | 文档评审通过 |
| **M2 Transport** | 新增 `backend/transport/`：把 `ipcMain.handle` 抽象成可插拔传输（`electron` / `jsonrpc` 两实现），Electron 下跑通 | Electron 下全部功能回归通过 |
| **M3 Node sidecar** | Tauri 主进程 spawn `backend/main.mjs`，JSON-RPC over stdio；系统能力（paths/dialog/clipboard/shell）在 Rust 侧实现 | Tauri 下 builder 页面能读写文件、能开 venv |
| **M4 Python 栈** | 移植 `python/*`：uv → venv → liaison → shell → scripts 全部走通 | Tauri 下能装 psychopy、能起 liaison、能跑脚本 |
| **M5 多窗口 + 状态** | 移植 `frames.js` / `state.js` 到 Tauri；`getProfiles` 改异步；窗口位置/大小/最大化状态持久化 | 三个视图互相切换、状态恢复 |
| **M6 Git (Pavlovia)** | 移植 `tools/git.js`：isomorphic-git 在 webview 里跑还是 sidecar 里跑，需实测（见 §7） | 能登录、能 clone、能 push |
| **M7 PsychoJS** | express 换成 `psychojs` protocol；实验窗口多开 | 能跑一个完整实验并出数据 |
| **M8 协议 + 打包** | deep-link、单实例、文件关联、NSIS 打包、开机自启 | 双击 `.psyexp` 能打开；`psychopy-studio://` 能拉起 |
| **M9 切换** | 前端 globals 切到 Tauri 注入；**删除 `electron/`** 全部 Electron 专属文件 + `electron` 依赖 | 仓库无 `require("electron")` |

---

## 7. 风险与待确认

| 风险 | 影响 | 应对 |
|---|---|---|
| **R1** `$state(window.electron)` 在 Tauri 注入下失效 | 🔴 高——可能波及前端 200 处调用 | **M0 最先验证**。失败则改 globals 显式构造，前端 0 改动 |
| **R2** `isomorphic-git` 在 webview 里跑不了（依赖 `http/node`） | 🟡 中——`tools/git.js` 用了 `isomorphic-git/http/node` | **必须放 sidecar**，不能放 webview。M6 确认 |
| **R3** `safeStorage`（加密存 token，`git.js:265/333`）在 Tauri 下无对应 | 🟡 中——Pavlovia 登录凭证存储 | ✅ 已决（D7）：**不兼容旧密文**，改用 Tauri keychain 方案，迁移后强制重登一次 |
| **R3b** `git.js` 用 `BrowserWindow` 做两件事：① 登录窗口（`git.js:171`）② 广播事件（`git.js:946` `getAllWindows().forEach(send)`） | 🟡 中 | ① 改 `WebviewWindow`；② 改 `app.emit` |
| **R3c** `getSafeAddress()` 从 8002 起递增探测空闲端口，用于 **PsychoJS** 服务 | 🟢 低 | ✅ 已决（D5）：protocol 方案后整个函数可删（`python/utils.js:72`） |
| **R4** Node sidecar 随包分发体积 | 🟢 低——+50MB | ✅ 已决（D2）：可接受；也可后续裁 Rust |
| **R5** `sendSync("electron.state.getProfiles")` 无同步替代 | 🟡 中 | ✅ 已决（D6）：改异步，前端 1 处调用点（`preload.js:145`） |
| **R6** macOS `open-file` 时序（应用未启动时收到文件） | 🟡 中 | Tauri `opened` 事件有 buffer，M8 处理 |
| **R7** `python.psychojs.run` 依赖 express + BrowserWindow | 🟡 中 | ✅ 已决（D5）：服务不对外，M7 用 protocol 替掉 |
| **R8** 前端有 `if (electron)` 的 web 降级分支 | 🟢 低 | 保留 `web/preload.js` 兜底，不动 |

**M0 阶段必须验证的技术假设**（其余决策已闭环）：

1. `window.electron` 在 Tauri 下到底是"contextBridge 式惰性代理"还是"普通对象"？——决定 R1 的解法
2. `isomorphic-git/http/node` 是否强依赖 Node 的 `http` 模块？是 → 必须放 sidecar（M0 只做判定，M6 实施）

---

## 8. 规模估算

| 项 | 行数 | 处理 |
|---|---|---|
| 删除（Electron 专属） | ~676（`index.cjs` 290 + `svelte.js` 233 + `frames.js` 155，减去重叠） | 逻辑搬 Rust 或重写 |
| **保留（纯 Node）** | **~6800** | 零改动，仅换 IPC 传输层 |
| 新增 Rust | ~800–1200 | 窗口/菜单/协议/系统能力/transport 桥接 |
| 新增 `backend/transport/` | ~400 | `ipcMain.handle` → JSON-RPC 适配 |
| 前端改动 | **~50** | globals 装配 + getProfiles 异步化 |

**结论：这不是重写，是换壳。6800 行稳的、已验证的 Node 代码一行不动。**

---

## 9. 决策状态

### 9.1 已确认

| # | 决策 | 结论 | 日期 |
|---|---|---|---|
| D1 | 后端方案（§4.1） | ✅ **采用 Node sidecar**；纯 Rust 重写否决 | 2026-11-04 |
| D2 | 包体积 +50MB | ✅ 现阶段可接受 | 2026-11-04 |
| D3 | Python 环境去留（§2.4） | ✅ 必须保留，与 Tauri 迁移无关 | 2026-11-04 |
| D4 | 迁移仓库与分支 | ✅ `generated/GnosisStudio` 仓库，分支 `feat/tauri-migration`（从 `dev` 切出） | 2026-11-04 |
| D5 | PsychoJS express 服务是否对外 | ✅ **不对外**，仅本机加载资源 → M7 采用 Tauri 自定义 protocol，保留 HTTP 方案作废 | 2026-11-04 |
| D6 | `getProfiles` 同步 → 异步 | ✅ 改为异步，启动时序随之调整（前端仅 1 处调用点 `preload.js:145`） | 2026-11-04 |
| D7 | `safeStorage` 旧 token 兼容 | ✅ **不兼容**，迁移后一次性强制重登。理由：Rust 侧实现旧密文解密成本高、收益低，且 Pavlovia token 重取代价小 | 2026-11-04 |
| D8 | `demo/GnosisStudio` 脚手架重写 | ✅ 在 M0 阶段完成（`package.json` / `tauri.conf.json` / 端口 / identifier 对齐） | 2026-11-04 |
| D9 | M0 spike 作为首个任务 | ✅ 是，M0 先于一切业务移植（决定前端 200 处调用点是否要动） | 2026-11-04 |

### 9.2 无遗留待确认项

所有阻塞性决策已闭环。后续若出现新分歧，在对应里程碑开工前单独提出。
