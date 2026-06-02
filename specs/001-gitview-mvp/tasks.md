---
description: "GitView V1 MVP — 跨平台 Git 可视化客户端任务清单"
---

# Tasks: GitView V1 MVP — 跨平台 Git 可视化客户端

**Input**: Design documents from `/specs/001-gitview-mvp/`

**Prerequisites**: plan.md ✓, spec.md ✓

**Tests**: 本计划包含**适量集成测试 + 关键单元测试**（用户确认）。不采用严格 TDD；
测试任务标注 `[T]` 但不强制"先于实现"。

**Organization**: 任务按 User Story 分组以支持每个 Story 独立实现与测试。
P1（US1-US3）构成 MVP 最小闭环；P2 完善常用工作流；P3 提供支撑性能力。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可与同 Phase 其他 [P] 任务并行（不同文件且无依赖）
- **[Story]**: 标识所属 User Story（仅在 User Story Phase 中出现）
- 所有任务包含**目标 / 实现要点 / 验收标准**三段

## Path Conventions

- 后端 Rust：`src-tauri/src/...`
- 前端 Vue/TS：`src/...`
- 集成测试：`tests/integration/...`（Rust），`tests/frontend/...`（Vitest）
- 项目辅助脚本：`scripts/...`

所有路径均从仓库根目录 `/Volumes/venus/code/git-view/` 计算。

---

## Phase 1: Setup（共享基础设施初始化）

**Purpose**: 项目骨架与开发环境搭建，落实宪法 Principle I（代码质量工具链）与
Principle II（注释比例检查工具）。

- [x] T001 创建 Tauri 2 + Vue 3 + TypeScript 项目骨架于仓库根目录

  **目标**：使用 `npm create tauri-app@latest -- --template vue-ts` 或手动初始化，
  生成 `package.json`、`tsconfig.json`、`vite.config.ts`、`src-tauri/Cargo.toml`、
  `src-tauri/tauri.conf.json`、`src-tauri/build.rs`、`src-tauri/src/main.rs`、
  `src/main.ts`、`src/App.vue` 等核心文件。

  **实现要点**：
  - Tauri 版本锁定 2.x（最新稳定版）
  - Vue 锁定 3.4+，TypeScript 锁定 5.x，Vite 锁定 5.x
  - `tauri.conf.json` 中 `productName="GitView"`、`identifier="com.gitview.app"`
  - 启用 `tauri-plugin-fs`、`tauri-plugin-dialog`、`tauri-plugin-shell`、
    `tauri-plugin-os` 插件并在 `Cargo.toml` 中声明
  - 设置 `tauri.conf.json` 的 `app.windows[0]` 默认尺寸 1280x800

  **验收标准**：`npm run tauri dev` 可在本机启动应用窗口；`cargo build` 在
  `src-tauri/` 目录通过；`tsc --noEmit` 在仓库根目录通过。

- [x] T002 [P] 安装并配置 Element Plus 与 Pinia 与 Vue Router

  **目标**：在 `src/main.ts` 中注册 Element Plus（按需引入）、Pinia、Vue Router。

  **实现要点**：
  - `npm install element-plus pinia vue-router@4`
  - 安装 `unplugin-vue-components`、`unplugin-auto-import` 实现 Element Plus 按需引入
  - 在 `vite.config.ts` 中配置上述插件的 `resolvers: [ElementPlusResolver()]`
  - `src/main.ts` 中：`app.use(createPinia()).use(router).mount('#app')`
  - 创建 `src/router/index.ts` 定义 7 个页面路由占位

  **验收标准**：启动应用后侧边栏导航可点击切换 7 个空页面；浏览器/控制台无组件未注册警告。

- [x] T003 [P] 配置 Rust 后端核心依赖于 `src-tauri/Cargo.toml`

  **目标**：在 `Cargo.toml` 中添加 V1 MVP 所需全部 Rust 依赖。

  **实现要点**：在 `[dependencies]` 中添加：
  ```toml
  tokio = { version = "1", features = ["full"] }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls", "rustls-tls-native-roots"] }
  rusqlite = { version = "0.31", features = ["bundled", "chrono"] }
  keyring = "2"
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  tracing-appender = "0.2"
  thiserror = "1"
  anyhow = "1"
  uuid = { version = "1", features = ["v4", "serde"] }
  chrono = { version = "0.4", features = ["serde"] }
  dunce = "1"
  url = "2"
  dirs = "5"
  ```

  **验收标准**：`cargo build` 成功完成；`cargo tree` 显示全部依赖且无版本冲突。

- [x] T004 [P] 配置 TypeScript / Vue 格式化与静态分析工具

  **目标**：满足宪法 Principle I（代码质量门禁）。

  **实现要点**：
  - `npm install -D prettier eslint @typescript-eslint/parser @typescript-eslint/eslint-plugin eslint-plugin-vue`
  - 创建 `.prettierrc.json`：`{ "semi": true, "singleQuote": true, "trailingComma": "all", "printWidth": 100 }`
  - 创建 `.eslintrc.cjs`：扩展 `plugin:vue/vue3-recommended`、
    `@typescript-eslint/recommended`，规则 `max-warnings: 0`
  - 在 `package.json` 中添加脚本：
    `"lint": "eslint . --ext .ts,.vue --max-warnings 0"`,
    `"format:check": "prettier --check 'src/**/*.{ts,vue,json}'"`,
    `"format": "prettier --write 'src/**/*.{ts,vue,json}'"`

  **验收标准**：`npm run lint` 与 `npm run format:check` 在初始化代码上通过；故意
  注入 `console.log` 后 lint 报错。

- [x] T005 [P] 配置 Rust 格式化与静态分析工具

  **目标**：满足宪法 Principle I。

  **实现要点**：
  - 创建 `src-tauri/.rustfmt.toml`：`max_width = 100`、`use_field_init_shorthand = true`
  - 创建 `src-tauri/clippy.toml`：`avoid-breaking-exported-api = false`
  - 在 `src-tauri/Cargo.toml` 添加 `[lints.clippy]` 段：`pedantic = "warn"`、
    `nursery = "warn"`
  - 在仓库根添加 `package.json` 脚本：
    `"rust:fmt": "cargo fmt --manifest-path src-tauri/Cargo.toml -- --check"`,
    `"rust:clippy": "cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings"`

  **验收标准**：`npm run rust:fmt` 与 `npm run rust:clippy` 通过；引入 `println!`
  调试输出后 clippy 报错。

- [x] T006 [P] 创建中文注释比例验证脚本 `scripts/check-comment-ratio.sh`

  **目标**：实现宪法 Principle II 的 CI 检查（注释行数 / 非空代码行数 ≥ 50%）。

  **实现要点**：
  - 使用 `awk` 或 Python 实现：遍历 `src/**/*.{ts,vue}`、`src-tauri/src/**/*.rs`
  - 注释行匹配：Rust `^\s*//`、`^\s*/\*`、`^\s*\*`；TS `^\s*//`、`^\s*/\*`；
    Vue 模板内 `<!--`、JS 内同 TS
  - 仅统计含 CJK 字符（`[一-鿿]`）的注释行
  - 非空代码行：去除空行与纯注释行
  - 排除：`scripts/.comment-exemptions` 列出的路径（如 `db/migrations/*.sql`、
    `tauri.conf.json`、自动生成代码）
  - 输出：每个不达标的文件、其比例与差距；脚本退出码非零阻断 CI

  **验收标准**：手动测试一个达标文件与一个不达标文件，输出准确；CI 任务可调用。

- [x] T007 [P] 创建调试输出检查脚本 `scripts/check-no-debug-prints.sh`

  **目标**：实现宪法 Principle I 的"无遗留调试输出"门禁。

  **实现要点**：
  - 使用 `grep -rn` 扫描 `src/`、`src-tauri/src/`
  - 模式：Rust `println!`、`eprintln!`、`dbg!`；TS `console\.\(log\|debug\|trace\)`；
    TODO/FIXME 不带 `#\d+` issue 链接
  - 输出违规行 `<文件>:<行号>:<内容>`
  - 提供白名单文件（`main.rs` 启动日志等）

  **验收标准**：脚本能识别上述模式并以非零码退出；白名单生效。

- [x] T008 [P] 创建 Token 明文泄漏检查脚本 `scripts/check-no-token-leak.sh`

  **目标**：实现宪法 Principle III 的安全门禁（SC-009 / SC-010）。

  **实现要点**：
  - 扫描 SQLite 数据库的 `accounts`、`operation_logs` 表（CI 中需先运行一次产品流
    后扫描；本地开发可跳过）
  - 模式：`ghp_[A-Za-z0-9]{36}`（GitHub PAT）、`glpat-[A-Za-z0-9_-]{20}`
    （GitLab PAT）、`https://[^@/]+@`（带凭据的 URL）
  - 扫描日志目录：`~/Library/Logs/com.gitview.app/`（macOS）或对应目录
  - 任何匹配立即报错并退出非零

  **验收标准**：手动伪造 Token 字符串塞入测试日志验证脚本可发现。

- [x] T009 [P] 创建注释豁免清单 `.specify/comment-exemptions.yml`

  **目标**：维护宪法 Principle II 的豁免清单。

  **实现要点**：
  ```yaml
  exempt_paths:
    - src-tauri/src/db/migrations/*.sql
    - src-tauri/tauri.conf.json
    - tsconfig.json
    - vite.config.ts
    - .eslintrc.cjs
    - .prettierrc.json
  exempt_reasons:
    - "SQL 迁移文件按惯例不要求函数级注释"
    - "配置文件按惯例不要求注释"
  ```

  **验收标准**：T006 脚本读取此文件并跳过列出路径。

- [x] T010 [P] 创建主布局组件 `src/layouts/AppLayout.vue` 与导航占位

  **目标**：搭建应用整体布局壳子。

  **实现要点**：
  - `el-container` 包裹 `el-aside`（侧边栏）+ `el-main`（主内容区） + `el-header`（顶栏）
  - 侧边栏使用 `el-menu`，列出 7 个菜单项：首页 / 远程仓库 / 本地仓库 / Clone 中心 /
    账号管理 / 操作日志 / 设置
  - 顶栏占位：当前账号显示位、全局搜索（未实现）、同步按钮、设置入口
  - 主内容区使用 `<router-view>` 渲染当前路由

  **验收标准**：启动应用后看到完整三段式布局；菜单点击可切换路由（页面为空占位）。

- [x] T011 创建 GitHub Actions CI 工作流 `.github/workflows/ci.yml`

  **目标**：满足宪法 Principle I/II 在主分支合并前的强制门禁。

  **实现要点**：
  - 触发：`push` 到任意分支、`pull_request` 到 `main`
  - 矩阵：`ubuntu-latest`、`macos-latest`、`windows-latest`
  - Steps：
    1. checkout
    2. setup Node 20、Rust stable + clippy + rustfmt
    3. `npm ci`
    4. `npm run lint`
    5. `npm run format:check`
    6. `npm run rust:fmt`
    7. `npm run rust:clippy`
    8. `bash scripts/check-comment-ratio.sh`
    9. `bash scripts/check-no-debug-prints.sh`
    10. `npm run build`（前端）
    11. `cd src-tauri && cargo build`（后端，Linux 上需安装系统依赖
        `libwebkit2gtk-4.1-dev`、`libssl-dev`、`libgtk-3-dev`、
        `libayatana-appindicator3-dev`、`librsvg2-dev`；Windows 上需要
        **MSVC C++ Build Tools / Visual Studio 2022 Build Tools** 与
        WebView2 Runtime；macOS 需 Xcode Command Line Tools）

  **验收标准**：本地 `act` 或推送到分支后 CI 全部 step 通过。

- [x] T012 创建空白页面文件占位（7 个页面）

  **目标**：为后续 Phase 提供路由可达的页面壳。

  **实现要点**：在 `src/pages/` 下创建：`Dashboard.vue`、`Accounts.vue`、
  `RemoteRepositories.vue`、`CloneCenter.vue`、`LocalRepositories.vue`、
  `RepositoryDetail.vue`、`Logs.vue`、`Settings.vue`。每个文件含 `<template>`
  一行标题与文件级中文注释（符合 Principle II）。

  **验收标准**：所有 7 个路由可达且渲染各自标题；`check-comment-ratio.sh` 通过。

---

**Checkpoint Phase 1**: 项目骨架可启动，工具链与 CI 就绪。可进入 Foundational Phase。

---

## Phase 2: Foundational（阻塞性前置依赖）

**Purpose**: 所有 User Story 共用的底层基础设施。⚠️ **Phase 3+ 任务不得在
Phase 2 完成前启动。**

- [x] T013 创建统一错误类型 `src-tauri/src/errors.rs`

  **目标**：定义贯穿后端所有 service 与 command 的错误类型。

  **实现要点**：
  ```rust
  /// GitView 后端统一错误类型。
  /// 序列化后可直接返回给前端，前端按 code 字段映射本地化文案。
  #[derive(Debug, thiserror::Error, serde::Serialize)]
  #[serde(tag = "code", content = "detail")]
  pub enum GitViewError {
      #[error("数据库错误：{0}")] Database(String),
      #[error("凭据存储错误：{0}")] Credential(String),
      #[error("Token 无效或已过期")] TokenInvalid,
      #[error("API 地址错误")] ApiUrlInvalid,
      #[error("网络错误：{0}")] Network(String),
      #[error("TLS 证书错误：{0}")] TlsCert(String),
      #[error("权限不足")] Forbidden,
      #[error("资源不存在：{0}")] NotFound(String),
      #[error("Git 命令执行失败：{0}")] GitCommand(String),
      #[error("Git 未安装或路径无效")] GitNotFound,
      #[error("路径冲突：{0}")] PathConflict(String),
      #[error("路径不存在：{0}")] PathMissing(String),
      #[error("用户取消")] UserCancelled,
      #[error("内部错误：{0}")] Internal(String),
  }
  pub type Result<T> = std::result::Result<T, GitViewError>;
  ```
  从 `rusqlite::Error`、`reqwest::Error`、`keyring::Error`、`io::Error` 实现
  `From` 转换。

  **验收标准**：`cargo build` 通过；所有 service 均可使用 `crate::errors::Result`。

