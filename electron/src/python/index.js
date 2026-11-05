import { app } from "electron";
import { handle } from "../transport/index.js";
import { uv } from "./uv.js";
import { venvs, getVenv } from "./venv.js";
import { resolvePackageVersion } from "./utils.js";
import { Liaison, getLiaison } from "./liaison.js";
import { PythonShell } from "./shell.js";
import { PythonScript } from "./script.js";
import { run as psychojsRun, getPsychoJSServer } from "./psychojs.js";


// make sure all Python instances are killed on quit
app.on("quit", (evt, code) => {
  // close python
  for (let venv of Object.values(venvs)) {
    venv.killAll()
  }
})
// register all Python handlers
export const handlers = {
    liaison: {
        start: handle("python.liaison.start", async (evt, venv) => (await getLiaison(venv)).start()),
        stop: handle("python.liaison.stop", async (evt, venv) => (await getLiaison(venv)).stop()),
        send: handle("python.liaison.send", async (evt, venv, message, timeout) => (await getLiaison(venv)).send(message, timeout)),
        started: handle("python.liaison.started", async (evt, venv) => (await getLiaison(venv)).started),
        ready: handle("python.liaison.ready", async (evt, venv) => await (await getLiaison(venv)).ready.promise)
    },
    venv: {
        setup: handle("python.venv.setup", async (evt, venv, prerelease=undefined) => (await getVenv(venv)).setup(prerelease)),
        executable: handle("python.venv.executable", async (evt, venv) => (await getVenv(venv)).executable),
        installPackage: handle("python.venv.installPackage", async (evt, venv, name, version=undefined) => (await getVenv(venv)).installPackage(name, version)),
        uninstallPackage: handle("python.venv.uninstallPackage", async (evt, venv, name) => (await getVenv(venv)).uninstallPackage(name)),
        getPackages: handle("python.venv.getPackages", async (evt, venv) => (await getVenv(venv)).getPackages()),
        getPackageDetails: handle("python.venv.getPackageDetails", async (evt, venv, name) => (await getVenv(venv)).getPackageDetails(name)),
        hasGIL: handle("python.venv.hasGIL", async (evt, venv) => (await getVenv(venv)).hasGIL())
    },
    uv: {
        folder: handle("python.uv.folder", (evt) => uv.folder),
        executable: handle("python.uv.executable", (evt) => uv.executable),
        systemInfo: handle("python.uv.systemInfo", (evt) => uv.systemInfo),
        findDirectory: handle("python.uv.findDirectory", (evt, option) => uv.findDirectory(option)),
        setDirectory: handle("python.uv.setDirectory", (evt, option) => uv.setDirectory(option)),
        exists: handle("python.uv.exists", (evt) => uv.exists()),
        needsUpdate: handle("python.uv.needsUpdate", (evt) => uv.needsUpdate()),
        resolvePackageVersion: handle("python.uv.resolvePackageVersion", (evt, version, pipname) => resolvePackageVersion(version, pipname)),
        install: handle("python.uv.install", (evt) => uv.install()),
        makeExecutable: handle("python.uv.makeExecutable", (evt, psychopyVersion, pythonVersion) => uv.makeExecutable(psychopyVersion, pythonVersion)),
        findPython: handle("python.uv.findPython", (evt, version) => uv.findPython(version)),
        getEnvironments: handle("python.uv.getEnvironments", (evt) => uv.getEnvironments())
    },
    shell: {
        list: handle("python.shell.list", async (evt, venv) => Object.keys((await getVenv(venv)).shells)),
        send: handle("python.shell.send", async (evt, venv, id, msg) => (await getVenv(venv)).shells[id].send(msg)),
        open: handle("python.shell.open", async (evt, venv) => {
            let shell = new PythonShell(await getVenv(venv))
            shell.start()

            return shell.id
        }),
        close: handle("python.shell.close", async (evt, venv, id) => (await getVenv(venv)).shells[id].close())
    },
    scripts: {
        run: handle("python.scripts.run", async (evt, venv, file, ...args) => {
            let script = new PythonScript(await getVenv(venv), file, args);
            return script.start()
        }),
        wait: handle("python.scripts.wait", async (evt, venv, id) => (await getVenv(venv)).scripts[id]?.wait()),
        stop: handle("python.scripts.stop", async (evt, venv, id) => (await getVenv(venv)).scripts[id]?.stop())
    },
    psychojs: {
        // M7：转发到 Rust 宿主（真 HTTP 静态服务 + 专用窗口）
        run: handle("python.psychojs.run", async (evt, cwd, params={}) => await psychojsRun(cwd, params)),
        stop: handle("python.psychojs.stop", (evt, address) => getPsychoJSServer(address).stop()),
    }
}