import fs from "fs";
import path from "path";
import { app } from "electron";
import { handle, senderId } from "../transport/index.js";


// path to JSON file with details of last app load
const lastStateFile = path.join(
    app.getPath("appData"), "psychopy4", "last_app_state.json"
);

// object storing details of current app state
export var state = {
  frames: {},
  profiles: {}
}

/**
 * Store details of a new frame
 * 
 * @param {number} id Window ID of the new frame
 */
export function newFrame(id, view) {
    state.frames[id] = {
        pos: [null, null],
        size: [null, null],
        maximized: false,
        files: [],
        view: view,
    }
}

/**
 * Remove details of a closed frame
 * 
 * @param {number} id Window ID of the closed frame
 */
export function closeFrame(id, view) {
    delete state.frames[id]
}

/**
 * Update the details of a frame
 * 
 * @param {*} id Window ID of the frame whose details to update
 * @param {object} details Object with details to update - non-present attributes won't be updated
 */
export function updateFrame(id, details) {
    // key 不存在时自建。
    //
    // Electron 版由 frames.js 在开窗时调 newFrame(win.webContents.id) 预注册；
    // Tauri/sidecar 下窗口由 Rust 管，业务侧收不到开窗通知，只会在前端调用
    // updateFrame 时第一次见到这个 key。若不自建，Object.assign(undefined, ...)
    // 会抛 "Cannot convert undefined or null to object"（已实测），窗口状态全丢。
    //
    // 依据 details 里的 files 判断是 builder 还是 runner，与 newFrame 的 view 语义一致。
    if (!state.frames[id]) {
        newFrame(id, id)
    }
    Object.assign(
        state.frames[id],
        details
    )
}

/**
 * Store the last received value for a set of element profiles, and save to file
 * 
 * @param {string} key Which set of profiles these are (e.g. "components", "loops", "devices")
 * @param {object} value Profiles as received from Python
 */
export function setProfiles(key, value) {
    state.profiles[key] = value
    saveState()
}

/**
 * Get the last received values for each set of element profiles
 * 
 * @returns {object} Object of profiles keyed by set (e.g. "components", "loops", "devices")
 */
export async function getProfiles() {
    // 异步化（D6）：Tauri 没有同步 invoke，前端 profiles.svelte.js 已改成 await
    return state.profiles
}

/**
 * Save current app state to JSON file
 */
export function saveState() {
    fs.writeFileSync(
        lastStateFile,
        JSON.stringify(state, undefined, 4)
    )
}

// make sure last app load file exists
if (!fs.existsSync(lastStateFile)) {
    saveState()
}

// load last state from file
export var lastState = {
    frames: {},
    profiles: {}
};
try {
    Object.assign(
        lastState,
        JSON.parse(
            fs.readFileSync(lastStateFile)
        )
    )
} catch (err) {
    console.error("Failed to load previous app state.", err)
}
// carry over last received profiles, so they're preserved even if not received again this session
Object.assign(state.profiles, lastState.profiles)


export const handlers = {
    updateFrame: handle("electron.state.updateFrame", (evt, details) => updateFrame(senderId(evt), details)),
    setProfiles: handle("electron.state.setProfiles", (evt, key, value) => setProfiles(key, value)),
    // 同步 → 异步（D6）：返回 promise，不再写 evt.returnValue
    getProfiles: handle("electron.state.getProfiles", async () => await getProfiles()),
}


export default {
    state: state,
    newFrame: newFrame,
    updateFrame: updateFrame,
    closeFrame: closeFrame,
    setProfiles: setProfiles,
    getProfiles: getProfiles,
    saveState: saveState
}