- [x] T014 [P] 配置 tracing 日志基础设施 `src-tauri/src/main.rs`

  **目标**：建立结构化日志系统，输出到滚动文件且支持级别过滤。

  **实现要点**：
  - 日志目录：使用 `dirs::data_local_dir()` + `gitview/logs`
  - `tracing-appender::rolling::daily()` 创建按天滚动 appender
  - `tracing_subscriber` 设置：JSON 格式、`EnvFilter::from_default_env()`
    默认 `info`
  - 在 `main.rs` 启动时初始化，应用退出时 `flush`
  - 强制注入 `LC_ALL=C` 与 `GIT_TERMINAL_PROMPT=0` 到所有 Git 子进程的
    环境变量基线
  - 中文文件头注释说明日志路径与级别配置

  **验收标准**：启动应用后日志目录出现 `gitview.log.YYYY-MM-DD` 文件；调用
  `tracing::info!` 可见输出。

- [x] T015 创建 SQLite 连接池 `src-tauri/src/db/pool.rs`

  **目标**：为多线程访问提供安全的 SQLite 连接池。

  **实现要点**：
  - 数据库路径：`dirs::data_local_dir() + gitview/gitview.db`
  - 首次启动若文件不存在则自动创建（含必要目录）
  - 启用 WAL：`PRAGMA journal_mode = WAL;`
  - 启用外键：`PRAGMA foreign_keys = ON;`
  - 提供 `pub struct DbPool { inner: Arc<Mutex<Connection>> }`，简单互斥锁
    满足单进程访问（V1 不引入 r2d2）
  - 提供 `pub fn with_conn<F, R>(f: F) -> Result<R>` 闭包式访问

  **验收标准**：单元测试创建临时数据库并执行简单查询通过。

- [x] T016 创建数据库迁移管理 `src-tauri/src/db/migrations.rs` 与初始迁移文件

  **目标**：版本化管理 SQLite schema。

  **实现要点**：
  - 内置 `schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT)` 表
  - 迁移文件嵌入：`include_str!("migrations/001_init.sql")` 等
  - 顺序检测：查询已应用版本，按未应用顺序执行
  - 创建 `src-tauri/src/db/migrations/001_init.sql` 包含全部 7 张业务表
    DDL（参考产品设计文档 §16），加上 `gitlab_instance_configs` 表
  - 每个 SQL 顶部使用 `-- 中文注释说明用途`（注释比例豁免范围内但仍写）
  - `application_id` PRAGMA 设置防止误打开非本应用 DB

  **验收标准**：首次启动后所有 7+1 张表创建成功；二次启动跳过已应用迁移。

- [x] T017 [P] 创建领域模型骨架 `src-tauri/src/models/`

  **目标**：定义与前端 TS 类型一一对应的 Rust 数据结构。

  **实现要点**：在 `src-tauri/src/models/` 下创建：
  - `account.rs` 含 `Account`（**必含 `enabled: bool` 字段对应 FR-009 启用/禁用**、
    `is_default: bool`、`platform`、`web_base_url`、`api_base_url`、`username`、
    `display_name?`、`avatar_url?`、`token_key`、`created_at`、`updated_at`、
    `last_sync_at?`）、`GitPlatform` 枚举、`GitLabInstanceConfig`
  - `repository.rs` 含 `RemoteRepository`、`LocalRepository`、`RepositoryStatus`、
    `Visibility`
  - `clone_task.rs` 含 `CloneTask`、`CloneTaskStatus`
  - `git.rs` 含 `FileChange`、`FileStatus`、`Branch`、`CommitInfo`、`GitStatus`
  - `settings.rs` 含 `Settings` key/value 结构与 `CloneProtocol`、
    `DirectoryStrategy`、`Theme`、`Language` 枚举
  - `operation_log.rs` 含 `OperationLog`、`OperationType`（V1 范围：
    `add_account`、`delete_account`、`test_connection`、`sync_repos`、
    `clone`、`fetch`、`pull`、`push`、`commit`、`checkout`、`create_branch`、
    `scan_repos`、`discard_changes`；**不含 V2 的 merge/rebase**）、
    `OperationStatus`

  所有结构体派生 `Debug, Clone, Serialize, Deserialize`；枚举使用
  `#[serde(rename_all = "snake_case")]`；时间字段使用 `chrono::DateTime<Utc>`
  序列化为 ISO 8601。

  **验收标准**：`cargo build` 通过；JSON 序列化样本符合预期；Account 含
  enabled 字段。

- [x] T018 [P] 创建工具模块 `src-tauri/src/utils/`

  **目标**：通用辅助函数集中管理。

  **实现要点**：
  - `path.rs`：`normalize_path()`（使用 `dunce::canonicalize`）、
    `ensure_dir_exists()`、`is_git_repository(path)`、`join_safe()`
  - `process.rs`：`run_command(cmd, args, env, cwd) -> Result<Output>` 异步包装
    `tokio::process::Command`；自动注入 `LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`；
    超时支持
  - `redact.rs`：`redact_token(text: &str) -> String` 使用正则匹配并替换：
    GitHub PAT（`ghp_`/`gho_`/`ghu_` 等前缀）、GitLab PAT（`glpat-` 前缀）、
    Bearer token 头、`https://<token>@host` 形式 URL；替换为
    `<REDACTED-TOKEN>` 或 `https://<REDACTED>@host`
  - `time.rs`：`now_iso8601()`、`parse_iso8601()`

  **验收标准**：每个工具函数配套单元测试至少 2 个用例。

- [x] T019 创建 Tauri 应用状态与 command 注册骨架 `src-tauri/src/main.rs`

  **目标**：建立 Tauri 应用主入口，注入服务状态与 command 注册。

  **实现要点**：
  - 定义 `AppState { db: DbPool, credential: CredentialService, ... }` 后续
    Phase 逐步填充
  - `tauri::Builder::default().setup(|app| { ... }).manage(state)
    .invoke_handler(tauri::generate_handler![...])`
  - `setup` 中：初始化日志、连接数据库、执行迁移
  - 错误处理：`AppHandle::manage` 失败时 `panic!` 退出（启动期错误用户无法
    恢复）

  **验收标准**：应用启动后日志显示"迁移已应用 N 个"，无 panic。

- [x] T020 [P] 创建前端 Tauri 调用封装 `src/api/tauri.ts`

  **目标**：统一 invoke 与事件订阅入口，集中处理错误。

  **实现要点**：
  ```ts
  /**
   * 调用 Tauri 后端命令的统一封装。
   * 自动捕获错误并转换为前端友好的 Error 对象。
   */
  export async function invokeCmd<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      return await invoke<T>(cmd, args);
    } catch (e) {
      const err = e as { code?: string; detail?: string };
      throw new GitViewClientError(err.code ?? 'Internal', err.detail);
    }
  }

  export function listenEvent<T>(event: string, handler: (payload: T) => void): UnlistenFn { ... }
  ```
  错误码到中文文案的本地化映射放在 `src/api/error-messages.ts`。

  **验收标准**：单元测试 mock `@tauri-apps/api/tauri` 后调用返回预期；错误抛出
  正确包装。

- [x] T021 [P] 创建前端类型定义骨架 `src/types/`

  **目标**：与 Rust models 对应的 TS 类型，确保前后端契约一致。

  **实现要点**：在 `src/types/` 下创建 `account.ts`、`repository.ts`、
  `cloneTask.ts`、`git.ts`、`settings.ts`、`operationLog.ts`。每个文件
  导出 `interface` 与 `enum`，字段名使用 `camelCase`，与 Rust 的 `#[serde(rename_all = "camelCase")]` 配套。所有 enum 使用字符串字面量联合
  类型（如 `type GitPlatform = 'github' | 'gitlab' | 'gitee'`）。

  **验收标准**：`tsc --noEmit` 通过。

- [x] T022 [P] 创建 Pinia store 骨架 `src/stores/`

  **目标**：定义 6 个 store 文件作为后续填充骨架。

  **实现要点**：在 `src/stores/` 下创建 `account.ts`、`remoteRepository.ts`、
  `localRepository.ts`、`cloneTask.ts`、`settings.ts`、`app.ts`。每个使用
  `defineStore` API，导出 `useXxxStore()`，初始 `state` 为空数组或默认值。

  **验收标准**：`tsc --noEmit` 通过；Vue DevTools 可见 6 个 store。

- [x] T023 创建凭据存储服务 `src-tauri/src/services/credential_service.rs`

  **目标**：封装 `keyring` 提供 Token 增删查接口（宪法 Principle III 安全要求）。

  **实现要点**：
  - 服务名常量：`SERVICE_NAME: &str = "gitview"`
  - 提供 `pub fn save_token(account_id: &str, token: &str) -> Result<()>`
  - 提供 `pub fn load_token(account_id: &str) -> Result<String>`（用于内部
    API 请求时取出）
  - 提供 `pub fn delete_token(account_id: &str) -> Result<()>`
  - 提供 `pub fn token_exists(account_id: &str) -> Result<bool>`
  - **关键安全约束**：load_token 返回的字符串只能在 service 内部使用，
    禁止暴露给 Tauri command 层；token 不得参与 `Serialize`
  - 检测 keyring 可用性：`pub fn check_availability() -> Result<()>` 用于
    启动期诊断

  **验收标准**：单元测试覆盖：保存→读取→删除→读取（应返回 NotFound）；
  Linux 上 Secret Service 不可用时返回明确错误。

- [x] T024 配置 Tauri 安全策略与权限 `src-tauri/tauri.conf.json`

  **目标**：声明 Tauri 2 安全权限（capabilities），限制前端可调用的 API 范围。

  **实现要点**：
  - 启用 `tauri-plugin-fs` 但限制范围到用户项目目录（不允许任意路径读写）
  - 启用 `tauri-plugin-dialog` 用于目录选择
  - 启用 `tauri-plugin-shell` 用于 "打开目录" / "打开终端"（受限的
    `open` 与 `execute`）
  - 启用 `tauri-plugin-os` 用于平台识别
  - CSP：`"default-src 'self'; img-src 'self' data: https:; style-src 'self' 'unsafe-inline';"`
    （Element Plus 需 unsafe-inline 样式）
  - 创建 `src-tauri/capabilities/default.json` 列出允许的命令清单（先空，后续
    Phase 增量添加）

  **验收标准**：开发模式启动应用，前端调用未授权 API 时被 Tauri 阻断。

---

**Checkpoint Phase 2**: 数据库、日志、凭据、错误类型、模型、工具就绪，
前端 Tauri 桥接与 store 骨架就绪。所有 User Story 可并行启动。

---

## Phase 3: User Story 1 — 多平台多账号统一管理 (Priority: P1) 🎯 MVP

**Goal**: 用户能够添加 GitHub / GitLab / Gitee 账号（含自建 GitLab 实例），
通过 Token 测试连接，管理账号列表与默认账号。

**Independent Test**: 添加一个 GitHub 公开账号、一个自建 GitLab 账号、一个 Gitee
账号，测试连接均返回正确用户信息；删除账号后凭据从安全存储清除。

- [x] T025 [P] [US1] 定义 `GitHostingProvider` trait 于 `src-tauri/src/services/provider.rs`

  **目标**：统一三大平台的接口抽象，便于 Provider 实现与切换。

  **实现要点**：
  ```rust
  /// Git 托管平台 Provider 抽象。
  /// 每个平台实现该 trait 后即可被 account_service 统一调用。
  #[async_trait::async_trait]
  pub trait GitHostingProvider: Send + Sync {
      async fn get_current_user(&self) -> Result<UserProfile>;
      async fn list_repositories(&self, page: u32, per_page: u32)
          -> Result<RepositoryPage>;
  }

  pub struct UserProfile { pub username: String, pub display_name: Option<String>, pub avatar_url: Option<String> }
  pub struct RepositoryPage { pub items: Vec<RemoteRepository>, pub has_next: bool }
  ```
  添加 `async_trait = "0.1"` 依赖。

  **验收标准**：`cargo build` 通过；trait 可被 mock。

- [x] T026 [P] [US1] 实现 GitHub Provider `src-tauri/src/services/github_service.rs`

  **目标**：调用 GitHub REST API 获取当前用户信息（首期仅 `get_current_user`，
  `list_repositories` 留到 US2 实现）。

  **实现要点**：
  - 构造函数 `new(api_base_url: String, token: String, proxy: Option<String>) -> Self`
  - 使用 `reqwest::Client::builder()` 配置：超时 30s、代理（若有）、UA 字符串
    `GitView/1.0`
  - `get_current_user()`：`GET {api_base}/user`，Header
    `Authorization: Bearer {token}`、`Accept: application/vnd.github+json`、
    `X-GitHub-Api-Version: 2022-11-28`
  - 解析响应字段：`login`、`name`、`avatar_url`
  - 错误处理：401 → `TokenInvalid`、403 → `Forbidden`、网络错误 → `Network`、
    超时 → `Network("超时")`
  - **Token 不得出现在 panic、debug、log 消息中**

  **验收标准**：集成测试使用 `wiremock` 模拟 GitHub API 返回 200/401/403，断言
  对应行为。

