/**
 * Electron shim —— Node sidecar 运行时用。
 *
 * sidecar 下没有 Electron，但业务代码里 14 个文件仍 `import ... from "electron"`。
 * 本模块用 Node 标准库提供等价实现，**保持 API 形状一致**，让业务代码零改动。
 *
 * 覆盖清单（按引用次数排序）：
 *   app.getPath("appData" x21 / "documents")   ← 路径解析
 *   app.on / whenReady / quit / isPackaged     ← 生命周期（sidecar 下由 Rust 管）
 *   dialog.showOpenDialogSync / showSaveDialogSync
 *   shell.openExternal / openPath / showItemInFolder
 *   safeStorage.isEncryptionAvailable / encryptString / decryptString
 *   BrowserWindow.getAllWindows                ← 事件广播目标
 *
 * 不含（sidecar 下无对应物，调用即报错，便于尽早暴露误用）：
 *   BrowserWindow 构造、Menu、webContents —— 这些属于窗口系统（M5/M7 处理）
 */

import os from "node:os";
import path from "node:path";
import fs from "node:fs";
import { randomBytes } from "node:crypto";
import { dialog, shell } from "./native-shim.js";

/**
 * appData 必须与 Electron 的 appData 布局对齐，否则用户已装环境会找不到数据。
 *   Windows: %APPDATA%
 *   macOS:   ~/Library/Application Support
 *   Linux:   ~/.config
 * Electron 在其下再加一个目录（roaming appName）。Rust 侧要保证传入同样的 appName。
 */
function appDataPath() {
    if (process.platform === "win32") {
        return process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming");
    }
    if (process.platform === "darwin") {
        return path.join(os.homedir(), "Library", "Application Support");
    }
    return process.env.XDG_CONFIG_HOME || path.join(os.homedir(), ".config");
}

/** Rust 侧通过环境变量注入应用名，默认 psychopy4（与现网一致） */
const APP_NAME = process.env.GNOSTIS_APP_NAME || process.env.ELECTRON_APP_NAME || "psychopy4";

/**
 * 与 Electron app.getPath 对齐。
 * "appData" → <base>/<APP_NAME>，但现网代码里多数直接用裸 appData 目录存 psychopy4 文件，
 * 因此这里返回裸 base，保持与旧行为一致（见 uv.js / liaison.js 用法）。
 */
function getPath(key) {
    switch (key) {
        case "appData": {
            const base = appDataPath();
            // Electron 的 appData 是平台 base 目录，不是 appName 子目录 —— 代码里自己拼 appName
            return base;
        }
        case "documents":
            return os.homedir();
        case "home":
            return os.homedir();
        case "temp":
            return os.tmpdir();
        default:
            throw new Error(`Unsupported path key in sidecar: ${key}`);
    }
}

// 进程退出钩子
const quitHandlers = new Set();
const beforeQuitHandlers = new Set();

function quit(code = 0) {
    for (const fn of quitHandlers) fn({ code });
    process.exit(code);
}

function on(event, fn) {
    switch (event) {
        case "quit":
            quitHandlers.add(fn);
            break;
        case "before-quit":
            beforeQuitHandlers.add(fn);
            break;
        case "second-instance":
            // sidecar 下由 Rust 处理单实例，这里忽略
            break;
        default:
            break;
    }
    return app;
}

function whenReady() {
    return Promise.resolve(app);
}

function requestSingleInstanceLock() {
    // Rust 已持有单实例锁，sidecar 永远返回 false 表示"我是被拉起的"
    return false;
}

function setAsDefaultProtocolClient(protocol) {
    // 由 Rust 注册协议，sidecar 忽略
    return true;
}

const app = {
    getPath,
    setPath: () => {},
    getVersion: () => process.env.GNOSTIS_VERSION || "0.0.0",
    quit,
    on,
    whenReady,
    isPackaged: true,
    requestSingleInstanceLock,
    setAsDefaultProtocolClient,
    get isDev() { return process.env.NODE_ENV === "development"; },
};

/**
 * safeStorage —— 加密存储 Pavlovia token。
 *
 * D7 决定：不兼容旧密文。这里用 XOR + 本地密钥文件做占位（sidecar 阶段不真正安全），
 * 真正方案在 M6 用 Tauri keychain / stronghold 替换 Rust 侧实现。
 *
 * 注意：返回值为 base64 字符串，与 Electron 一致，前端无需感知。
 */
const KEY_FILE = path.join(appDataPath(), APP_NAME, ".storage-key");

function getKey() {
    try {
        return Buffer.from(fs.readFileSync(KEY_FILE, "utf8"), "hex");
    } catch {
        const key = cryptoRandom(32);
        fs.mkdirSync(path.dirname(KEY_FILE), { recursive: true });
        fs.writeFileSync(KEY_FILE, key.toString("hex"), { mode: 0o600 });
        return key;
    }
}

function cryptoRandom(n) {
    // Node 内置，跨平台（/dev/urandom 在 Windows 上不存在）
    return randomBytes(n);
}

function xor(input, key) {
    const out = Buffer.alloc(input.length);
    for (let i = 0; i < input.length; i++) out[i] = input[i] ^ key[i % key.length];
    return out;
}

const safeStorage = {
    isEncryptionAvailable: () => true,
    encryptString(str) {
        return xor(Buffer.from(str, "utf8"), getKey()).toString("base64");
    },
    decryptString(enc) {
        return xor(Buffer.from(enc, "base64"), getKey()).toString("utf8");
    },
};

/**
 * BrowserWindow —— 只在事件广播路径用到 getAllWindows。
 * sidecar 下事件由 Rust 的 emit 推送，这里返回空数组即可（广播由 shim 的 sendEvent 接管）。
 */
class BrowserWindow {
    static getAllWindows() {
        return [];
    }
}

class Menu {
    static buildFromTemplate() {
        throw new Error("Menu is not available in sidecar (window system is Rust-side)");
    }
    static setApplicationMenu() {}
}

export { app, dialog, shell, safeStorage, BrowserWindow, Menu };
