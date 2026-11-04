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
import { randomBytes, createCipheriv, createDecipheriv } from "node:crypto";
import { execFileSync } from "node:child_process";
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
 * **必须兼容 Electron 写下的旧密文**，否则用户现有的 Pavlovia 登录状态会丢失。
 * 实测本机 `~/Library/Application Support/psychopy4/pavlovia/users.json`
 * 就是 Electron safeStorage 的产物（`v10` 前缀 = Chromium OSCrypt）。
 *
 * 格式（Chromium OSCrypt，macOS）：
 *   [3 字节 "v10"] [16 字节 IV] [AES-128-CBC 密文]
 * 密钥：macOS Keychain 里的 "Electron Safe Storage" 条目，值为 base64 的 16 字节。
 * 实测该条目存在且可读取，同一个 key 能解开 Electron 写的文件。
 *
 * 不自己造 key：一处换了 key，所有旧文件立刻作废，而且用户不会收到任何提示。
 */

/** macOS：从 Keychain 读 Electron 的 OSCrypt 密钥（base64 的 16 字节） */
function macKeychainKey() {
    const raw = execFileSync(
        "security",
        ["find-generic-password", "-s", "Electron Safe Storage", "-w"],
        { encoding: "utf8" }
    ).trim();
    return Buffer.from(raw, "base64");
}

/**
 * Windows：Electron 用 DPAPI 保护一个 32 字节随机密钥，存在
 * `%APPDATA%\<app>\Local State` 的 os_crypt.encrypted_key 里。
 * 目前只在 macOS 上验证过；Windows 需要 DPAPI 调用（Rust 侧更适合），
 * 先明确失败，不要静默降级成"能加密但解不开旧数据"。
 */
function windowsKey() {
    throw new Error(
        "safeStorage on Windows is not ported yet; Electron DPAPI unwrap needs a Rust-side implementation"
    );
}

let cachedKey = null;
function getKey() {
    if (cachedKey) return cachedKey;
    if (process.platform === "darwin") {
        cachedKey = macKeychainKey();
    } else if (process.platform === "win32") {
        cachedKey = windowsKey();
    } else {
        throw new Error(
            `safeStorage: unsupported platform ${process.platform} (Electron uses libsecret here)`
        );
    }
    return cachedKey;
}

/**
 * 注意：Electron 的 encryptString 返回 **Buffer**，decryptString 接收 **Buffer**。
 * git.js 直接把这些 Buffer 写文件 / 读文件，所以这里必须保持同样的类型，
 * 不能图省事返回 base64 字符串。
 */
const safeStorage = {
    isEncryptionAvailable: () => process.platform === "darwin" || process.platform === "win32",

    encryptString(str) {
        const key = getKey();
        if (key.length !== 16) {
            throw new Error(`safeStorage: expected a 16-byte key, got ${key.length}`);
        }
        const iv = randomBytes(16);
        const cipher = createCipheriv("aes-128-cbc", key, iv);
        const payload = Buffer.concat([cipher.update(str, "utf8"), cipher.final()]);
        return Buffer.concat([Buffer.from("v10"), iv, payload]);
    },

    decryptString(buf) {
        // 兼容旧明文：Electron 版在解密失败时也会回退（见 git.js loadUsers 的 catch）
        if (!Buffer.isBuffer(buf)) buf = Buffer.from(buf);
        // 没有 v10 头 = 未加密的旧文件，原样返回
        if (buf.subarray(0, 3).toString() !== "v10") {
            return buf.toString("utf8");
        }
        const key = getKey();
        const iv = buf.subarray(3, 19);
        const payload = buf.subarray(19);
        // 空明文（只有头+IV）是合法情况：空用户表
        if (payload.length === 0) return "";
        const decipher = createDecipheriv("aes-128-cbc", key, iv);
        return Buffer.concat([decipher.update(payload), decipher.final()]).toString("utf8");
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