- [x] T027 [P] [US1] 实现 GitLab Provider `src-tauri/src/services/gitlab_service.rs`

  **目标**：支持 gitlab.com 与自建 GitLab，处理 API 地址、自签名证书、代理。

  **实现要点**：
  - 构造函数额外接收 `GitLabInstanceConfig`（用于自签名证书白名单、API 路径
    前缀、代理）
  - 自签名证书：当 `allow_invalid_certs == true` 时
    `ClientBuilder::danger_accept_invalid_certs(true)`；但**仅对该实例生效**，
    严禁全局
  - `get_current_user()`：`GET {api_base}/user`，Header `PRIVATE-TOKEN: {token}`
  - 解析字段：`id`、`username`、`name`、`avatar_url`、`web_url`
  - 错误映射同 T026

  **验收标准**：集成测试覆盖 gitlab.com 与自签名证书场景（使用
  `rcgen` 生成临时证书）。

- [x] T028 [P] [US1] 实现 Gitee Provider `src-tauri/src/services/gitee_service.rs`

  **目标**：调用 Gitee OpenAPI 获取当前用户信息。

  **实现要点**：
  - `get_current_user()`：`GET {api_base}/user?access_token={token}`（Gitee 习惯用 query），
    或 Header `Authorization: token {token}`
  - 解析字段：`login`、`name`、`avatar_url`
  - 错误映射同 T026

  **验收标准**：集成测试模拟 Gitee API 三种响应。

- [x] T029 [US1] 实现 GitLab API 地址推导工具于 `src-tauri/src/services/gitlab_service.rs`

  **目标**：用户填写 Web 地址后自动推导 API 地址（spec FR-005）。

  **实现要点**：
  ```rust
  /// 根据 GitLab Web 地址推导 API 地址。
  /// 规则：保留协议、host、端口；在路径末尾追加 /api/v4。
  /// 已带子路径（如 https://code.company.com/gitlab）需正确拼接。
  pub fn derive_gitlab_api_url(web_url: &str) -> Result<String> { ... }
  ```
  使用 `url::Url::parse` + path 拼接；末尾 `/` 规范化。

  **验收标准**：单元测试覆盖：标准、非标准端口、HTTP 内网、子路径部署。

- [x] T030 [US1] 实现 account_service 核心逻辑 `src-tauri/src/services/account_service.rs`

  **目标**：账号 CRUD、连接测试、默认账号管理。

  **实现要点**：
  - `add_account(payload: AddAccountPayload) -> Result<Account>`：先调用 provider
    test，成功后开启数据库事务：插入 accounts 行（含 `enabled = true` 默认值）、
    保存 Token 到凭据；任何一步失败则回滚（凭据已存时清除）
  - `test_account_connection(payload) -> Result<UserProfile>`：构造临时 provider
    调用 `get_current_user`
  - `list_accounts() -> Result<Vec<Account>>`
  - `update_account(id, fields) -> Result<Account>`：fields 是 `AccountUpdate`
    可选字段集合（`display_name?`、`remark?`、`enabled?`、其他元信息）；
    `enabled` 字段对应 FR-009 启用/禁用切换；**禁用账号后该账号下的远程仓库
    缓存保留但不再参与同步与列表默认筛选**
  - `delete_account(id) -> Result<()>`：先删除 keyring Token，再删数据库行；
    若是默认账号需将默认转移到剩余 enabled 账号中的最早一个
  - `set_default_account(id) -> Result<()>`：原子更新（先全部置 0 再置目标为 1）；
    目标账号必须 `enabled = true`，否则返回错误
  - 提供独立辅助方法 `set_account_enabled(id, enabled) -> Result<Account>`
    （供前端开关组件直接调用，等价于 `update_account` 的语法糖）
  - **账号同步互斥**：内部维护 `Arc<Mutex<HashSet<account_id>>>` 跟踪正在同步的
    账号集合；同一账号重复触发同步时返回 `BusyAccount`；不同账号允许并行
    （对应 spec Edge Case "多账号同时同步" 与 plan §Risks）

  **验收标准**：集成测试覆盖 add → list → update(enabled=false) → set_default
  应拒绝禁用账号 → enable → set_default → delete 全链路；同账号并发同步
  返回 BusyAccount 错误。

- [x] T031 [US1] 实现自建 GitLab 实例配置持久化于 `account_service.rs`

  **目标**：保存 `gitlab_instance_configs` 一对一关联到账号。

  **实现要点**：
  - `add_account` payload 携带可选的 `instance_config` 字段
  - 同一事务中插入 `accounts` 与 `gitlab_instance_configs`
  - 字段：`allow_insecure_http`、`allow_invalid_certs`、`use_system_proxy`、
    `proxy_url`、`request_timeout_seconds`、`default_clone_protocol`、
    `ssh_host_alias`、`api_path_prefix`、`last_connection_status`、
    `last_connection_error`
  - 提供 `get_gitlab_instance_config(account_id)` 用于 provider 实例化

  **验收标准**：单元测试创建一个带 instance_config 的 GitLab 账号，读出后字段
  一致。

- [x] T032 [US1] 实现 Tauri command `add_account` 于 `src-tauri/src/commands/accounts.rs`

  **目标**：前端调用入口。

  **实现要点**：
  ```rust
  #[tauri::command]
  pub async fn add_account(state: tauri::State<'_, AppState>, payload: AddAccountPayload)
      -> Result<Account, GitViewError> { ... }
  ```
  在 `main.rs` 的 `invoke_handler` 中注册；在 `capabilities/default.json`
  中允许。

  **验收标准**：前端可通过 `invokeCmd('add_account', ...)` 调用并获得正常响应。

- [x] T033 [US1] 实现其他账号 Tauri commands

  **目标**：实现 `test_account_connection`、`list_accounts`、`update_account`、
  `delete_account`、`set_default_account`、`sync_account_repositories`
  （后者在 US2 中真正同步仓库，本任务先建立 stub）。

  **实现要点**：每个 command 转发到 `account_service` 对应方法；payload 类型在
  models 中定义；返回值序列化保持稳定（不变更字段名以免破坏前端）。

  **验收标准**：6 个命令均可被前端调用并返回预期；`sync_account_repositories`
  返回空数组（待 US2 填充）。

- [x] T034 [P] [US1] 创建前端账号 API 封装 `src/api/account.api.ts`

  **目标**：封装上述 6 个 command，提供强类型接口。

  **实现要点**：
  ```ts
  export const accountApi = {
    add(payload: AddAccountPayload): Promise<Account> { return invokeCmd('add_account', { payload }); },
    test(payload: TestConnectionPayload): Promise<UserProfile> { ... },
    list(): Promise<Account[]> { ... },
    update(id: string, fields: Partial<AccountUpdate>): Promise<Account> { ... },
    delete(id: string): Promise<void> { ... },
    setDefault(id: string): Promise<void> { ... },
  };
  ```

  **验收标准**：TS 类型覆盖；Vitest 单元测试 mock invokeCmd 调用断言参数。

- [x] T035 [P] [US1] 实现 Pinia store `src/stores/account.ts`

  **目标**：账号列表的响应式状态管理。

  **实现要点**：
  - state：`accounts: Account[]`、`loading: boolean`、`error: string | null`
  - actions：`loadAccounts()`、`addAccount(payload)`、`removeAccount(id)`、
    `setDefault(id)`、`updateAccount(id, fields)`
  - getters：`defaultAccount`、`accountsByPlatform(platform)`、
    `gitlabSelfHostedAccounts`

  **验收标准**：组件可订阅 store 并自动响应变化。

- [x] T036 [US1] 实现账号管理页面 `src/pages/Accounts.vue`

  **目标**：呈现账号列表并提供添加/编辑/删除入口。

  **实现要点**：
  - 顶部操作栏：「添加账号」按钮 → 打开 `AccountFormDialog`
  - 使用 `el-table` 渲染账号列表，列：头像、平台、用户名、显示名、服务地址、
    是否默认、是否启用（`el-switch` 绑定 `accountStore.toggleEnabled(id)`，
    禁用后行的其他操作按钮变灰但仍可编辑/删除）、最近同步时间、操作（同步 /
    测试连接 / 编辑 / 设为默认 / 启用 禁用 / 删除）
  - "启用/禁用"切换：调用 `accountApi.update(id, { enabled })`；禁用账号若
    为当前默认账号，系统自动将默认转移到第一个 enabled 账号并提示用户
  - 删除按钮触发 `ConfirmDangerDialog`（Principle III 删除确认）：列出"将
    清除系统凭据、保留已克隆本地仓库记录"，要求用户输入用户名关键词二次确认
  - 加载状态：`v-loading="accountStore.loading"`

  **验收标准**：与设计文档 §7.5 一致；空状态显示 `<EmptyState />`；启用/
  禁用切换后列表立即响应；禁用当前默认账号触发默认账号自动转移。

- [x] T037 [US1] 实现账号表单对话框 `src/components/account/AccountFormDialog.vue`

  **目标**：账号添加/编辑表单 UI。

  **实现要点**：
  - 平台选择：GitHub / GitLab.com / 私有 GitLab / Gitee
  - 选择"私有 GitLab"时展开实例配置高级字段：实例名称、Web 地址、API 地址
    （自动推导，可手动修改）、Token、是否允许 HTTP、是否允许自签名证书、
    使用系统代理、代理地址、请求超时、默认 Clone 协议、SSH 主机别名、
    API 路径前缀
  - 选择 HTTP 时弹出 `el-message-box` 安全提示
  - "测试连接"按钮：调用 `accountApi.test(payload)`，成功后显示用户名/头像，
    失败显示中文错误提示
  - 保存按钮：禁用直到测试连接通过
  - Token 输入框使用 `type="password"`，不缓存到 localStorage

  **验收标准**：手工测试 GitHub / 自建 GitLab / Gitee 三种平台均可添加成功。

- [x] T038 [US1] 实现账号卡片与默认账号切换组件 `src/components/account/AccountCard.vue` 与 `src/components/account/AccountSwitcher.vue`

  **目标**：复用的账号卡片视图与顶栏账号切换器。

  **实现要点**：
  - `AccountCard.vue`：紧凑展示头像、用户名、平台标签、默认标识
  - `AccountSwitcher.vue`：顶栏下拉选择当前活跃账号，调用
    `accountStore.setDefault(id)`

  **验收标准**：账号切换后顶栏立即更新；其他页面（US2/US3）可读取当前默认账号。

- [x] T039 [US1] [T] 编写三平台 Provider 集成测试 `tests/integration/{github,gitlab,gitee}_service_test.rs`

  **目标**：验证 GitHub / GitLab / Gitee Provider 的关键路径与错误映射，
  及 GitLab API 地址推导工具。

  **实现要点**：
  - 使用 `wiremock` 启动本地 mock 服务为三平台分别建立
    `tests/integration/github_service_test.rs`、`gitlab_service_test.rs`、
    `gitee_service_test.rs`
  - 共通用例：成功获取用户、401 → TokenInvalid、403 → Forbidden、
    超时 → Network、Token 不出现在错误消息中
  - GitLab 专项：自签名证书允许场景（用 `rcgen` 生成临时证书）、API 地址
    推导（`derive_gitlab_api_url` 表驱动测试覆盖标准/非标准端口/HTTP 内网/
    子路径部署四类输入）
  - Gitee 专项：query 参数 `access_token` 与 Header `Authorization: token`
    两种认证模式均测试
  - 用 `cargo test --test github_service_test`（及其他）运行

  **验收标准**：所有用例通过；三平台覆盖均衡。

- [x] T040 [US1] [T] 编写凭据服务单元测试 `src-tauri/src/services/credential_service.rs` 内部 `#[cfg(test)]`

  **目标**：验证保存/读取/删除链路 + 验证 delete_account 后凭据从系统安全存储
  100% 清除（对应 SC-011）。

  **实现要点**：
  - 使用 `MOCK_KEYRING` 环境变量或 `keyring::set_default_credential_builder` 注入
    内存实现（避免污染 CI 环境真实 keyring）
  - 用例：save 后 load 一致、delete 后 load 返回错误、不可用环境探测

  **验收标准**：本地 macOS 与 CI Linux 上均通过。

- [x] T041 [US1] 实现 GitView 全局 Token 脱敏中间件

  **目标**：所有错误日志、UI 错误信息均经过 `redact_token` 处理。

  **实现要点**：
  - `log_service`（在 US6 完整实现）的占位接口：`record_error(op, msg)`
    在内部调用 `redact::redact_token(msg)`
  - GitHub/GitLab/Gitee provider 在生成错误消息前显式脱敏
  - 单元测试：注入伪造 PAT 字符串到错误消息验证脱敏

  **验收标准**：测试通过；CI 中的 token 泄漏扫描脚本 (T008) 在本 phase
  代码上扫描无报警。

---

**Checkpoint US1**: 用户可以添加 / 测试 / 列出 / 编辑 / 删除三大平台账号；
Token 安全存储且不泄漏。US1 独立可发布。

---

## Phase 4: User Story 2 — 远程仓库统一浏览与搜索 (Priority: P1)

**Goal**: 用户能同步账号下远程仓库列表，统一搜索、筛选、收藏，查看仓库详情。

**Independent Test**: 已添加 1 个账号时执行"同步仓库"应完整拉取分页数据；
搜索关键词应在缓存上过滤；收藏跨重启持久化。

