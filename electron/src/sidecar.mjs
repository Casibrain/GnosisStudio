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

// 父进程断开 = 退出（防孤儿进程，见风险评估 R3）
process.stdin.on("end", () => process.exit(0));
process.on("SIGTERM", () => process.exit(0));
process.on("SIGINT", () => process.exit(0));

// 未捕获异常直接上报，不要静默吞掉后变成"IPC 挂起"
process.on("uncaughtException", err => {
    process.stderr.write(`[sidecar] uncaught: ${err?.stack ?? err}\n`);
    process.exit(1);
});
process.on("unhandledRejection", err => {
    process.stderr.write(`[sidecar] unhandled rejection: ${err?.stack ?? err}\n`);
});

export { setNativeBridge, sendEvent };
