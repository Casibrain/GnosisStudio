/**
 * 宿主（Rust）桥接。
 *
 * 前端只有 "前端 -> sidecar" 一条请求路。但少数事情 sidecar 自己做不了：
 * 开 OAuth 登录窗口需要真实的 BrowserWindow + cookie 管理。
 *
 * 实现：hostRequest() 把请求写成普通 JSON-RPC，只是方法名以 host 开头，
 * 由 Rust 侧在 stdio 循环里拦截处理并回同一个 id。宿主进程管理（M8）
 * 落地后这条通路会被正式接管，业务代码不需要再改。
 *
 * 当前阶段：宿主进程尚未启动（M8 范围），调用会明确报错而不是静默失败。
 */

/** 宿主进程（Rust 编译出的可执行文件） */
let host = null;
const pending = new Map();
let nextId = 1;

/**
 * 向宿主发请求，等它回同一个 id。
 * @param {string} method
 * @param {any[]} params
 * @param {number} [timeout]
 */
export function hostRequest(method, params = [], timeout = 180000) {
    if (!host) {
        throw new Error(
            `host not running: ${method} requires the Rust host process; ` +
            "sidecar process management lands in M8"
        );
    }

    const id = `host_${nextId++}`;
    return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        host.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
        setTimeout(() => {
            if (pending.has(id)) {
                pending.delete(id);
                reject(new Error(`hostRequest timeout: ${method}`));
            }
        }, timeout);
    });
}

export function stopHost() {
    if (host) { host.kill(); host = null; }
}