- [x] T042 [P] [US2] 扩展 Provider trait 实现 `list_repositories` 分页拉取

  **目标**：补全三大平台 `list_repositories` 方法（T025-T028 中预留）。

  **实现要点**：
  - GitHub：`GET /user/repos?per_page=100&page={n}&affiliation=owner,collaborator,organization_member`，
    Header `Link` 解析下一页指针；返回字段映射 `full_name`、`description`、
    `default_branch`、`private`、`html_url`、`clone_url`、`ssh_url`、`language`、
    `updated_at`
  - GitLab：`GET /projects?membership=true&simple=true&per_page=100&page={n}`，
    Header `X-Next-Page` 用于翻页判断；字段映射 `path_with_namespace`、
    `description`、`default_branch`、`visibility`、`web_url`、`http_url_to_repo`、
    `ssh_url_to_repo`、`last_activity_at`
  - Gitee：`GET /user/repos?type=all&per_page=100&page={n}`；字段映射类似 GitHub
  - 限流处理：429 时读取 `Retry-After`，最多重试 3 次

  **验收标准**：集成测试模拟分页（3 页共 250 条）成功合并。

- [x] T043 [US2] 实现仓库同步服务 `src-tauri/src/services/account_service.rs`（扩展）

  **目标**：实现 `sync_account_repositories(account_id)` 业务逻辑。

  **实现要点**：
  - 流程：取出账号 → 构造 provider → 循环分页拉取 → 与现有缓存对比
    （按 `<platform>:<owner>:<name>` 主键）→ upsert 到 `remote_repositories`
    表 → 更新 `accounts.last_sync_at`
  - 进度事件：通过 `tauri::AppHandle::emit("repository-sync-progress", { account_id, fetched, total })`
  - 增量策略：V1 简化为全量替换该账号下的远程仓库缓存（先 mark stale 再
    upsert，结束后清理 stale）；同步失败时**不 commit stale 清理**保证缓存
    不丢
  - 限流处理：捕获 429 并保留已同步数据

  **验收标准**：集成测试模拟 50 条仓库分页同步；中断后再次同步收敛一致。

- [x] T044 [P] [US2] 实现远程仓库查询 service `src-tauri/src/services/repository_service.rs`（扩展）

  **目标**：从 SQLite 查询远程仓库列表，支持筛选与搜索。

  **实现要点**：
  - `list_remote_repositories(filter: RemoteRepoFilter) -> Result<Vec<RemoteRepository>>`
  - filter 字段：`account_id?`、`platforms: Vec<GitPlatform>`、`owners: Vec<String>`、
    `visibilities: Vec<Visibility>`、`only_favorite: bool`、`only_cloned: bool`、
    `search: Option<String>`
  - 构造参数化 SQL：`WHERE` 子句按 filter 字段拼接；`search` 使用 `LIKE '%'||?||'%'`
    匹配 `name OR description`
  - 排序：默认按 `updated_at_remote DESC`
  - 性能：合理索引（在 init migration 中已加 `(account_id)`、`(platform)`、
    `(is_favorite)`、`(full_name)` 索引）

  **验收标准**：单元测试覆盖：单条件、多条件组合、搜索匹配。

- [x] T045 [US2] 实现 Tauri commands `list_remote_repositories`、`search_remote_repositories`、`refresh_remote_repositories`、`get_remote_repository_detail`、`toggle_favorite_remote_repository`

  **目标**：US2 全部 5 个前端入口。

  **实现要点**：
  - `list_remote_repositories(filter)` → repository_service.list_remote_repositories
  - `search_remote_repositories(keyword, filter)` → 同上叠加 search 字段
  - `refresh_remote_repositories(account_id?)` → 调用 sync_account_repositories；
    支持单账号或全部账号
  - `get_remote_repository_detail(repo_id)` → SELECT 单条 + 关联本地仓库
    信息
  - `toggle_favorite_remote_repository(repo_id)` → UPDATE is_favorite

  **验收标准**：5 个命令注册并在 capabilities 中允许；前端调用返回正常。

- [x] T046 [P] [US2] 创建前端远程仓库 API 与 store

  **目标**：`src/api/remoteRepository.api.ts`、`src/stores/remoteRepository.ts`

  **实现要点**：
  - api 封装 5 个 command
  - store state：`repos: RemoteRepository[]`、`loading`、`syncing`、`syncProgress`
  - actions：`fetchList(filter)`、`refresh(accountId?)`、`toggleFavorite(repoId)`
  - 订阅 `repository-sync-progress` 事件并更新 `syncProgress`

  **验收标准**：组件订阅 store 后能看到同步进度实时变化。

- [x] T047 [US2] 实现远程仓库列表页 `src/pages/RemoteRepositories.vue`

  **目标**：仓库列表的完整 UI。

  **实现要点**：
  - 顶部筛选区：平台多选、账号多选、Owner 输入、可见性、`only_favorite`、
    `only_cloned`、`search`
  - 主体使用 `el-table` 或 `el-virtual-list`（>500 条时切到虚拟滚动）
  - 列：选择框、平台 Badge、仓库名（可点击打开详情）、Owner、所属账号、默认
    分支、可见性、语言、是否已克隆、最近更新时间、操作（Clone / 详情 /
    打开网页 / 复制 HTTPS / 复制 SSH / 收藏）
  - 顶部「同步」按钮：选择"同步当前账号"或"同步全部账号"

  **验收标准**：5000 条数据虚拟滚动渲染无卡顿；筛选实时生效。

- [x] T048 [US2] 实现远程仓库表组件 `src/components/repository/RemoteRepoTable.vue`

  **目标**：抽取列表组件用于复用与单测。

  **实现要点**：props：`items: RemoteRepository[]`、`loading: boolean`、
  `selection: string[]`；emits：`update:selection`、`open-detail`、`clone`、
  `toggle-favorite`、`copy-url(type)`。使用 `defineProps`/`defineEmits`
  类型化。

  **验收标准**：Vitest 单元测试覆盖 props 渲染、emit 事件触发。

- [x] T049 [US2] 实现仓库详情抽屉 `src/components/repository/RepoDetailDrawer.vue`

  **目标**：点击仓库后的详情侧滑面板。

  **实现要点**：
  - 使用 `el-drawer`，方向 right，宽度 480px
  - 字段：仓库名、描述、平台、所属账号、Owner、默认分支、可见性、Web URL、
    HTTPS Clone URL、SSH Clone URL、语言、最近更新时间、本地路径（若已克隆）
  - 操作按钮：Clone、打开网页、复制 HTTPS、复制 SSH、收藏、打开本地仓库
    （若已克隆）
  - 复制使用 `navigator.clipboard.writeText`

  **验收标准**：与设计文档 §8.7 完全一致。

- [x] T050 [US2] 实现搜索与筛选 UI 组件（嵌入 RemoteRepositories.vue 或独立组件）

  **目标**：可组合多条件的筛选区。

  **实现要点**：使用 `el-input` (search) + `el-select multiple` (平台/账号/owner)
  + `el-checkbox` (only_favorite / only_cloned) + `el-select` (visibility)。
  搜索使用 `debounce(300ms)` 触发列表刷新（依赖 utils/debounce.ts）。

  **验收标准**：手动输入关键词，列表在 500 ms 内更新。

- [x] T051 [US2] [T] 编写远程仓库分页同步集成测试 `tests/integration/repository_sync_test.rs`

  **目标**：验证多页拉取与限流处理。

  **实现要点**：wiremock 模拟 3 页响应（GitHub Link 头 / GitLab X-Next-Page），
  断言最终入库 N 条；模拟 429 + Retry-After，断言重试逻辑；模拟同步中断时
  保留旧缓存。

  **验收标准**：所有用例通过。

- [x] T052 [US2] [T] 编写 list_remote_repositories 筛选单元测试

  **目标**：验证 SQL 筛选逻辑。

  **实现要点**：在 `repository_service.rs` 内 `#[cfg(test)]` 模块，准备
  20 条种子数据，验证 6 类筛选组合的命中数量与顺序。

  **验收标准**：所有用例通过。

---

**Checkpoint US2**: 远程仓库可同步、浏览、搜索、筛选、收藏；为 US3 提供数据
源。US1+US2 可作为"账号 + 仓库浏览"独立发布。

---

## Phase 5: User Story 3 — 多仓库批量克隆 (Priority: P1) 🎯 核心差异化能力

**Goal**: 用户能多选远程仓库批量 Clone，配置目录策略与并发数，实时查看每个
任务进度，支持取消/重试。

**Independent Test**: 多选 5 个仓库，配置并发 3，观察任务队列同时最多 3 个执行，
全部成功后自动加入本地仓库列表；HTTPS 仓库的 `remote.origin.url` 不含 Token。

- [x] T053 [P] [US3] 实现 Git CLI 检测与基础执行封装 `src-tauri/src/services/git_cli_service.rs`

  **目标**：构建 Git CLI 的统一调用入口。

  **实现要点**：
  - `pub struct GitCliService { git_path: PathBuf }`
  - `pub async fn detect_git() -> Result<GitVersionInfo>`：运行 `git --version` 并
    解析版本号；同时读取 `git config --global user.name/user.email`
  - `pub async fn run(&self, args: &[&str], cwd: Option<&Path>, env: &[(&str, &str)])
    -> Result<Output>`：调用 `utils::process::run_command`
  - 强制注入环境变量：`LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`、`GIT_ASKPASS=`（按需）
  - 提供 `set_git_path(path)` 用于设置自定义 Git 可执行文件路径

  **验收标准**：在已安装 Git 的环境下 detect_git 返回非空版本；未安装时返回
  `GitNotFound`。

- [x] T054 [US3] 实现 Clone 命令封装于 `git_cli_service.rs`

  **目标**：封装 `git clone` 调用并提供进度回调。

  **实现要点**：
  ```rust
  pub async fn clone_repository(
      &self,
      remote_url: &str,
      target_path: &Path,
      protocol_credentials: Option<CredentialInjection>,
      progress: impl Fn(CloneProgressEvent) + Send + Sync + 'static,
      cancel_token: CancellationToken,
  ) -> Result<()>
  ```
  - 命令：`git clone --progress {remote_url} {target_path}`
  - 添加 `--config credential.helper=` 临时禁用全局凭据 helper（避免污染）
  - HTTPS 时通过 `CredentialInjection` 注入临时凭据（见 T056）
  - 启动子进程后 `tokio::spawn` 异步读取 stderr（git clone 进度走 stderr）
  - 使用 `tokio_util::sync::CancellationToken` 监听取消信号；取消时 `child.kill()`
  - 失败时清理 `target_path` 的半成品目录

  **验收标准**：本地实际 clone 一个小仓库成功；中途取消后目标目录被清理。

- [x] T055 [P] [US3] 实现 Clone 进度解析器 `src-tauri/src/services/clone_task_service.rs`（独立模块）

  **目标**：解析 Git clone 的 stderr 输出为结构化进度事件。

  **实现要点**：
  - 阶段识别正则：
    - `Cloning into '...'` → 阶段 `Initializing`
    - `Enumerating objects: N(, done)?` → `Enumerating`
    - `Counting objects: N% (a/b), ...` → `Counting`, progress = N
    - `Compressing objects: N% (a/b)` → `Compressing`
    - `Receiving objects: N% (a/b), ...` → `Receiving`, progress = N
    - `Resolving deltas: N% (a/b)` → `Resolving`
    - `Updating files: N% (a/b)` → `Checkout`
  - 输出事件结构：`{ stage: String, percent: u8, raw_line: String }`
  - 行缓冲：按 `\r` 或 `\n` 切分

  **验收标准**：单元测试喂入真实 git clone stderr 样本，断言事件序列与最终
  `percent=100`。

- [x] T056 [US3] 实现 HTTPS 临时凭据注入 `src-tauri/src/services/clone_task_service.rs`（CredentialInjection）

  **目标**：HTTPS Clone 时安全注入 Token，确保 token 不写入 remote URL 或日志。

  **实现要点**：方案 A（推荐）：使用临时 GIT_ASKPASS 脚本
  - 创建临时脚本（Unix：`.sh`，Windows：`.bat`），内容为 `echo {token}`
  - 脚本路径写入随机临时目录（`std::env::temp_dir().join("gitview-askpass-{uuid}")`）
  - 设置子进程环境：`GIT_ASKPASS={script_path}`、`GIT_TERMINAL_PROMPT=0`
  - 任务结束后**立即**删除脚本（finally 块 + drop guard）
  - 脚本本身不写入日志；token 仅在临时文件中存在
  - 备选方案：使用 `git -c credential.helper=...` 但 token 仍出现在进程参数中，
    PS 等命令可见 → 不采用

  **验收标准**：测试一次 HTTPS Clone 后 `cat .git/config` 中 `remote.origin.url`
  无 token；操作日志中无 token。

- [x] T057 [US3] 实现 Clone 任务队列与并发控制 `src-tauri/src/services/clone_task_service.rs`

  **目标**：管理任务生命周期、并发上限、状态变更。

  **实现要点**：
  - 数据结构：`Arc<Mutex<HashMap<TaskId, CloneTaskHandle>>>`
  - 使用 `tokio::sync::Semaphore` 控制并发数（用户配置，默认 3，上限 8）
  - `create_clone_tasks(payload)`：批量插入 `clone_tasks` 表（status=pending）
  - `start_clone_tasks(task_ids)`：每个任务 spawn 一个 tokio task；进入 semaphore
    → 状态置 running → 调用 clone_repository → 监听进度并 emit Tauri event
    `clone-task-progress` 与状态变更 `clone-task-status-changed` → 成功后置
    success、失败置 failed
  - `cancel_clone_task(task_id)`：触发 CancellationToken；状态置 cancelled
  - `retry_clone_task(task_id)`：重置 progress/error，重新 spawn
  - `clear_finished_clone_tasks()`：删除 status 为 success/cancelled/skipped 的
    任务记录
  - 已存在目录检测：clone 前若 `target_path` 已存在则 status=skipped（spec
    FR-022）

  **验收标准**：5 个任务并发 3 时同时执行不超过 3；取消任务后任务表 status 正确；
  重试失败任务后状态重置。

