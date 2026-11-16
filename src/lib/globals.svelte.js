import { initTauriBridge } from './tauri-bridge.js';

// 必须在任何变量定义之前初始化，确保 window.electron 已被代理
initTauriBridge();

// if running from electron, these will be set by preload.js, otherwise will be undefined
export var electron = $state(window.electron)
export var git = $state(window.git)
export var python = $state(window.python)
// global stores for pavlovia and hardware stuff
export var devices = $state({})
export var projects = $state({})
export var users = $state({})
// global stores for UI stuff
export var panels = $state([])