//! Python 环境管理 (uv 封装)
//! 迁移自 `electron/src/python/uv.js`

use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

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

    pub fn get_environments(&self) -> Result<Value, String> {
        // uv python list
        let res = self.run_uv(&["python", "list"])?;
        Ok(json!(res))
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
            ("https://github.com/psychopy/psychopy/archive/refs/heads/dev.zip", "psychopy-dev")
        } else if prerelease == Some("release") {
            ("https://github.com/psychopy/psychopy/archive/refs/heads/release.zip", "psychopy-release")
        } else {
            let pkg = if psychopy_version.starts_with("2026") { "psychopy" } else { "psychopy-lib" };
            let url = format!("https://github.com/psychopy/{}/archive/refs/tags/{}.zip", pkg, psychopy_version);
            (url.as_str(), "psychopy-lib")
        };

        let ppy_folder = self.root_folder.join(".psychopy").join(psychopy_version);
        fs::create_dir_all(&ppy_folder).map_err(|e| e.to_string())?;

        self.download_folder(source, &ppy_folder, Some(module_name))?;

        let module_folder = ppy_folder.join(module_name);
        self.sync_project(&executable, &module_folder)?;

        Ok(json!(true))
    }

    /// 下载并解压资源文件夹 (迁移自 files.js:downloadFolder)
    pub fn download_folder(&self, url: &str, target: &Path, name: Option<&str>) -> Result<(), String> {
        let client = Client::new();
        let mut response = client.get(url).send().map_err(|e| e.to_string())?;
        
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
            let outpath = extract_path.join(file.m_name());
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
