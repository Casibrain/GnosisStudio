/**
 * Node sidecar 入口 —— M3
 *
 * Tauri 主进程以子进程方式拉起本文件，通过 stdio 说 JSON-RPC。
 * 它加载的业务代码与 Electron 版**完全相同**（electron/src/**），
 * 唯一的差别是 "electron" 这个模块名在解析时被指向 electron-shim.js。
 *
 * 解析重定向用 Node 的 module.registerHooks（Node 22+），不引入 loader 依赖。
 *
 * 启动协议（Rust ↔ sidecar）：
 *   Rust → sidecar : {"jsonrpc":"2.0","id":1,"method":"python.uv.install","params":[{"psychopy":"2024.1.0"}],"sender":"main"}
 *   sidecar → Rust : {"jsonrpc":"2.0","id":1,"result":...}
 *   sidecar → Rust : {"jsonrpc":"2.0","method":"event","params":{"tag":"uv","data":"..."}}
 *   sidecar → Rust : {"jsonrpc":"2.0","method":"ready","params":{"version":"..."}}   ← 就绪信号
 */

import { registerHooks } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// stdout 是 JSON-RPC 专用通道，任何业务代码的 console.log 都会污染它。
//
// 业务代码里 logging.log(...) 默认 echo=true 走 console.log（logging.js:53），
// execSync 也会经 input() -> output() -> logging.log 打印命令，实测会把
// 'UV\t>> "..."' 直接写进 stdout，导致 Rust 侧 JSON.parse 失败。
//
// 所以：在加载任何业务代码之前，把 console 的输出全部改道 stderr。
// 不要让业务代码单独改 —— 那要改几十处，且以后新增的 console.log 还会漏。
// ---------------------------------------------------------------------------
{
    const toStderr = (...args) => process.stderr.write(args.map(String).join(" ") + "\n");
    console.log = toStderr;
    console.info = toStderr;
    console.debug = toStderr;
    // console.warn / console.error 本来就是 stderr，不用动
}

// 把 "electron" 解析到本目录的 electron-shim.js
registerHooks({
    resolve(specifier, context, nextResolve) {
        if (specifier === "electron") {
            return {
                url: pathToFileURL(path.join(here, "electron-shim.js")).href,
                shortCircuit: true,
            };
        }
        return nextResolve(specifier, context);
    },
});

// 现在可以安全地加载业务代码
const { useTransport, sendEvent } = await import("./transport/index.js");
const { setNativeBridge } = await import("./native-shim.js");

// 切到 JSON-RPC 后端
const transport = useTransport("jsonrpc");
const rl = transport.start();

// 把只有宿主能做的事接上（目前是 OAuth 登录窗口，需要真实 BrowserWindow）。
// 桥接方式：hostRequest() 走 stdin 发 "host.*" 请求，由 Rust 侧处理后回同一 id。
// 完整的 sidecar 进程管理在 M8 落地，这里先只注入这条桥。
const { hostRequest } = await import("./sidecar-host.mjs");
setNativeBridge({
    authWindow: (url, hostname) => hostRequest("auth_window", [url, hostname]),
});

// 引入所有 handler 模块 —— 这一步会执行每个文件顶层的 handle() 注册
await import("./files.js");
await import("./tools/state.js");
await import("./tools/git.js");
await import("./python/index.js");

// 通知宿主：所有 handler 已注册完毕
process.stdout.write(JSON.stringify({
    jsonrpc: "2.0",
    method: "ready",
    params: {
        version: process.env.GNOSTIS_VERSION ?? "0.0.0",
        channels: transport.listHandlers(),
    },
}) + "\n");

// ---------------------------------------------------------------------------
// 退出路径
//
// 原先窗口状态（位置/大小/打开的文件）由 Electron 的 app.on("before-quit")
// 落盘（index.cjs:233）。sidecar 没有这个事件，必须自己在退出前调 saveState()，
// 否则用户每次重启都丢窗口布局。
// ---------------------------------------------------------------------------
const { saveState } = await import("./tools/state.js");

let exiting = false;
function shutdown(code = 0) {
    if (exiting) return;
    exiting = true;
    try {
        saveState();
    } catch (err) {
        process.stderr.write(`[sidecar] failed to save state: ${err?.message ?? err}\n`);
    }
    process.exit(code);
}

// 父进程断开 = 退出（防孤儿进程，见风险评估 R3）
process.stdin.on("end", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));
process.on("SIGINT", () => shutdown(0));

// 未捕获异常直接上报，不要静默吞掉后变成"IPC 挂起"
process.on("uncaughtException", err => {
    process.stderr.write(`[sidecar] uncaught: ${err?.stack ?? err}\n`);
    shutdown(1);
});
process.on("unhandledRejection", err => {
    process.stderr.write(`[sidecar] unhandled rejection: ${err?.stack ?? err}\n`);
});

export { setNativeBridge, sendEvent };
