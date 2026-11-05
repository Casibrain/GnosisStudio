import { handle } from "../transport/index.js";

/**
 * API 兼容层 —— 将原 SvelteKit API 路由逻辑迁移至 Node sidecar
 * 解决 Tauri 静态部署下 /api/* 404 的问题。
 */

export const handlers = {
    plugins: {
        list: handle("plugins.list", async (evt) => {
            try {
                const resp = await fetch('https://psychopy.org/plugins.json');
                return await resp.json();
            } catch (e) {
                throw new Error(`Failed to fetch plugins: ${e.message}`);
            }
        }),
    },
    report: {
        submit: handle("report.submit", async (evt, snapshot) => {
            // 迁移自 src/routes/api/report/+server.js
            try {
                // 1. Create task
                const taskResp = await fetch("https://api.clickup.com/api/v2/list/128673336/task", {
                    method: "POST",
                    headers: {
                        "Authorization": snapshot.token,
                        "Accept": "application/json",
                        "Content-Type": "application/json"
                    },
                    body: JSON.stringify({
                        name: snapshot.title,
                        description: snapshot.description,
                        priority: snapshot.priority,
                        custom_fields: [
                            { id: "1cc82c18-79c6-470b-aa63-b39a108afe90", value: ["39244b7f-eea7-47d2-8760-418d86dc525d"] },
                            { id: "90ee49a2-01ce-49be-a3bb-c7b12160eb03", value: snapshot.email },
                            { id: "e649173f-4f1a-4275-abff-1e699962eda1", value: snapshot.version.match(/\\d+$/)?.[0] || "" }
                        ]
                    })
                });
                const data = await taskResp.json();

                // 2. Attach logs
                const logs = [
                    ["last_app_load.log", snapshot.logs.app],
                    ["liaison.log", snapshot.logs.liaison],
                    ["context.json", JSON.stringify(snapshot.context, undefined, 4)]
                ];
                for (const [name, content] of logs) {
                    await fetch(`https://api.clickup.com/api/v2/task/${data.id}/comment`, {
                        method: "POST",
                        headers: {
                            "Authorization": snapshot.token,
                            "Accept": "application/json",
                            "Content-Type": "application/json"
                        },
                        body: JSON.stringify({
                            notify_all: false,
                            comment_text: `${name}\\n---\\n${content}\\n`
                        })
                    });
                }
                return data;
            } catch (e) {
                throw new Error(`Report submission failed: ${e.message}`);
            }
        }),
    }
};
