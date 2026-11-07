//! Python 环境管理 (uv 封装)
//! 迁移自 `electron/src/python/uv.js`

use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::fs::File;
use std::io::copy;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use reqwest::blocking::Client;

/// `normalize_version` 的纯逻辑部分，做成自由函数以便单测
/// （AppHandle 造不出来，但版本解析正是最容易出错、也最该锁住的部分）。
///
/// app_version 只在标识为 `"app"` / 空时用到。
pub fn normalize_version(version: &str, app_version: &str) -> String {
    if version.is_empty() || version == "app" {
        return app_version.to_string();
    }
    // 对应 Node 的 version.match(/\d+\.\d+\.\d+/)[0]：
    // 按"非版本字符"切段，找回第一段形如 x.y.z 的前三段数字
    for chunk in version.split(|c: char| !c.is_ascii_digit() && c != '.') {
        let parts: Vec<&str> = chunk.split('.').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 3 && parts[..3].iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
            return parts[..3].join(".");
        }
    }
    version.to_string()
}

/// 目录名是否形如 "3.10" / "2024.1"（两段数字）。
/// Node 版用 /^\d+\.\d+$/ 判断，命中就还原成 "x.y.*" 形式。
fn is_two_part_version(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() == 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

pub struct UvManager {
    pub root_folder: PathBuf,
    pub executable: String,
}

impl UvManager {
    /// 初始化 UvManager，自动探测 uv 可执行文件路径
    pub fn new(app: &AppHandle) -> Self {
        let app_data = app.path().app_data_dir().expect("failed to get app data dir");
        let root_folder = app_data.join("psychopy4").join("uv");
        
        // 优先级：环境变量 > 默认安装路径 > 系统 PATH
        let executable = std::env::var("GNOSTIS_UV").unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                // Windows 默认路径 (通常在 %USERPROFILE%\.cargo\bin 或 %LOCALAPPDATA%\uv\bin)
                let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
                let path = PathBuf::from(local_app_data).join("uv").join("bin").join("uv.exe");
                if path.exists() { return path.to_string_lossy().to_string(); }
                "uv".to_string()
            } else {
                // Unix 默认路径
                let home = std::env::var("HOME").unwrap_or_default();
                let path = PathBuf::from(home).join(".cargo").join("bin").join("uv");
                if path.exists() { return path.to_string_lossy().to_string(); }
                "uv".to_string()
            }
        });

        Self {
            root_folder,
            executable,
        }
    }

    /// 执行 uv 命令并返回输出
    fn run_uv(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.executable)
            .args(args)
            .output()
            .map_err(|e| format!("failed to execute uv: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("uv error: {err}"))
        }
    }

    pub fn folder(&self) -> Value {
        json!(self.root_folder.to_string_lossy())
    }

    pub fn executable(&self) -> Value {
        json!(self.executable)
    }

    pub fn system_info(&self) -> Value {
        json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "env": "production"
        })
    }

    pub fn exists(&self) -> Value {
        let check = Command::new(&self.executable)
            .arg("--version")
            .output();
        
        match check {
            Ok(out) if out.status.success() => json!(true),
            _ => json!(false),
        }
    }

    pub fn find_python(&self, version: String) -> Result<Value, String> {
        // uv python find <version>
        let res = self.run_uv(&["python", "find", &version])?;
        Ok(json!(res))
    }

    /// 列出已有的 Python 环境（对应 uv.js:getEnvironments）。
    ///
    /// 必须返回对象数组 `[{executable, psychopyVersion, pythonVersion}]`：
    /// VenvChooser.svelte 遍历时直接读 `env.psychopyVersion` / `env.executable`。
    /// 早先这里把 `uv python list` 的原始文本 json! 出去，前端拿到的全是
    /// undefined —— 下拉框永远空着，且不报错。
    ///
    /// 扫描方式也与 Node 版一致：扫 pyFolder 的子目录逐个确认有没有 python。
    /// 不用 `uv python list`，它列的是 uv 自己的解释器缓存，不是我们的 venv。
    pub fn get_environments(&self) -> Result<Value, String> {
        let py_folder = self.py_folder();
        let mut out: Vec<Value> = Vec::new();

        // 没有目录就是没有环境，不是错误
        let Ok(entries) = fs::read_dir(&py_folder) else {
            return Ok(json!(out));
        };

        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(|s| s.to_string()) else {
                continue;
            };
            // 目录名 2024.1 -> 版本 2024.1.*（Node 版同样 "reinstate .* syntax"）
            let psychopy_version = if is_two_part_version(&name) {
                format!("{name}.*")
            } else {
                name.clone()
            };

            let folder = entry.path();
            let Ok(found) = self.run_uv(&["python", "find", &folder.to_string_lossy()]) else {
                continue;
            };
            if found.is_empty() {
                continue;
            }

            // python --version 输出 "Python 3.10.x"
            let python_version = Command::new(&found)
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| {
                    let text = if o.stdout.is_empty() {
                        String::from_utf8_lossy(&o.stderr).to_string()
                    } else {
                        String::from_utf8_lossy(&o.stdout).to_string()
                    };
                    text.split_whitespace()
                        .find(|w| w.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
                        .map(|w| {
                            let parts: Vec<&str> = w.split('.').collect();
                            if parts.len() >= 2 {
                                format!("{}.{}", parts[0], parts[1])
                            } else {
                                w.to_string()
                            }
                        })
                })
                .unwrap_or_default();

            out.push(json!({
                "executable": found,
                "psychopyVersion": psychopy_version,
                "pythonVersion": python_version,
            }));
        }

        Ok(json!(out))
    }

    pub fn install(&self) -> Result<Value, String> {
        // 实际安装逻辑通常涉及复杂的 curl | sh 脚本
        // 这里先实现一个基础检查，若不存在则提示
        if self.exists().as_bool().unwrap_or(false) {
            Ok(json!("uv is already installed"))
        } else {
            // 生产环境下应调用对应的安装脚本
            Err("uv is not installed. Please install it manually or use the setup wizard.".to_string())
        }
    }

    pub fn install_package(&self, executable: &str, package: &str, version: Option<String>) -> Result<Value, String> {
        let mut args = vec!["pip", "install"];
        
        let pkg_spec = if let Some(v) = version {
            format!("{}=={}", package, v)
        } else {
            package.to_string()
        };
        
        args.push(&pkg_spec);
        args.push("--python");
        args.push(executable);

        let output = Command::new(&self.executable)
            .args(&args)
            .output()
            .map_err(|e| format!("failed to execute uv pip install: {e}"))?;

        if output.status.success() {
            Ok(json!(true))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("uv pip install error: {err}"))
        }
    }

    pub fn uninstall_package(&self, executable: &str, package: &str) -> Result<Value, String> {
        let args = ["pip", "uninstall", package, "--python", executable];

        let output = Command::new(&self.executable)
            .args(&args)
            .output()
            .map_err(|e| format!("failed to execute uv pip uninstall: {e}"))?;

        if output.status.success() {
            Ok(json!(true))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("uv pip uninstall error: {err}"))
        }
    }

    /// 同步本地项目与环境 (迁移自 venv.js:syncProject)
    pub fn sync_project(&self, executable: &str, folder: &Path) -> Result<Value, String> {
        let args = ["sync", "--directory", folder.to_str().unwrap_or(""), "--python", executable, "--inexact"];
        
        let env_path = std::path::Path::new(executable)
            .parent().unwrap_or(Path::new("."))
            .parent().unwrap_or(Path::new("."));

        let output = Command::new(&self.executable)
            .args(&args)
            .env("UV_PROJECT_ENVIRONMENT", env_path)
            .output()
            .map_err(|e| format!("failed to sync project: {e}"))?;

        if output.status.success() {
            Ok(json!(true))
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Python 环境存放目录（对应 uv.js 的 pyFolder）
    fn py_folder(&self) -> PathBuf {
        self.root_folder.join(".python")
    }

    /// 把前端传来的版本标识规范成真实目录名。
    ///
    /// 对应 uv.js:getVenv 里的两步（resolvePackageVersion + 剥离 extras）：
    ///   1. `"app"` / 空 -> 当前 app 版本。前端 VenvChooser.svelte:9 传的就是 "app"，
    ///      不替换会拼成 `.python/app` 而永远找不到
    ///   2. 含 extras 的字符串（如 `"2024.1.0.dev1+abc"`）-> 取前三段数字。
    ///      目录名是 `2024.1.0`，带 extras 会拼错
    ///
    /// 通配符（`2024.1.*`）不在这里解析：那要联网查 PyPI，由 Node 侧
    /// resolvePackageVersion 先处理，Rust 只接已解析的值。
    fn normalize_version(&self, app: &AppHandle, version: &str) -> String {
        normalize_version(version, &app.package_info().version.to_string())
    }

    /// 取某个 PsychoPy 版本 venv 里的 Python 可执行文件路径。
    pub fn get_venv_executable_ctx(
        &self,
        app: &AppHandle,
        version: &str,
    ) -> Result<Value, String> {
        let version = self.normalize_version(app, version);
        let folder = self.py_folder().join(&version);
        if let Ok(found) = self.run_uv(&["python", "find", &folder.to_string_lossy()]) {
            if !found.is_empty() {
                return Ok(json!(found));
            }
        }
        // uv 没找到时按 venv 布局猜：Windows 在 Scripts/ 下，其余在 bin/ 下
        let exe = if cfg!(target_os = "windows") {
            folder.join("Scripts").join("python.exe")
        } else {
            folder.join("bin").join("python")
        };
        if exe.exists() {
            Ok(json!(exe.to_string_lossy()))
        } else {
            Err(format!("no python environment found for version {version}"))
        }
    }

    ///
    /// 列出 venv 里已安装的包（对应 venv.js:getPackages）。
    ///
    /// 返回 `{包名: 版本}` 对象，与 Node 版形状一致——前端直接用 `in` 判断。
    pub fn get_packages(&self, executable: &str) -> Result<Value, String> {
        let res = self.run_uv(&["pip", "list", "--python", executable, "--format", "json"])?;
        let parsed: Vec<Value> = serde_json::from_str(&res)
            .map_err(|e| format!("failed to parse pip list output: {e}"))?;

        let mut out = serde_json::Map::new();
        for pkg in parsed {
            if let (Some(name), Some(version)) = (pkg.get("name").and_then(|v| v.as_str()), pkg.get("version")) {
                out.insert(name.to_string(), version.clone());
            }
        }
        Ok(Value::Object(out))
    }

    /// 完整初始化虚拟环境 (迁移自 venv.js:setup)
    pub fn setup_venv(&self, python_version: &str, psychopy_version: &str, prerelease: Option<&str>) -> Result<Value, String> {
        let executable = self.find_python(python_version.to_string())?.as_str().ok_or("failed to find python executable")?.to_string();
        
        let base_deps = [
            ("liaison-py", "liaison-py[websocket]"),
            ("esprima", "esprima"),
            ("dukpy", "dukpy"),
            ("javascripthon", "javascripthon"),
        ];

        for (_, cmd) in base_deps {
            self.install_package(&executable, cmd, None)?;
        }

        let (source, module_name) = if psychopy_version == "dev" || prerelease == Some("dev") {
            ("https://github.com/psychopy/psychopy/archive/refs/heads/dev.zip".to_string(), "psychopy-dev")
        } else if prerelease == Some("release") {
            ("https://github.com/psychopy/psychopy/archive/refs/heads/release.zip".to_string(), "psychopy-release")
        } else {
            let pkg = if psychopy_version.starts_with("2026") { "psychopy" } else { "psychopy-lib" };
            (format!("https://github.com/psychopy/{}/archive/refs/tags/{}.zip", pkg, psychopy_version), "psychopy-lib")
        };

        let ppy_folder = self.root_folder.join(".psychopy").join(psychopy_version);
        fs::create_dir_all(&ppy_folder).map_err(|e| e.to_string())?;

        self.download_folder(&source, &ppy_folder, Some(module_name))?;

        let module_folder = ppy_folder.join(module_name);
        self.sync_project(&executable, &module_folder)?;

        Ok(json!(true))
    }

    /// 下载并解压资源文件夹 (迁移自 files.js:downloadFolder)
    pub fn download_folder(&self, url: &str, target: &Path, name: Option<&str>) -> Result<(), String> {
        let client = Client::new();
        let response = client.get(url).send().map_err(|e| e.to_string())?;
        
        if !response.status().is_success() {
            return Err(format!("download failed: {}", response.status()));
        }

        let filename = url.split('/').last().unwrap_or("download.zip");
        let zip_path = target.join(filename);
        let mut file = File::create(&zip_path).map_err(|e| e.to_string())?;
        
        let mut contents = response;
        copy(&mut contents, &mut file).map_err(|e| e.to_string())?;

        let file = File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        let extract_path = target.join(name.unwrap_or("extracted"));
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            // enclosed_name 会剔除 `..` 等越界路径，避免 zip slip
            let entry_name = file
                .enclosed_name()
                .ok_or_else(|| format!("unsafe path in zip: {}", file.name()))?;
            let outpath = extract_path.join(entry_name);
            if file.is_dir() {
                fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }

        fs::remove_file(zip_path).ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 前端 VenvChooser.svelte:9 传 "app"。Node 版把 "app" 换成真实版本，
    /// Rust 版若直接用它拼目录，会去找 `.python/app` —— 用户所有环境"消失"。
    #[test]
    fn app_keyword_resolves_to_app_version() {
        assert_eq!(normalize_version("app", "2024.1.0"), "2024.1.0");
        assert_eq!(normalize_version("", "2024.1.0"), "2024.1.0");
    }

    /// getVenv 会剥离 extras：目录名是 2024.1.0，不是 2024.1.0.dev1+abc
    #[test]
    fn extras_are_stripped_to_three_parts() {
        assert_eq!(normalize_version("2024.1.0.dev1+abc", "x"), "2024.1.0");
        assert_eq!(normalize_version("v2024.1.0", "x"), "2024.1.0");
    }

    /// 已经是干净版本号时原样返回（不能把 2024.1.0 弄成 2024.1）
    #[test]
    fn clean_versions_pass_through() {
        assert_eq!(normalize_version("2024.1.0", "x"), "2024.1.0");
        assert_eq!(normalize_version("2023.2.3", "x"), "2023.2.3");
    }

    /// 两段数字（如 "3.10"）不该被当作三段版本截断
    #[test]
    fn two_part_versions_are_not_truncated() {
        assert_eq!(normalize_version("3.10", "x"), "3.10");
    }

    /// 目录名 2024.1 要还原成 2024.1.*，与 Node 的 "reinstate .* syntax" 一致
    #[test]
    fn two_part_folder_names_get_star_suffix() {
        assert!(is_two_part_version("2024.1"));
        assert!(is_two_part_version("3.10"));
        assert!(!is_two_part_version("2024.1.0"));
        assert!(!is_two_part_version("app"));
        assert!(!is_two_part_version("2024..1"));
    }
}