- [x] T058 [US3] 实现目录组织策略 `src-tauri/src/services/clone_task_service.rs`

  **目标**：根据用户选择计算每个 Clone 任务的 `target_path`。

  **实现要点**：
  ```rust
  pub enum DirectoryStrategy { Flat, ByPlatformAndAccount, ByOwner }
  pub fn compute_target_path(root: &Path, repo: &RemoteRepository, strategy: DirectoryStrategy) -> PathBuf {
      match strategy {
          Flat => root.join(&repo.name),
          ByPlatformAndAccount => root.join(repo.platform.as_str()).join(&repo.owner).join(&repo.name),
          ByOwner => root.join(&repo.owner).join(&repo.name),
      }
  }
  ```
  在调用前 `utils::path::ensure_dir_exists(parent)`。

  **验收标准**：单元测试覆盖 3 种策略 + 边界（同名不同 owner、Unicode 路径）。

- [x] T059 [US3] 实现 Clone Tauri commands `src-tauri/src/commands/clone_tasks.rs`

  **目标**：注册 6 个 command：`create_clone_tasks`、`list_clone_tasks`、
  `start_clone_tasks`、`cancel_clone_task`、`retry_clone_task`、
  `clear_finished_clone_tasks`。

  **实现要点**：薄包装层，所有业务逻辑在 service；payload 类型定义在
  `models/clone_task.rs`：
  ```rust
  pub struct CreateCloneTasksPayload {
      pub remote_repository_ids: Vec<String>,
      pub target_root: PathBuf,
      pub directory_strategy: DirectoryStrategy,
      pub protocol: CloneProtocol,
      pub concurrency: u8,
      pub existing_dir_policy: ExistingDirPolicy,
      pub auto_add_to_local_repos: bool,
  }
  ```

  **验收标准**：6 个命令均注册且 capabilities 中允许。

- [x] T060 [US3] 实现 Clone 成功后自动加入本地仓库 `clone_task_service.rs`

  **目标**：spec FR-025，clone 成功后若用户启用 `auto_add_to_local_repos`，
  在 `local_repositories` 表插入对应记录。

  **实现要点**：
  - 调用 `repository_service::add_local_repository_from_clone(task, repo_metadata)`
  - 关联 `account_id`、`remote_url`（来自远程仓库缓存）、`platform`、`owner`
  - 默认 `status = clean`、`ahead = 0`、`behind = 0`
  - 同时更新 `remote_repositories.local_repository_id`

  **验收标准**：clone 成功后本地仓库页面立即看到新仓库。

- [x] T061 [P] [US3] 创建前端 Clone 任务 API 与 store

  **目标**：`src/api/cloneTask.api.ts`、`src/stores/cloneTask.ts`

  **实现要点**：
  - api 封装 6 个 command
  - store state：`tasks: CloneTask[]`、`activeCount`、`totalProgress`
  - 订阅 `clone-task-progress` 与 `clone-task-status-changed` 事件并实时
    更新单条任务
  - actions：`createTasks(payload)`、`start(ids)`、`cancel(id)`、`retry(id)`、
    `clearFinished()`

  **验收标准**：组件可订阅 store 看到任务进度实时变化。

- [x] T062 [US3] 实现批量 Clone 对话框 `src/components/clone/BatchCloneDialog.vue`

  **目标**：从远程仓库列表多选触发的配置对话框。

  **实现要点**：
  - 接收 props：`selectedRepos: RemoteRepository[]`
  - 字段：目标根目录（el-input + 选择按钮触发 `dialog.open({ directory: true })`）、
    目录组织方式（el-radio-group 3 选项）、Clone 协议（HTTPS / SSH，默认值
    从 settings 读取）、并发数（el-slider 1-8，默认 3）、已存在目录策略
    （跳过 / 提示，V1 仅"跳过"）、克隆完成后自动加入本地仓库（el-switch）
  - 底部显示预览：每个仓库的最终 target_path
  - 确认按钮调用 `cloneTaskApi.createTasks(payload)` 然后 `start(ids)`

  **验收标准**：对话框打开后所有字段从 settings 预填；确认后跳转 Clone 中心。

- [x] T063 [US3] 实现 Clone 中心页面 `src/pages/CloneCenter.vue` 与任务表组件

  **目标**：展示任务队列、进度、状态。

  **实现要点**：
  - 顶部统计：总任务数、运行中、成功、失败、跳过、已取消
  - 任务列表 `el-table`：仓库名、平台、所属账号、目标路径、状态 Tag、进度
    （`el-progress` 显示百分比与阶段）、错误信息（鼠标悬浮显示）、创建时间、
    操作（取消 / 重试 / 打开目录 / 查看日志 / 移除）
  - 顶部操作：全部启动、清空已完成
  - 进度组件 `src/components/clone/CloneProgress.vue` 抽取，支持
    阶段标签 + 百分比

  **验收标准**：5 个任务并发执行时 UI 实时更新；取消按钮可终止任务。

- [x] T064 [US3] [T] 编写 Clone 进度解析器单元测试 `src-tauri/src/services/clone_task_service.rs#test`

  **目标**：验证 stderr 解析对常见 Git 输出的正确性。

  **实现要点**：准备真实 git clone stderr 样本字符串（覆盖小仓库、大仓库、
  多阶段），断言解析后事件序列与最终 100% 完成。

  **验收标准**：单元测试通过。

- [x] T065 [US3] [T] 编写 Token 脱敏单元测试 `src-tauri/src/utils/redact.rs#test`

  **目标**：覆盖各类 Token 模式。

  **实现要点**：
  - GitHub 形式：`ghp_*`、`gho_*`、`ghu_*`、`ghs_*`、`ghr_*`
  - GitLab 形式：`glpat-*`
  - URL 形式：`https://<user>:<token>@host`、`https://<token>@host`
  - Bearer 头：`Authorization: Bearer xxx`
  - 多种混合在长文本中

  **验收标准**：所有用例脱敏后无 token 残留。

- [x] T066 [US3] [T] 编写并发控制集成测试 `tests/integration/clone_concurrency_test.rs`

  **目标**：验证 semaphore 并发上限实际生效。

  **实现要点**：mock `git_cli_service::clone_repository` 为延时 1 秒的 future；
  创建 10 个任务 concurrency=3；断言同时刻 running 状态计数永远 ≤ 3。

  **验收标准**：测试通过。

- [x] T067 [US3] [T] 编写 Token 不泄漏端到端测试 `tests/integration/clone_token_safety_test.rs`

  **目标**：spec SC-009 / SC-010 的自动化验证。

  **实现要点**：本地起一个简单 Git HTTP 服务器（如 `git http-backend` over
  python http.server，或使用 `gitoxide-test` 助手），用 Token 认证 Clone 一次；
  Clone 完成后：
  - 读取 `.git/config`，断言 `remote.origin.url` 不含 token
  - 读取 operation_logs 表，断言所有 message/output 字段不含 token
  - 读取本地日志文件，断言无 token

  **验收标准**：测试通过；这是 MVP 安全验收的关键自动化用例。

---

**Checkpoint US3**: 多仓库批量 Clone 全链路可用；进度实时；Token 安全；成功后
自动入本地仓库列表。US1+US2+US3 构成 **MVP 第一可用版本**，可独立发布。

---

## Phase 6: User Story 4 — 本地仓库集中管理 (Priority: P2)

**Goal**: 用户能手动添加、扫描父目录批量添加、查看仓库状态、批量 Fetch、
打开目录或终端。

**Independent Test**: 选择父目录 `~/Projects` 扫描，识别所有 `.git`；刷新所有
状态，看到 ahead/behind/changed_files 正确显示；批量 Fetch 全部仓库不卡顿。

- [x] T068 [P] [US4] 实现 Git 仓库读取服务 `src-tauri/src/services/git_reader_service.rs`

  **目标**：解析仓库状态（不修改仓库）。

  **实现要点**：
  - `pub async fn status(repo_path: &Path) -> Result<RepoStatusSnapshot>`：
    运行 `git status --porcelain=v2 --branch -z`，解析每行得到
    `current_branch`、`upstream`、`ahead`、`behind`、`file_changes`
  - `pub async fn list_branches(repo_path: &Path) -> Result<Vec<Branch>>`：
    `git branch --all --format='%(refname:short)|%(upstream:short)|%(HEAD)'`
  - `pub async fn log(repo_path: &Path, page: u32, page_size: u32) ->
    Result<Vec<CommitInfo>>`：`git log --pretty=format:"%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s"
    --date=iso -n {page_size} --skip {offset}`
  - `pub async fn diff(repo_path: &Path, file: Option<&str>, cached: bool) ->
    Result<DiffResult>`：`git diff [--cached] -- {file}`；大文件检测（> 1 MB
    返回占位）
  - 所有方法不修改仓库，只读

  **验收标准**：在本仓库实测各方法返回符合预期。

- [x] T069 [US4] 实现 RepositoryStatus 推导逻辑于 `git_reader_service.rs`

  **目标**：将 status 原始数据映射到 spec 定义的 8 种状态枚举。

  **实现要点**：
  - `path_missing`: 路径不存在
  - `no_remote`: 无 upstream 远程
  - `detached_head`: HEAD 不在分支上
  - `conflict`: 文件状态含 UU/AA/DD
  - `modified`: changed_files > 0
  - `need_push`: ahead > 0 且 behind == 0
  - `need_pull`: behind > 0 且 ahead == 0
  - `clean`: 其他情况
  - 多状态共存时优先级：path_missing > conflict > detached_head > modified >
    need_push > need_pull > no_remote > clean

  **验收标准**：单元测试覆盖每种状态。

- [x] T070 [P] [US4] 实现本地仓库扫描与添加 service `src-tauri/src/services/repository_service.rs`（扩展）

  **目标**：实现 add / scan / remove 业务逻辑。

  **实现要点**：
  - `add_local_repository(path: &Path) -> Result<LocalRepository>`：校验路径存在、
    包含 `.git`、未重复加入；读取 status；写入 `local_repositories`
  - `scan_local_repositories(root: &Path) -> Result<Vec<LocalRepository>>`：
    使用 `walkdir` 遍历，目录深度限制（默认 5 层）；遇到 `.git` 即识别为
    仓库且不再继续深入（避免扫到子模块）；跳过已存在记录
  - `remove_local_repository(id: &str) -> Result<()>`：仅删除数据库行，**不**
    删除磁盘文件
  - `refresh_status(id: &str)` / `refresh_all_status()`：调用 git_reader_service
    更新状态

  **验收标准**：扫描包含 10 个仓库的目录速度 ≤ 5 秒；不重复添加。

- [x] T071 [US4] 实现批量 Fetch service `repository_service.rs`

  **目标**：并行 Fetch 多个仓库，单仓库失败不阻塞。

  **实现要点**：
  - `batch_fetch(ids: Vec<String>) -> Result<BatchFetchSummary>`：使用
    `tokio::task::JoinSet` spawn 多任务，并发上限 4
  - 每个仓库调用 `git_cli_service.run(&["fetch", "--all", "--prune"], cwd, env)`
  - 收集每个仓库的成功/失败结果到 summary
  - 失败原因记录到 operation_logs（含中文翻译，待 US6 完成集成）

  **验收标准**：10 个仓库批量 fetch，1 个故意指向不存在远程；其他 9 个成功，
  失败结果汇总展示。

- [x] T072 [US4] 实现本地仓库 Tauri commands `src-tauri/src/commands/local_repositories.rs`

  **目标**：注册 8 个命令：`add_local_repository`、`scan_local_repositories`、
  `list_local_repositories`、`remove_local_repository`、
  `refresh_local_repository_status`、`refresh_all_local_repository_status`、
  `open_repository_folder`、`open_repository_in_terminal`、`batch_fetch_repositories`。

  **实现要点**：
  - `open_repository_folder(id)` 调用 `tauri-plugin-shell` 的 `open` 打开目录
  - `open_repository_in_terminal(id)` 平台分支：macOS 调用
    `open -a Terminal {path}`；Windows 调用 `wt -d {path}` 或 `cmd /c start ...`；
    Linux 优先 `gnome-terminal`/`xterm`/`konsole`，按可用性兜底
  - 删除前在前端用 `ConfirmDangerDialog` 二次确认（Principle III）

  **验收标准**：8 个命令均注册；前端调用有效。

- [x] T073 [P] [US4] 创建前端本地仓库 API、store、页面

  **目标**：`src/api/localRepository.api.ts`、`src/stores/localRepository.ts`、
  `src/pages/LocalRepositories.vue`、`src/components/repository/LocalRepoTable.vue`

  **实现要点**：
  - api 封装上述 8 个命令
  - store state：`repos: LocalRepository[]`、`scanning`、`refreshing`
  - 页面布局：顶部"添加仓库"+"扫描目录"+"刷新所有状态"+"批量 Fetch"按钮
  - 表组件列：选择框、仓库名、本地路径、当前分支、远程地址、平台 Badge、
    所属账号、状态 Tag、未提交文件数、ahead/behind、最后打开时间、操作
    （打开仓库 / Fetch / Pull / Push / 打开目录 / 打开终端 / 从列表移除）
  - "从列表移除"按钮触发 `ConfirmDangerDialog`，强调"不删除磁盘文件"

  **验收标准**：500 个仓库虚拟滚动；状态刷新不阻塞 UI。

- [x] T074 [US4] 实现 RepoStatusOverview 组件 `src/components/repository/RepoStatusOverview.vue`

  **目标**：单仓库的状态总览（用于首页仪表盘与详情）。

  **实现要点**：展示状态 Tag、当前分支、ahead/behind 计数、未提交文件数；
  使用 props 接收 LocalRepository。

  **验收标准**：用于 RepositoryDetail.vue 顶栏与 Dashboard.vue 卡片。

