/**
 * JSON-RPC 传输后端 —— Node sidecar 用（M3 接线）。
 *
 * 设计要点：
 *   - 去掉 Electron 注入的 evt 首参，业务 handler 只收真实参数
 *   - handler 抛异常 / reject -> JSON-RPC error，message 为原始字符串
 *     （不要包成 {error: ...}，见 docs/ipc-contract.md §3.1）
 *   - sendEvent 保留层级 tag 语义，并通过订阅推送
 *
 * 协议：JSON-RPC 2.0 over stdio（一行一帧，NDJSON）
 *   请求  {"jsonrpc":"2.0","id":1,"method":"python.uv.install","params":[...]}
 *   响应  {"jsonrpc":"2.0","id":1,"result":...}
 *         {"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"..."}}
 *   事件  {"jsonrpc":"2.0","method":"event","params":{"tag":"uv","data":...}}
 */

import { createInterface } from "node:readline";

const ERR_INTERNAL = -32000;
const ERR_METHOD_NOT_FOUND = -32601;

/**
 * @param {{input?: NodeJS.ReadableStream, output?: NodeJS.WritableStream}} options
 */
export function create(options = {}) {
    const input = options.input ?? process.stdin;
    const output = options.output ?? process.stdout;

    /** @type {Map<string, Function>} */
    const handlers = new Map();
    /** @type {Set<string>} */
    const syncChannels = new Set();

    function write(obj) {
        output.write(JSON.stringify(obj) + "\n");
    }

    function sendEvent(tag, message) {
        // 层级广播：a:b:c -> a:b:c, a:b, a
        const channels = [];
        if (tag) {
            channels.push(tag);
            let t = tag;
            while (t?.includes?.(":")) {
                t = t.substring(0, t.lastIndexOf(":"));
                channels.push(t);
            }
        }
        for (const channel of channels) {
            write({ jsonrpc: "2.0", method: "event", params: { tag: channel, data: message } });
        }
    }

    async function dispatch(msg) {
        const { id, method, params = [], sender = null } = msg;
        const fn = handlers.get(method);
        if (!fn) {
            write({ jsonrpc: "2.0", id, error: { code: ERR_METHOD_NOT_FOUND, message: `No handler for ${method}` } });
            return;
        }
        try {
            // 构造与 Electron 同形的 evt：保留 sender.id 语义（这里是窗口 label）
            const evt = { _transport: "jsonrpc", sender: { id: sender } };
            const result = await fn(evt, ...(Array.isArray(params) ? params : [params]));
            write({ jsonrpc: "2.0", id, result: result ?? null });
        } catch (err) {
            // 保持裸字符串语义，前端 handleError 已加 typeof 守卫
            write({
                jsonrpc: "2.0",
                id,
                error: { code: ERR_INTERNAL, message: err instanceof Error ? err.message : String(err) },
            });
        }
    }

    function start() {
        const rl = createInterface({ input, crlfDelay: Infinity });
        rl.on("line", line => {
            const text = line.trim();
            if (!text) return;
            let msg;
            try {
                msg = JSON.parse(text);
            } catch {
                write({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "Parse error" } });
                return;
            }
            dispatch(msg);
        });
        return rl;
    }

    return {
        handle(channel, fn) {
            handlers.set(channel, fn);
        },
        on(channel, fn) {
            // 同步通道在 sidecar 下不支持 —— D6 已决定改异步
            syncChannels.add(channel);
            handlers.set(channel, fn);
        },
        sendEvent,
        listHandlers() {
            return { handle: [...handlers.keys()], on: [...syncChannels] };
        },
        start,
    };
}
