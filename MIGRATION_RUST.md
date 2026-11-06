# GnosisStudio: Sidecar $\rightarrow$ Rust 迁移路线图

## 1. 核心准则 (The North Star)
**“能用 Rust 实现的不再写在 Sidecar 中”**

本准则定义为所有重构任务的最高优先级。在任何功能的迁移、新增或优化过程中，必须首先评估该功能是否可以用 Rust 实现。如果可以，则禁止在 Node.js Sidecar 中实现。

**最终目标：** 逐步削减 Node.js Sidecar 的职责，直到其体积缩减至零，最终彻底关掉 Node Sidecar 进程，实现纯粹的 Tauri (Rust) + Frontend 架构。

---

## 2. 当前 Sidecar 职责分析
根据 `sidecar.mjs` 的加载项，当前 Sidecar 承担了以下核心职责：

| 模块 | 职责描述 | 迁移复杂度 | 优先级 |
| :--- | :--- | :--- | :--- |
| `files.js` | 文件系统操作 (读写, 路径管理) | 低 | P0 |
| `tools/state.js` | 应用状态持久化 (窗口布局, 配置) | 低 | P0 |
| `tools/git.js` | Git 版本控制集成 | 中 | P1 |
| `python/index.js` | Python 环境管理 (uv, 依赖安装) | 中 | P1 |
| `tools/api_compat.js` | 旧版 API 兼容层 | 中 | P2 |
| `native-shim.js` | 宿主能力桥接 (Auth Window 等) | 低 | P0 |

---

## 3. 迁移架构模式

为了实现“不知不觉”的迁移，采用 **“拦截 $\rightarrow$ 代理 $\rightarrow$ 替换”** 的渐进式策略。

### 3.1 Tauri Command 模式 (目标态)
所有功能最终应转化为 Tauri 的 `#[tauri::command]`。
- **路径：** `Frontend` $\rightarrow$ `Tauri Command (Rust)` $\rightarrow$ `OS/FileSystem`。
- **优势：** 性能最高，内存占用最低，类型安全。

### 3.2 代理转发模式 (过渡态)
在迁移期间，对于尚未 Rust 化的功能，由 Rust 充当路由器。
- **路径：** `Frontend` $\rightarrow$ `Tauri Command (Rust)` $\rightarrow$ `JSON-RPC` $\rightarrow$ `Node Sidecar` $\rightarrow$ `OS`。
- **逻辑：** Rust 侧维护一个 Dispatcher，根据 method 名称判断：
  - 如果 Rust 已实现 $\rightarrow$ 直接执行并返回。
  - 如果 Rust 未实现 $\rightarrow$ 转发给 Sidecar $\rightarrow$ 等待回包 $\rightarrow$ 返回给 Frontend。

---

## 4. 分阶段执行计划

### 第一阶段：基础设施与低挂果实 (Foundation & Low-Hanging Fruit)
**目标：** 建立转发机制，迁移纯工具类函数。
- [ ] **实现 Rust 路由分发器：** 建立一个统一的 Command 接口，支持将请求动态分发至 Rust 内部或转发至 Sidecar。
- [ ] **迁移 `state.js`：** 将应用状态的读写（JSON 文件）迁移至 Rust，利用 `serde_json`。
- [ ] **迁移 `files.js`：** 将基础的文件读写、目录扫描迁移至 Rust `std::fs`。

### 第二阶段：外部工具链集成 (Toolchain Integration)
**目标：** 掌握核心外部进程控制权。
- [ ] **迁移 `python/index.js`：** 将 `uv` 等 Python 环境管理逻辑迁移至 Rust，利用 `std::process::Command` 管理子进程。
- [ ] **迁移 `git.js`：** 使用 `git2-rs` 或直接调用 `git` 命令行实现 Git 功能。

### 第三阶段：复杂业务逻辑与兼容层 (Complex Logic & Compatibility)
**目标：** 清理遗留代码，完成最后迁移。
- [ ] **迁移 `api_compat.js`：** 将旧版 API 的转换逻辑在 Rust 侧实现。
- [ ] **处理 `native-shim.js`：** 将所有宿主能力（如 OAuth 窗口）彻底改为 Tauri 插件或原生 Rust 实现。

### 第四阶段：收割 (The Harvest)
**目标：** 彻底删除 Node.js 依赖。
- [ ] **Sidecar 零调用验证：** 监控 Sidecar 日志，确保没有任何请求进入 Node 进程。
- [ ] **删除 Sidecar 启动逻辑：** 在 `main.rs` 中删除启动 Node 进程的代码。
- [ ] **移除 Node 相关依赖：** 删除 `electron/src` 及相关 package.json。

---

## 5. 风险控制与验证

1. **行为对齐验证：** 每个功能迁移后，必须对比 Rust 实现与 Node 实现的输出结果（尤其是文件路径处理和错误码）。
2. **渐进式切换：** 采用 Feature Flag 或配置文件，支持在 Rust 实现和 Sidecar 实现之间快速切换，以便快速回滚。
3. **性能监控：** 记录迁移前后的内存占用和响应时延，量化 Rust 化的收益。