- [x] T075 [US4] [T] 编写本地仓库扫描集成测试 `tests/integration/repository_scan_test.rs`

  **目标**：验证扫描逻辑正确性。

  **实现要点**：使用 `tempfile::TempDir` 创建临时目录结构，含 3 个 `.git` 仓库
  + 1 个子模块 + 1 个非 git 目录；运行 scan 后断言识别出 3 个仓库且不重复。

  **验收标准**：测试通过。

- [x] T076 [US4] [T] 编写 status 推导单元测试

  **目标**：覆盖 8 种状态的推导逻辑。

  **实现要点**：在 `git_reader_service.rs` 内 `#[cfg(test)]`，使用 `tempfile`
  创建仓库并模拟各种状态（无 remote、detached、conflict、modified、ahead、
  behind）。

  **验收标准**：测试通过。

---

**Checkpoint US4**: 本地仓库可管理与状态可视化；为 US5 提供入口。

---

## Phase 7: User Story 5 — 单仓库可视化 Git 工作流 (Priority: P2)

**Goal**: 用户在仓库工作区视图完成"查看变更 → diff → stage → commit → push"
完整流程，并切换分支。

**Independent Test**: 打开仓库，修改文件，查看 diff 高亮；stage 一个文件，输入
commit message 提交成功；执行 push 远程更新。

- [x] T077 [US5] 扩展 git_cli_service：stage / unstage / commit / discard

  **目标**：实现仓库的写入类 Git 操作。

  > 注：T077/T078/T079 同样修改 `src-tauri/src/services/git_cli_service.rs`
  > 同一文件，不再标 `[P]`；建议同一开发者顺序完成，或通过 PR 序列化合并
  > 避免合并冲突。

  **实现要点**：
  - `stage_file(repo, file)`：`git add -- {file}`
  - `stage_all(repo)`：`git add -A`
  - `unstage_file(repo, file)`：`git restore --staged -- {file}`
  - `unstage_all(repo)`：`git restore --staged .`
  - `commit(repo, message, description)`：组合 message + description 到
    临时文件 `.git/COMMIT_GITVIEW`，调用 `git commit -F {file} --cleanup=strip`，
    成功后**删除临时文件**（注意：临时文件位于 `.git/` 内，Principle III
    允许 Git 自身的 housekeeping）
  - `discard_changes(repo, files, confirmed: bool)`：仅当 `confirmed = true`
    时执行 `git checkout -- {files}` 或对未跟踪文件 `git clean -fd -- {files}`；
    `confirmed = false` 时直接返回 `UserCancelled`（双重防御，确保只有通过
    前端 `ConfirmDangerDialog` 才能触发；对应 Principle III）

  **验收标准**：本地实测各操作；commit 多行 message 与中文内容正确入库；
  discard 无 confirmed 标志时返回 UserCancelled。

- [x] T078 [US5] 扩展 git_cli_service：fetch / pull / push

  **目标**：网络相关 Git 操作（同文件，依赖 T077 完成）。

  **实现要点**：
  - `fetch(repo)`：`git fetch --all --prune`
  - `pull(repo)`：`git pull --ff-only` 默认；如失败检测原因（non-fast-forward
    / conflict）并返回 error code 让前端友好提示
  - `push(repo)`：`git push`；解析常见拒绝原因（non-fast-forward / no upstream /
    permission denied）映射到 `GitViewError`
  - 输出脱敏：在写入 operation_logs 前调用 `redact::redact_token`

  **验收标准**：本地实测 fetch/pull/push；故意触发非快进时返回正确错误码。

- [x] T079 [US5] 扩展 git_cli_service：list_branches / checkout / create_branch

  **目标**：分支管理基础操作（同文件，依赖 T078 完成）。

  **实现要点**：
  - `list_branches(repo) -> Vec<Branch>`：基于 git_reader 已有实现
  - `checkout_branch(repo, name)`：`git checkout {name}`；**调用前必先调用
    `git_reader::status()` 校验脏工作区**，存在未提交变更时返回
    `DirtyWorkdir` 错误而非直接执行（前端按错误码 disable 按钮并 tooltip
    提示，对应 FR-044 与 T086）
  - `create_branch(repo, name, checkout)`：`git checkout -b {name}` 或
    `git branch {name}`
  - 远程分支 checkout：`git checkout -b {local} {remote_branch}`，自动设置
    upstream

  **验收标准**：本地实测 branch 列表、切换、创建；脏工作区下切换分支返回
  `DirtyWorkdir`。

- [x] T080 [US5] 实现 Git Tauri commands `src-tauri/src/commands/git.rs`

  **目标**：注册 **15 个 git_* command**。

  **实现要点**：完整列表（15 个）：`git_status`、`git_diff`、`git_stage_file`、
  `git_unstage_file`、`git_stage_all`、`git_unstage_all`、`git_commit`、
  `git_fetch`、`git_pull`、`git_push`、`git_list_branches`、
  `git_checkout_branch`、`git_create_branch`、`git_log`、`git_discard_changes`。

  每个 command 参数包含 `repo_id`（从 local_repositories 查 path），转发到
  service 层。`git_discard_changes` 额外要求 `confirmed: bool` 参数（与 T077
  的服务层校验对应；前端 ConfirmDangerDialog 通过后才传 `confirmed: true`）。

  **验收标准**：**15 个命令**均注册且 capabilities 允许；`git_discard_changes`
  无 confirmed=true 时返回 UserCancelled。

- [x] T081 [US5] 实现 commit 前置校验逻辑 `account_service.rs` 或 `git_cli_service.rs`

  **目标**：spec FR-038 / Acceptance Scenarios，提交前校验 5 项。

  **实现要点**：
  - 已暂存文件 > 0
  - message 非空
  - 仓库非 conflict 状态
  - 仓库非 detached HEAD
  - 全局或本地 `user.name` 与 `user.email` 已配置（缺一则 commit 前阻断）
  - 任一未通过时返回 `GitViewError::Internal` 含中文原因；前端显示给用户

  **验收标准**：5 项校验单元测试均覆盖。

- [x] T082 [P] [US5] 实现前端 Git API 与 RepositoryDetail 页面骨架

  **目标**：`src/api/git.api.ts` + `src/pages/RepositoryDetail.vue` 三栏布局。

  **实现要点**：
  - api 封装 14 个 command
  - 路由：`/repositories/:id` → RepositoryDetail.vue，根据 id 从
    localRepositoryStore 取 repo
  - 三栏布局：左侧文件变更（GitFileChanges.vue）、中间 Diff 查看器（DiffViewer.vue）、
    右侧/底部 Commit 面板（CommitPanel.vue）
  - 顶栏：仓库名、当前分支（BranchSelector.vue）、ahead/behind、远程地址、
    Fetch/Pull/Push 按钮、打开目录、打开终端

  **验收标准**：路由可达；三栏布局可见。

- [x] T083 [US5] 实现文件变更列表组件 `src/components/git/GitFileChanges.vue`

  **目标**：显示 modified / added / deleted / renamed / untracked / conflict 文件。

  **实现要点**：
  - 区分"已暂存"与"未暂存"两个段落
  - 每行：状态字母（M/A/D/R/U/C）+ 文件路径
  - 操作：单击切换 stage/unstage、右键菜单（打开文件、在文件管理器中显示、
    discard changes）
  - **discard changes 必经流程**：右键 → ConfirmDangerDialog 弹出 → 显示
    将丢弃的文件路径清单与"不可恢复"警告 → 用户输入仓库名作为二次确认关键词
    → 仅在确认通过后调用 `gitApi.discardChanges({ repoId, files, confirmed: true })`
    （Principle III 实现）
  - "全部暂存"与"全部取消暂存"按钮
  - 状态过滤器（只看 modified / 只看 untracked）

  **验收标准**：与设计文档 §11.4 一致；UI 流畅；discard 流程在
  ConfirmDangerDialog 未通过时**不**触发任何 API 调用。

- [x] T084 [US5] 实现 Diff 查看器组件 `src/components/git/DiffViewer.vue`

  **目标**：渲染统一 diff 格式，高亮新增/删除行。

  **实现要点**：
  - V1 使用纯文本渲染 + CSS 行类：`.diff-add` 绿色背景 / `.diff-del` 红色背景 /
    `.diff-context` 默认 / `.diff-hunk` 灰色头部
  - 大文件（> 1 MB）显示提示："文件过大不予显示，请使用外部工具查看"
  - 二进制/图片文件占位提示
  - 文件 > 5000 行启用虚拟滚动（V1 简化：直接 max-height + overflow）

  **验收标准**：渲染本仓库一个真实 diff 显示正确。

- [x] T085 [US5] 实现 Commit 面板组件 `src/components/git/CommitPanel.vue`

  **目标**：commit message 与 description 输入，提交按钮。

  **实现要点**：
  - 字段：message（el-input 单行，必填）、description（el-input textarea
    可选）
  - 字符计数显示
  - 提交按钮：禁用状态条件（无暂存文件 / message 空 / 用户配置缺失）
  - 提交成功后 emit `committed` 事件供父组件刷新

  **验收标准**：手工测试提交流程。

- [x] T086 [US5] 实现分支选择器组件 `src/components/git/BranchSelector.vue`

  **目标**：分支切换与新建（实现 FR-044 脏工作区阻断）。

  **实现要点**：
  - `el-dropdown` 显示当前分支名
  - 下拉列表：本地分支段 + 远程分支段（远程分支以 `origin/` 前缀）
  - **脏工作区检测与切换按钮 disable**：
    - 组件 mounted 时调用 `gitApi.status(repoId)` 获取当前状态
    - 当 `changed_files > 0` 或 status 为 `modified/conflict/detached_head` 时，
      切换分支按钮设为 `disabled = true`，并通过 `el-tooltip` 显示
      "存在未提交变更，请先提交或暂存 (stash) 后再切换分支"
    - `git_checkout_branch` 调用返回 `DirtyWorkdir` 错误时也复用同一 tooltip
      文案显示
    - 监听 `committed` / `stashed` 等事件刷新可切换状态
  - "从此分支创建本地分支"用于远程分支
  - "新建分支"对话框：输入名称 + 是否切换过去（新建分支不受脏工作区阻断）

  **验收标准**：与设计文档 §11.8 V1 范围一致；脏工作区下切换按钮 disable
  且 tooltip 文案准确（对应 spec Acceptance Scenario 11 与 FR-044）。

- [x] T087 [US5] 实现 CommitHistory 组件（V1 简版）`src/components/git/CommitHistory.vue`

  **目标**：spec FR-? 与设计文档 §11.9 V1 显示简单提交列表。

  **实现要点**：
  - 调用 `git_log(repo_id, page, page_size=50)` 分页加载
  - 每行：hash 短码、message、author、date
  - 滚动到底部自动加载下一页

  **验收标准**：本仓库 commit 列表可加载且分页正确。

- [x] T088 [US5] 实现 Pull 冲突与 Push 拒绝错误提示 UI

  **目标**：spec Acceptance Scenarios 10/11 的中文友好提示。

  **实现要点**：
  - Pull 冲突 → `el-message-box` 警告："Pull 后存在冲突，请使用外部工具（如
    VS Code / git mergetool）解决冲突后再提交"
  - Push 被拒（non-fast-forward）→ "推送被拒绝（远程有新提交），请先 Pull"
  - Push 被拒（permission）→ "Token 权限不足，请确认 PAT 含 repo:write 权限"
  - Push 无 upstream → 提供"创建 upstream 分支"按钮调用 `git push -u origin {branch}`

  **验收标准**：手工触发各场景显示对应文案。

- [x] T089 [US5] 接入 RepositoryDetail.vue 路由从 LocalRepositories.vue 打开

  **目标**：从本地仓库列表的"打开仓库"按钮跳转到详情页。

  **实现要点**：`<router-link to="`/repositories/${repo.id}`">`；进入时
  localRepositoryStore 更新 `last_opened_at`。

  **验收标准**：跳转后 RepositoryDetail.vue 自动加载该仓库数据。

- [x] T090 [US5] [T] 编写 commit 临时文件提交单元测试

  **目标**：验证多行 message 与中文字符正确提交。

  **实现要点**：`tempfile` 创建临时 git 仓库，调用 `git_cli_service::commit` 提交
  包含换行与中文的 message，然后 `git log --format=%B` 读取并比对。

  **验收标准**：内容一致。

- [x] T091 [US5] [T] 编写 commit 前置校验单元测试

  **目标**：覆盖 5 项校验。

  **实现要点**：分别构造缺失条件的状态，断言返回对应错误。

  **验收标准**：每项校验通过。

- [x] T092 [US5] [T] 编写 git status 解析单元测试

  **目标**：验证 porcelain v2 解析。

  **实现要点**：喂入真实 porcelain v2 输出样本字符串，断言 file_changes 数组
  与 ahead/behind 计数正确。

  **验收标准**：测试通过。

---

**Checkpoint US5**: 单仓库 Git 完整工作流可用；US1+US2+US3+US4+US5 构成
**完整 MVP 闭环**。

---

## Phase 8: User Story 6 — 操作日志与问题诊断 (Priority: P3)

**Goal**: 用户能查看所有 Git 操作日志，Token 已脱敏，常见错误有中文友好提示。

**Independent Test**: 触发多种 Git 操作后日志列表可见；故意触发认证失败，日志
中无 Token；常见错误显示中文翻译。

