/**
 * Tauri-Electron Bridge
 *
 * This bridge mimics the Electron contextBridge objects (window.electron, window.python, window.git)
 * by forwarding all calls to the Tauri `node_call` command.
 *
 * Mapping: window.python.uv.exists() -> invoke('node_call', { channel: 'python.uv.exists', params: [] })
 */

async function getInvoke() {
    if (window.__TAURI__?.core?.invoke) return window.__TAURI__.core.invoke;
    if (window.__TAURI__?.invoke) return window.__TAURI__.invoke;

    try {
        const { invoke } = await import('@tauri-apps/api');
        return invoke;
    } catch (e) {
        console.error("[Bridge] Failed to resolve invoke:", e);
        return async () => { throw new Error("Tauri invoke not available"); };
    }
}

function createHybridProxy(rootName, path = []) {
    return new Proxy(() => {}, {
        get(target, prop) {
            if (typeof prop === 'symbol' || prop === 'then') return undefined;
            if (prop === '__isProxy') return true; // 调试标记
            return createHybridProxy(rootName, [...path, prop]);
        },
        apply(target, thisArg, args) {
            return (async () => {
                const invoke = await getInvoke();
                const channel = [rootName, ...path].join('.');
                try {
                    return await invoke('node_call', {
                        channel: channel,
                        params: args
                    });
                } catch (err) {
                    console.error(`[Bridge] Error calling ${channel}:`, err);
                    throw err;
                }
            })();
        }
    });
}

export function initTauriBridge() {
    console.log("[Bridge] Force-locking Tauri-Electron IPC Bridge...");

    const roots = ['electron', 'python', 'git'];

    roots.forEach(root => {
        try {
            Object.defineProperty(window, root, {
                get: () => createHybridProxy(root),
                set: () => {
                    console.warn(`[Bridge] Blocked attempt to overwrite window.${root}`);
                    return false;
                },
                configurable: false, // 禁止删除或重新定义
                enumerable: true
            });
            console.log(`[Bridge] Locked window.${root}`);
        } catch (e) {
            console.error(`[Bridge] Failed to lock window.${root}:`, e);
        }
    });

    console.log("[Bridge] Bridge forced-lock complete.");
}
