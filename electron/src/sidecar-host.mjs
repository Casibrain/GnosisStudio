/**
 * 宿主（Rust）桥接。
 *
 * 架构（见 docs/ipc-contract.md §3）：
 *   前端 invoke -> Rust node_call -> sidecar stdin
 *   sidecar stdout -> Rust -> 前端
 *
 * 反向通路（sidecar 需要宿主能力）：
 *   sidecar 往 stdout 写 {"id":"host_N","method":"psychojs_run",...}
 *   Rust 识别出 host_ 前缀后自己执行，把结果写回 stdin
 *
 * 所以这里 hostRequest() 只需要"写一个带 host_ 前缀的 id 的请求，然后等"。
 * 进程管理完全在 Rust 侧，sidecar 不持有宿主进程句柄。
 *
 * 与 Electron 版的区别：Electron 版里 BrowserWindow 是 Node 自己的对象；
 * 这里 sidecar 是 Rust 的子进程，拿不到 BrowserWindow，只能委托。
 */

const pending = new Map();
let nextId = 1;

/**
 * 向宿主发请求，等它回同一个 id。
 * @param {string} method
 * @param {any[]} params
 * @param {number} [timeout]
 */
export function hostRequest(method, params = [], timeout = 180000) {
    const id = `host_${nextId++}`;
    return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        // 走 stdout：Rust 的 read_loop 会区分 host_ 前缀和普通响应
        process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
        setTimeout(() => {
            if (pending.has(id)) {
                pending.delete(id);
                reject(new Error(`hostRequest timeout: ${method}`));
            }
        }, timeout);
    });
}

/**
 * 宿主回的结果从 stdin 进来，由 transport 识别 host_ id 后回调这里。
 * @param {string} id
 * @param {any} result
 * @param {Error|null} error
 */
export function hostRespond(id, result, error) {
    const p = pending.get(id);
    if (!p) return;
    pending.delete(id);
    error ? p.reject(new Error(error.message ?? String(error))) : p.resolve(result);
}