- [ ] T093 [P] [US6] 实现操作日志 service `src-tauri/src/services/log_service.rs`

  **目标**：记录、查询、清理操作日志，包含敏感信息脱敏。

  **实现要点**：
  - `record(op_type, repo_id, command, status, output, error, duration)`：
    在写入前对 `command` / `output` / `error` 统一调用 `redact::redact_token`
  - `list(filter: LogFilter) -> Vec<OperationLog>`：支持按 op_type、status、
    repo_id、时间范围筛选；分页 page+page_size
  - `get_detail(id) -> Option<OperationLog>`：含完整 output/error
  - `clear_older_than(days: u32) -> usize`：删除指定天数前的日志（删除属
    Principle III 范围，需在设置中由用户启用并设阈值）
  - `translate_error(raw_error: &str) -> Option<String>`：spec FR-048
    至少 5 类错误中文翻译

  **验收标准**：单元测试覆盖：记录、脱敏、查询、翻译。

- [ ] T094 [US6] 实现错误翻译映射表 `src-tauri/src/services/log_service.rs`（内部表）

  **目标**：spec FR-048 中文友好提示。

  **实现要点**：
  ```rust
  /// 常见错误原始消息 → 中文翻译映射。
  /// 使用 `contains` 匹配以容忍 Git 版本差异。
  static ERROR_TRANSLATIONS: &[(&str, &str)] = &[
      ("Authentication failed", "认证失败，请检查 Token、SSH Key 或账号权限。"),
      ("Repository not found", "仓库不存在，或当前账号没有访问权限。"),
      ("Permission denied (publickey)", "SSH Key 未配置或无权限，请检查本机 SSH 配置。"),
      ("Could not resolve host", "无法解析主机，请检查网络连接、DNS 或代理设置。"),
      ("path already exists and is not an empty directory", "目标目录已存在且不为空，请更换目录或选择跳过该仓库。"),
      ("non-fast-forward", "推送被拒绝（远程有新提交），请先执行 Pull。"),
      ("The TLS connection was non-properly terminated", "TLS 连接中断，请检查网络与证书配置。"),
  ];
  ```

  **验收标准**：单元测试覆盖每条翻译命中。

- [ ] T095 [US6] 集成日志记录到现有 services

  **目标**：在 account / clone / repository / git 各 service 的关键操作处
  调用 `log_service::record`。

  **实现要点**：
  - account_service：add/delete/test_connection
  - clone_task_service：每个任务的 start/finish/error
  - repository_service：scan、add、remove
  - git_cli_service：每个 Git 命令执行后（commit、push、pull、fetch、checkout）
  - 操作类型枚举 `OperationType` 在 models 中定义
  - 通过依赖注入或全局 `LOG_SERVICE` 单例访问

  **验收标准**：触发各类操作后日志表均有对应行；token 被脱敏。

- [ ] T096 [US6] 实现日志相关 Tauri commands `src-tauri/src/commands/logs.rs`

  **目标**：`list_operation_logs`、`get_operation_log_detail`、
  `clear_old_operation_logs`。

  **实现要点**：`clear_old_operation_logs` 在前端调用前需弹出
  `ConfirmDangerDialog`（涉及数据删除，Principle III）。

  **验收标准**：3 个命令均注册并可调用。

- [ ] T097 [P] [US6] 创建前端日志 API、store、页面

  **目标**：`src/api/logs.api.ts`、`src/pages/Logs.vue`、日志详情对话框。

  **实现要点**：
  - 页面顶部筛选：操作类型多选、状态多选、仓库下拉、时间范围
  - 表格列：时间、操作类型 Tag、仓库名、状态 Tag、耗时、命令摘要、操作（详情）
  - 点击行打开日志详情对话框：完整命令、output、error（已脱敏），下方显示
    `translated_error_message`（若有）
  - 顶部"清理旧日志"按钮触发 ConfirmDangerDialog（确认天数后执行）

  **验收标准**：与设计文档 §15 一致；10000 条日志虚拟滚动可用。

- [ ] T098 [US6] [T] 编写敏感信息脱敏端到端测试 `tests/integration/log_redaction_test.rs`

  **目标**：触发若干带 token 的操作后扫描 operation_logs 表无 token 明文。

  **实现要点**：mock provider 返回带 token URL 的错误；执行 add_account 失败；
  查询日志表验证 `output` 与 `error` 字段中无 token；扫描日志文件同样验证。

  **验收标准**：测试通过；可与 T067 共享部分逻辑。

---

**Checkpoint US6**: 操作日志完整覆盖；安全审计通过。

---

## Phase 9: User Story 7 — 设置与默认目录管理 (Priority: P3)

**Goal**: 用户能集中管理 Git 环境、默认目录、Clone 协议、并发、主题、网络代理。

**Independent Test**: 修改各项设置后重启应用，设置保持生效；批量 Clone 对话框
预填来自设置的默认值。

- [ ] T099 [P] [US7] 实现设置 service `src-tauri/src/services/settings_service.rs`

  **目标**：键值对读写，提供类型化 getter/setter。

  **实现要点**：
  - 内部表 settings(key, value, updated_at)
  - 提供分组化的强类型 API：`get_general()` / `set_general()`、
    `get_git()` / `set_git()`、`get_network()` / `set_network()`、
    `get_external_tools()` / `set_external_tools()`
  - 默认值：default_project_directory = `~/Projects`、clone_protocol = HTTPS、
    concurrency = 3、directory_strategy = ByPlatformAndAccount、theme = System、
    language = zh、auto_start_check = true
  - 序列化：value 字段统一存 JSON 字符串

  **验收标准**：单元测试覆盖 get/set 跨重启。

- [ ] T100 [US7] 实现 Git 检测 service 扩展 `git_cli_service.rs`

  **目标**：`detect_git` 与 `set_git_path` 两个 command 的业务逻辑。

  **实现要点**：
  - `detect_git()`：先尝试设置中保存的自定义路径，再 `which git`/`where git`，
    再常见安装位置（macOS `/usr/bin/git`、`/opt/homebrew/bin/git`；Windows
    `C:\Program Files\Git\bin\git.exe`；Linux `/usr/bin/git`）
  - 返回 `GitDetectionResult { found, path, version, user_name, user_email }`
  - `set_git_path(path)` 校验路径存在并可执行 `git --version`

  **验收标准**：未安装 Git 的环境（mock PATH）返回 `found=false`。

- [ ] T101 [US7] 实现设置 Tauri commands `src-tauri/src/commands/settings.rs`

  **目标**：`get_settings`、`update_settings`、`detect_git`、`set_git_path`。

  **实现要点**：薄包装层。

  **验收标准**：4 个命令均注册。

- [ ] T102 [P] [US7] 创建前端设置 API、store、页面

  **目标**：`src/api/settings.api.ts`、`src/stores/settings.ts`、
  `src/pages/Settings.vue`

  **实现要点**：
  - 页面使用 `el-tabs`：通用 / Git / 网络 / 外部工具 / 账号与安全
  - 通用：启动时打开上次仓库、启动时自动检查仓库状态、默认项目目录、默认 Clone
    协议、默认并发数、默认目录组织方式、语言、主题
  - Git：Git 可执行文件路径（含"检测 Git"按钮，调用 detect_git）、user.name、
    user.email、默认 pull/push 策略
  - 网络：HTTP/HTTPS Proxy、使用系统代理、API 超时、Clone 超时
  - 外部工具：默认编辑器、默认终端、默认文件管理器、VS Code/Cursor/JetBrains
    打开仓库的命令
  - **账号与安全（FR-055 实现）**：
    - 列出每个账号一行：用户名、平台 Badge、凭据状态列
    - 凭据状态：调用 `accountApi.checkCredentialExists(accountId)` 即新增的
      Tauri 命令包装 `credential_service::token_exists`；返回 true 显示
      绿色 Tag "已存储"，返回 false 显示红色 Tag "凭据缺失"（伴 `el-tooltip`
      "Token 已从安全存储中丢失，请使用 ‘重新验证’ 修复"）
    - **绝不显示 token 明文**——只显示存在性布尔
    - "重新验证"按钮：弹出对话框输入新 Token → 调用 `test_account_connection`
      → 成功后调用 `accountApi.saveCredential(accountId, newToken)`
    - "删除凭据"按钮触发 `ConfirmDangerDialog`（删凭据但保留账号元数据），
      用户输入用户名二次确认
  - 后端补充任务：在 commands/accounts.rs 中新增 `check_credential_exists`、
    `save_credential` 两个 Tauri 命令封装 credential_service 对应方法（计入
    本任务实现范围，**不算额外命令**因为已在 T023 服务层实现）

  **验收标准**：与设计文档 §14 完全一致；修改后刷新页面仍保持；凭据状态列
  对手动从系统 keyring 删除 token 的账号正确显示"凭据缺失"。

- [ ] T103 [US7] 实现主题切换逻辑 `src/stores/app.ts` + `src/styles/`

  **目标**：light / dark / system 三主题切换。

  **实现要点**：
  - 应用根使用 `[data-theme="light|dark"]` 属性
  - Element Plus 暗色主题：动态引入 `element-plus/theme-chalk/dark/css-vars.css`
  - "跟随系统"使用 `matchMedia('(prefers-color-scheme: dark)')` 监听变化
  - settings 中保存当前主题；应用启动时恢复

  **验收标准**：切换 3 种主题视觉立即变化；重启后保持。

- [ ] T104 [US7] 实现简易 i18n 骨架 `src/i18n/`

  **目标**：spec 要求中/英文切换，V1 仅提供骨架与中文文案。

  **实现要点**：
  - `npm install vue-i18n@9`
  - 创建 `src/i18n/index.ts`、`src/i18n/zh.ts`、`src/i18n/en.ts`（en 文案先复制
    中文，后续翻译，V1 不强求完整英文）
  - `app.use(i18n)` 注册
  - 关键文案使用 `$t('...')` 替换；V1 优先覆盖标题、按钮、错误提示

  **验收标准**：设置中切换语言后立即更新标签文本。

- [ ] T105 [US7] 实现代理配置生效 `src-tauri/src/services/`（跨服务）

  **目标**：spec FR-058，所有远程调用按代理设置生效。

  **实现要点**：
  - `reqwest::ClientBuilder::proxy(reqwest::Proxy::all(&proxy_url)?)`
  - `git_cli_service::run` 时注入环境变量 `HTTP_PROXY` / `HTTPS_PROXY`
  - "使用系统代理" 开关时使用 `reqwest::Proxy::no_proxy()` 让 reqwest 跟随
    系统设置
  - 每次设置变更后通过事件 `settings-changed` 通知后端重建 client

  **验收标准**：本地起 HTTP 代理（如 `mitmproxy`），设置后捕获到所有 API 与
  Git 流量。

- [ ] T106 [US7] [T] 编写设置持久化集成测试 `tests/integration/settings_persistence_test.rs`

  **目标**：spec Acceptance Scenarios 验证。

  **实现要点**：set 多个键值 → 读出验证 → 关闭数据库连接再打开 → 读出验证。

  **验收标准**：测试通过。

---

**Checkpoint US7**: 设置中心完整可用。

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: 完成首页仪表盘、跨平台打包、安全审计、性能基线、文档与发布物。

- [ ] T107 [P] 实现首页仪表盘页面 `src/pages/Dashboard.vue`（spec FR-059）

  **目标**：聚合展示账号数、远程仓库数、本地仓库数、运行中 Clone 数、
  有未提交变更的仓库数、最近打开仓库、最近任务、快捷入口。

  **实现要点**：
  - 顶部 `el-row` 卡片 8 个统计指标（`el-statistic`）
  - 中部"最近打开仓库" `el-table`，最多 5 条
  - 中部"最近任务" `el-table`，最多 5 条
  - 底部快捷按钮：添加账号、从远程 Clone、添加本地仓库、扫描目录、打开设置

  **验收标准**：与设计文档 §6 一致；数据来自现有 stores。

- [ ] T108 [P] 完善 Tauri 应用元数据与图标 `src-tauri/tauri.conf.json` 与 `src-tauri/icons/`

  **目标**：应用名、版本、版权、图标按平台齐全。

  **实现要点**：
  - 准备 `icon.png` 1024x1024
  - 使用 `npm run tauri icon` 生成各平台图标
  - tauri.conf.json：`productName`、`version`、`copyright`、`category`
  - macOS bundle identifier、Windows MSI/NSIS 设置

  **验收标准**：`npm run tauri build` 在三平台生成可分发包。

- [ ] T109 实现首次启动引导 `src/App.vue` 与 `src/router/index.ts`

  **目标**：spec User Story 7 Acceptance Scenario 1，未安装 Git 时引导。

  **实现要点**：
  - 启动时调用 `detect_git`
  - 若 `found=false`：路由守卫拦截除 Settings 与引导页之外的所有路由，
    显示安装引导（macOS Homebrew、Windows 官网链接、Linux apt/yum 命令）
  - 检测 keyring 可用性：不可用时提示用户安装 `gnome-keyring`/`libsecret`

  **验收标准**：手工卸载 Git 重启应用应进入引导态。

- [ ] T110 添加跨平台构建脚本与 release workflow `.github/workflows/release.yml`

  **目标**：tag 触发自动构建 macOS .dmg / Windows .msi / Linux .deb / .AppImage。

  **实现要点**：
  - 触发：`push` 到 tag `v*`
  - 矩阵：三平台
  - 步骤：checkout、setup、安装平台依赖（Linux 上 `libwebkit2gtk-4.1-dev` 等）、
    `npm ci`、`npm run tauri build`、上传 artifact 到 GitHub Release
  - macOS notarize 与 Windows 签名待 V2 引入；**V1 用户首次运行未签名的
    `.dmg`/`.msi` 时会触发 macOS Gatekeeper 警告（"应用来自未知开发者"）
    与 Windows SmartScreen 警告**——需在 T111 README 中显式说明绕过方法
    （macOS：右键 → 打开；Windows：更多信息 → 仍要运行）

  **验收标准**：手工创建测试 tag 触发构建，artifact 可下载安装。

- [ ] T111 [P] 编写 README.md 中文文档

  **目标**：项目简介、安装、开发、Built with、License、首次运行警告说明。

  **实现要点**：
  - 项目简介、产品定位（来自 spec.md）
  - 截图：账号管理、远程仓库、批量 Clone、单仓库工作区（占位，待 V1 完成后补）
  - **安装与首次运行**：
    - macOS：双击 `.dmg` 拖入 Applications；首次运行可能提示"无法验证开发者"，
      右键应用图标 → 打开 → 确认；V1 暂未做 Apple notarize 公证
    - Windows：双击 `.msi` 安装；SmartScreen 警告时点"更多信息" → "仍要运行"；
      V1 暂未做代码签名
    - Linux：双击 `.deb` 或 `.AppImage`；可能需先安装 `libsecret`/`gnome-keyring`
  - 开发：`npm install` → `npm run tauri dev`
  - 构建：`npm run tauri build`
  - 测试：`npm test` 与 `cargo test`
  - Built with：Tauri 2 + Vue 3 + Rust + Element Plus
  - License：占位（用户后续决定）

  **验收标准**：README 完整可读；CI 不破坏；首次运行警告说明清晰。

- [ ] T112 跨平台手工 MVP 验收清单走查（spec SC-018 全部 20 项）

  **目标**：在 macOS、Windows、Ubuntu 三平台分别执行 SC-018 全部 20 项验收
  用例，记录结果。

  **实现要点**：
  - 在 `specs/001-gitview-mvp/` 下创建 `acceptance-report.md` 用于记录结果
  - 每项用例：步骤 + 预期 + 三平台实测 + Pass/Fail
  - 若有 Fail 立即修复并重测

  **验收标准**：20/20 在三平台全部 Pass，作为 MVP 发布门禁。

- [ ] T113 [P] 性能基线验证（spec SC-005 / SC-006 / SC-007 / SC-008）

  **目标**：基准测试关键性能指标。

  **实现要点**：
  - SC-005：种入 500 个本地仓库，刷新所有状态，记录耗时与 UI 流畅度
  - SC-006：种入 5000 条远程仓库，渲染列表，搜索过滤计时
  - SC-007：种入 10000 条操作日志，列表加载与筛选计时
  - SC-008：10 个中型仓库批量 Clone 总耗时
  - 数据收集脚本：`scripts/perf/seed-data.sh`、`scripts/perf/measure.ts`
  - 结果写入 `specs/001-gitview-mvp/perf-baseline.md`

  **验收标准**：四项指标均满足或在 acceptance-report.md 记录可接受偏差。

- [ ] T114 [P] 自动化安全审计：Token 泄漏扫描 + 凭据残留检查

  **目标**：spec SC-009 / SC-010 / SC-011 自动化验证。

  **实现要点**：
  - 运行 `scripts/check-no-token-leak.sh` 扫描数据库与日志
  - 编写 `scripts/check-credential-cleanup.sh`：调用 `keyring` 列出 `gitview`
    服务下的所有键，断言每个键对应账号在数据库中均存在；删除账号后键被清理
  - 加入 release workflow 作为发布门禁

  **验收标准**：脚本通过；CI 中可执行。

- [ ] T115 [P] 编写 V2/V3/V4 路线图 stub `docs/roadmap.md`

  **目标**：明确 V1 不做什么，V2+ 何时做。

  **实现要点**：基于产品设计文档 §5.2-5.4 转写为 markdown roadmap；列出每个
  能力的优先级、预估发布版本、当前状态。

  **验收标准**：文档可读；与 spec.md Assumptions 章节保持一致。

- [x] T116 [P] 创建 ConfirmDangerDialog 共用组件 `src/components/common/ConfirmDangerDialog.vue`

  **目标**：spec FR-? + Principle III 删除操作统一 UI。

  **实现要点**：
  - props：`title`、`message`、`itemsToDelete?: string[]`、`recoverabilityHint?: string`、
    `confirmKeyword?: string`（要求用户输入关键词二次确认）
  - emits：`confirm`、`cancel`
  - 视觉：红色警告头、列出删除项、要求用户输入指定关键词后才启用确认按钮
  - 在以下地方复用：删除账号、discard changes、清空旧日志、删除凭据、清理
    Clone 半成品目录（取消任务时）、批量 Fetch 时被识别为危险操作（无此场景，
    仅 5 个使用点）

  **验收标准**：在 5 个使用点验证生效。

- [x] T117 实现应用启动期未完成任务回扫 `src-tauri/src/main.rs` + `services/clone_task_service.rs`

  **目标**：spec SC-013 自动化保障——应用关闭/崩溃后重启时，将上次会话遗留
  的"伪运行中"任务统一标记为 interrupted/failed，避免出现"幽灵任务"。

  **实现要点**：
  - 在 `clone_task_service` 中新增 `pub async fn reconcile_orphan_tasks() ->
    Result<ReconcileSummary>`：
    1. 启动事务，`UPDATE clone_tasks SET status = 'failed', error_message =
       'Application terminated unexpectedly while task was running'
       WHERE status IN ('running', 'pending')` 并记录受影响行数
    2. 对于这些任务的目标目录，**不主动删除**（避免误删用户工作成果，
       符合 Principle III 即使是临时目录也由用户决定）；改为将目录路径写入
       新表字段或操作日志，由用户在 Clone 中心通过"清理半成品目录"按钮显式
       选择清理
    3. 同时清理临时 askpass 脚本残留：扫描 `std::env::temp_dir()` 下匹配
       `gitview-askpass-*` 的文件并删除（这些是 T056 的临时凭据脚本，本质
       属于本应用 housekeeping，无需用户确认）
  - 在 `main.rs` 的 `setup` 闭包中，于数据库迁移之后、Tauri 窗口显示之前
    调用 `reconcile_orphan_tasks()`，并使用 `tracing::info!` 记录回扫结果
  - 失败时仅记录 warn 日志，不阻断应用启动

  **验收标准**：手工方式构造测试场景——
  1. 在数据库中预置一条 `status = 'running'` 的 clone_tasks 记录
  2. 启动应用
  3. 验证该记录变为 `status = 'failed'`，`error_message` 含"unexpectedly"
  4. 用户在 Clone 中心可看到该任务被标记为 failed 而非 running

---

## Dependencies & Execution Order

### Phase Dependencies

```
Setup (Phase 1)
   ↓
Foundational (Phase 2) ─── BLOCKING ALL USER STORIES
   ↓
US1 (Phase 3) ─┐
US2 (Phase 4) ─┤  P1 — 并行可启动（受 Foundational 制约）
US3 (Phase 5) ─┘
   ↓ （依赖 US2 数据 + US3 队列）
US4 (Phase 6) ─┐
US5 (Phase 7) ─┘  P2
   ↓
US6 (Phase 8) ─┐
US7 (Phase 9) ─┘  P3
   ↓
Polish (Phase 10)
```

### User Story Dependencies

- **US1 (P1)**: 无前置依赖（Foundational 完成后可启动）
- **US2 (P1)**: 依赖 US1 的 Provider 抽象（trait T025）；其他独立
- **US3 (P1)**: 依赖 US2 的 remote_repositories 数据；可与 US4 部分并行（US4
  的扫描功能不依赖 US3）
- **US4 (P2)**: 依赖 US3 的"clone 成功后自动加入本地仓库"（T060）；自身扫描
  功能不强依赖
- **US5 (P2)**: 依赖 US4 的本地仓库入口；自身 Git 操作独立
- **US6 (P3)**: 横切关注点，最佳时机是 US1-US5 部分完成后再集成（T095 集成步骤
  贯穿前面所有 service）
- **US7 (P3)**: 独立，可与其他 P3 并行

### Within Each User Story

- Models → Services → Commands → 前端 API → Pinia store → 页面/组件
- 测试任务可在对应 service/工具完成后立即编写

### Parallel Opportunities

- **Phase 1**：T002-T010 全部 [P]，可并行
- **Phase 2**：T013-T024 中除 T013（errors.rs）与 T019（main.rs 状态注入）需先行外，
  T014-T018 均 [P]
- **Phase 3 US1**：T025-T028（三大平台 Provider）完全独立 [P]；T034-T038（前端
  API/store/页面/组件）可与后端 T030-T033 并行
- **Phase 4 US2**：T042（list_repositories trait 扩展）三平台并行；T046-T050
  前端组件并行
- **Phase 5 US3**：T053-T058（CLI 检测、Clone 命令、进度解析、凭据注入、队列、
  目录策略）部分串行（T054 依赖 T053、T056 依赖 T054）；前端 T061-T063 可并行
- **Phase 6 US4**：T068（git_reader）+ T070（scan）+ T071（batch_fetch）可三个开发
  者并行；前端 T073 可并行
- **Phase 7 US5**：T077/T078/T079 同文件 git_cli_service.rs 顺序完成（已**移除 [P]**
  避免合并冲突）；前端 T083/T084/T085/T086/T087 全部 [P]
- **Phase 8 US6**：T093 实现完成后，T095（集成日志到既有服务）需扫描所有
  service 文件，避免与同时进行的 service 修改冲突
- **Phase 9 US7**：T099/T100/T103/T104/T105 多数 [P]
- **Phase 10 Polish**：T107/T108/T111/T113/T114/T115/T116 全部 [P]；T117 涉及
  main.rs setup 改动，与 T108 元数据 / T110 release workflow 不冲突但建议
  T117 优先于跨平台手工验收 T112 完成（以便验收期间体验到启动回扫行为）

---

## Parallel Example: US1 启动期

```bash
# 开发者 A：后端 Provider 抽象与 GitHub
Task: "T025 [US1] 定义 GitHostingProvider trait 于 src-tauri/src/services/provider.rs"
Task: "T026 [US1] 实现 GitHub Provider src-tauri/src/services/github_service.rs"

# 开发者 B：后端 GitLab + 自建实例
Task: "T027 [US1] 实现 GitLab Provider src-tauri/src/services/gitlab_service.rs"
Task: "T029 [US1] 实现 GitLab API 地址推导工具"

# 开发者 C：后端 Gitee + 凭据测试
Task: "T028 [US1] 实现 Gitee Provider src-tauri/src/services/gitee_service.rs"
Task: "T040 [US1] [T] 编写凭据服务单元测试"

# 开发者 D：前端 UI
Task: "T034 [US1] 创建前端账号 API 封装 src/api/account.api.ts"
Task: "T035 [US1] 实现 Pinia store src/stores/account.ts"
Task: "T036 [US1] 实现账号管理页面 src/pages/Accounts.vue"
Task: "T037 [US1] 实现账号表单对话框 src/components/account/AccountFormDialog.vue"
```

T030/T031（account_service 与 GitLab 实例配置持久化）需在 T025-T028 至少一个
Provider 完成后启动；T032/T033（Tauri commands）需在 T030 完成后启动。

---

## Implementation Strategy

### MVP First (US1 → US2 → US3 顺序)

1. 完成 Phase 1 (Setup) + Phase 2 (Foundational) — 第 1 周
2. 完成 Phase 3 (US1) — 第 2 周；账号管理可独立 demo
3. 完成 Phase 4 (US2) — 第 3 周；可浏览远程仓库
4. 完成 Phase 5 (US3) — 第 4-5 周；**首次具备完整可发布的差异化能力**
5. 在此节点冻结向 alpha 测试用户开放：US1+US2+US3 = "多平台账号 + 仓库浏览 +
   批量 Clone" 已构成有价值的产品

### Incremental Delivery (P2 → P3)

6. 完成 Phase 6 (US4) — 第 6 周；本地仓库管理
7. 完成 Phase 7 (US5) — 第 7-8 周；单仓库 Git 工作流；**完整 MVP 闭环**
8. 完成 Phase 8 (US6) — 第 9 周；日志与诊断（横切集成需贯穿前面 service）
9. 完成 Phase 9 (US7) — 第 10 周；设置中心
10. 完成 Phase 10 (Polish) — 第 11-12 周；首页 / 打包 / 验收 / 文档

### Parallel Team Strategy

如有 4 名工程师：

- **Eng A（后端 / Provider）**：US1 Provider → US2 同步 → US3 队列 → US4 git_reader
  → US5 git_cli 写操作 → US6 log_service
- **Eng B（后端 / Git CLI）**：Foundational T015-T019 → US3 Clone 命令与进度解析
  → US5 Git 写操作 → US7 detect_git
- **Eng C（前端 / 业务）**：Foundational T020-T022 → US1 账号 UI → US2 远程仓库 UI
  → US3 Clone 中心 UI → US6 日志 UI
- **Eng D（前端 / 工作流）**：US4 本地仓库 UI → US5 仓库工作区 → US7 设置 → US10
  首页 / Polish

---

## Notes

- **任务 ID 全局唯一**：T001 至 T117，共 **117 个任务**
- **[P] 标记**：可与同 Phase 其他 [P] 任务并行
- **[T] 标记**：测试任务，可在被测代码完成后立即编写；不强制 TDD
- **[USx] 标记**：所属 User Story，便于追溯
- **每个任务包含目标 / 实现要点 / 验收标准三段**，保证不依赖额外上下文即可执行
- **宪法合规**：每个 service / command / 组件实现 MUST 满足 Principle II（中文
  注释 ≥ 50%）；CI（T011）强制检查
- **删除操作**：所有涉及删除（账号、本地仓库记录、Clone 半成品目录、旧日志、
  凭据、discard changes）MUST 通过 `ConfirmDangerDialog`（T116）二次确认
  （Principle III）
- **方案偏离**：实施过程中发现偏离原计划（新依赖、schema 变更、跨模块接口变化）
  MUST 暂停并请求用户确认（Principle IV）
- **每个 Phase 结尾验收**：完成 Checkpoint 后产出可工作版本并提交 Git tag，
  失败时可回退
