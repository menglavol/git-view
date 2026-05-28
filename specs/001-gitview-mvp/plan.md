# Implementation Plan: GitView V1 MVP — 轻量级跨平台 Git 可视化客户端

**Branch**: `001-gitview-mvp` | **Date**: 2026-05-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-gitview-mvp/spec.md`

**Note**: 本 plan.md 在 `/speckit-tasks` 命令中作为 tasks 生成的前置输入精简生成；
完整研究/数据模型/契约/快速入门文档（research.md / data-model.md / contracts/ /
quickstart.md）目前未拆分，相关信息已浓缩在 §Technical Context、§Project Structure、
§Data Model Overview、§Tauri Command Contracts 章节。后续如需补齐可独立执行
`/speckit-plan`。

## Summary

GitView 是一个轻量级跨平台 Git 可视化客户端，基于 **Tauri 2 + Vue 3 + TypeScript +
Rust** 构建，目标覆盖 macOS / Windows / Ubuntu。V1 MVP 聚焦 4 大核心体验：
**多账号添加稳定、远程仓库同步稳定、批量 Clone 稳定、基础 Git 操作稳定**。技术上
采用"前端 Vue 渲染 + 后端 Rust 业务 + 系统 Git CLI 执行实际 Git 操作"的分层架构；
凭据存储统一交由操作系统原生密钥库；本地元数据持久化于 SQLite。

## Technical Context

**Language/Version**:
- 前端：TypeScript 5.x，Vue 3.4+，Vite 5.x
- 后端：Rust 1.75+ (edition 2021)

**Primary Dependencies**:
- 桌面容器：Tauri 2.x（含 `tauri`、`tauri-plugin-fs`、`tauri-plugin-dialog`、
  `tauri-plugin-shell`、`tauri-plugin-os` 等核心插件）
- UI 组件：Element Plus 2.x
- 状态管理：Pinia 2.x
- 路由：Vue Router 4.x
- 异步状态（可选）：`@tanstack/vue-query`
- HTTP 客户端（Rust）：`reqwest` 0.11+（含 `rustls` 后端，支持自签名证书白名单）
- 异步运行时：`tokio` 1.x
- 序列化：`serde` 1.x + `serde_json`
- 数据库：`rusqlite` 0.31+（捆绑 SQLite 3.x）配合自实现迁移管理
- 凭据存储：`keyring` 2.x（封装 macOS Keychain / Windows Credential Manager /
  Linux Secret Service）
- 日志：`tracing` + `tracing-subscriber` + `tracing-appender`
- 错误处理：`thiserror` + `anyhow`
- 唯一标识：`uuid` 1.x
- Git 进程执行：系统 Git CLI（通过 `std::process::Command` + `tokio::process`）

**Storage**:
- 元数据：SQLite（位于 OS 用户配置目录，如 macOS `~/Library/Application Support/
  com.gitview.app/gitview.db`）；迁移脚本版本化
- 凭据：操作系统原生密钥库（按平台映射到 `keyring` crate）
- 日志：本地滚动日志文件（位于 OS 用户日志目录，单文件 ≤ 10 MB，保留最近 5 份）

**Testing**:
- 后端单元/集成：`cargo test`（含 `tokio::test` 异步用例）
- 前端单元/组件：`Vitest` + `@vue/test-utils`
- 端到端（手动）：MVP 验收用例（spec.md §SC-018）跨 macOS / Windows / Ubuntu
  人工走查
- 安全自动化扫描：日志/数据库 Token 明文扫描脚本（CI 任务）

**Target Platform**:
- macOS 12+（Apple Silicon 与 Intel）
- Windows 10 / 11
- Ubuntu 20.04+（其他主流 Linux 发行版尽力支持）

**Project Type**: Desktop application（前端 + 后端分布于同一仓库的桌面应用结构）

**Performance Goals**:
- 应用冷启动 ≤ 3 秒（macOS / Windows）
- 远程仓库列表渲染 ≥ 5000 条不卡顿（虚拟滚动）
- 本地仓库列表渲染 ≥ 500 条不卡顿
- 搜索过滤响应时间 ≤ 500 ms（已缓存数据）
- 批量 Clone 默认并发 3，上限 8
- 主线程任何操作不超过 200 ms 无响应

**Constraints**:
- 安装包体积目标 < 30 MB（macOS 与 Windows）
- 内存占用空闲态 < 200 MB
- 数据库文件单实例 < 100 MB（5000 仓库 + 10000 日志规模下）
- Token 明文不得出现在数据库、日志、UI 任何位置
- 删除文件类操作必须经用户显式确认（宪法 Principle III）

**Scale/Scope**:
- 单用户 / 单设备
- 账号数量 ≤ 20
- 远程仓库 ≤ 10000（缓存）
- 本地仓库 ≤ 1000
- 操作日志 ≤ 50000 条（滚动清理）
- V1 MVP 范围：spec.md 的 7 个 User Story（US1-US7），59 条 FR，18 条 SC

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

依据 GitView Constitution v1.0.0 的 4 条核心原则进行检查。

### Gate I — 代码质量优先 (Code Quality First)

- [x] **格式化工具**：Rust 使用 `cargo fmt`；TypeScript/Vue 使用 `prettier 3.x` +
      官方 Vue/TS 插件。
- [x] **静态分析工具**：Rust 使用 `cargo clippy -- -D warnings`；TypeScript/Vue
      使用 `eslint 8.x` + `@typescript-eslint`、`eslint-plugin-vue`，配置
      `--max-warnings 0`。
- [x] **函数/文件长度**：模块化设计，每个 service 单一职责；commands 文件按
      模块拆分（accounts / remote_repos / local_repos / clone_tasks / git /
      settings / logs）。无超阈值文件预期；如出现，必须拆分。
- [x] **无遗留废代码**：CI 加入 `grep` 扫描 `println!`、`console.log`、`TODO`
      无 issue 链接、`dbg!` 等的检查。

### Gate II — 中文注释规范 (Chinese Documentation Standard)

- [x] **注释比例 ≥ 50%**：所有 Rust `///` 文档注释与 Vue/TS `/** */` 文档注释
      使用中文撰写；CI 中加入 `scripts/check-comment-ratio.sh` 按文件粒度核算。
- [x] **覆盖策略**：
  - 文件头：注明用途、依赖、负责模块
  - 公共 API（`pub fn`、`pub struct`、`#[tauri::command]`、TS `export`）：完整
    文档注释（用途/参数/返回值/异常/示例）
  - 复杂逻辑（Clone 进度解析、Token 脱敏、API 地址推导、自签名证书处理）：行内
    解释 WHY
- [x] **豁免清单**：数据库迁移 SQL 文件、`tauri.conf.json` 等配置文件、自动生成
  的类型定义（如 TS 类型从 `serde` schema 生成）豁免。豁免清单维护于
  `.specify/comment-exemptions.yml`（任务中创建）。

### Gate III — 文件操作安全 (File Operation Safety) — **NON-NEGOTIABLE**

- [x] **删除点识别**：本 MVP 涉及以下文件/目录删除点
  1. 删除账号时清除系统凭据（`keyring::delete_password`）
  2. 取消 Clone 任务时清理半成品目录（`std::fs::remove_dir_all` 仅针对已识别为
     Clone 任务目标且未完成的目录）
  3. "从列表移除"本地仓库（**仅数据库行删除**，不删磁盘文件）
  4. discard changes（`git checkout -- <file>` 与 `git clean -fd`）
  5. 滚动清理过旧操作日志（数据库行删除）
- [x] **确认流程**：
  - 删除账号、discard changes 使用 `ConfirmDangerDialog.vue` 二次确认
  - 取消 Clone 任务清理半成品目录视为任务取消的隐含动作（不再二次确认，但日志
    必须记录清理范围）
  - 滚动清理日志属系统自动行为，需用户在设置中开启"自动清理 X 天前日志"并
    确认阈值
- [x] **读写直通**：所有数据库读写、SQLite 元数据写入、本地仓库元数据缓存写入
  无需逐次确认；统一遵循"读写直通、删除必确认"原则。

### Gate IV — 方案确认优先 (Plan-First Approval) — **NON-NEGOTIABLE**

- [x] **本 plan 已经用户确认**：用户在 `/speckit-tasks` 命令的 AskUserQuestion
  中显式确认"本次一并生成 plan.md 与 tasks.md"。
- [x] **五要素齐全**：变更目标（V1 MVP 落地）、关键步骤（10 个 Phase）、涉及
  文件清单（详见 Project Structure）、潜在风险（详见 §Risks）、回退方案
  （单 Phase 完成后即可作为可工作版本，问题时可回退到上一 Phase）。
- [x] **偏离协议**：实施过程中若需引入新依赖、调整数据库 schema、变更跨模块
  接口，MUST 暂停实施并提供变更方案再次请求用户确认。

### Gate 综合检查

- [x] **CI**：项目初始化任务（T011 / T013）将配置 GitHub Actions 工作流，强制
  执行格式化、静态分析、注释比例检查与跨平台构建。
- [x] **关联**：本 plan 关联到 `specs/001-gitview-mvp/plan.md`；tasks 生成将
  关联到同目录 tasks.md。
- [x] **跨平台**：所有路径处理使用 `std::path::PathBuf` 与 `dunce`；进程调用
  避免 shell 解释；CI 在三平台均构建。

## Project Structure

### Documentation (this feature)

```text
specs/001-gitview-mvp/
├── plan.md              # 本文件
├── spec.md              # 已生成
├── checklists/
│   └── requirements.md  # 已生成
└── tasks.md             # 即将生成
```

### Source Code (repository root)

```text
git-view/
├── src-tauri/                          # Rust 后端（Tauri 项目根）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs                     # Tauri 入口，注册命令与状态
│       ├── lib.rs                      # 模块导出聚合
│       ├── commands/                   # Tauri command 层（薄包装层）
│       │   ├── mod.rs
│       │   ├── accounts.rs             # 账号相关 command
│       │   ├── remote_repositories.rs  # 远程仓库 command
│       │   ├── local_repositories.rs   # 本地仓库 command
│       │   ├── clone_tasks.rs          # Clone 任务 command
│       │   ├── git.rs                  # 单仓库 Git 操作 command
│       │   ├── settings.rs             # 设置 command
│       │   └── logs.rs                 # 日志 command
│       ├── services/                   # 业务服务层
│       │   ├── mod.rs
│       │   ├── credential_service.rs   # 凭据存储（keyring 封装）
│       │   ├── account_service.rs      # 账号 CRUD 与连接测试
│       │   ├── github_service.rs       # GitHub Provider
│       │   ├── gitlab_service.rs       # GitLab Provider（含自建实例）
│       │   ├── gitee_service.rs        # Gitee Provider
│       │   ├── provider.rs             # GitHostingProvider trait
│       │   ├── git_cli_service.rs      # Git CLI 命令封装
│       │   ├── git_reader_service.rs   # Git 状态/diff/log 解析
│       │   ├── clone_task_service.rs   # Clone 任务队列与进度解析
│       │   ├── repository_service.rs   # 本地仓库扫描/状态
│       │   ├── settings_service.rs     # 设置读写
│       │   └── log_service.rs          # 操作日志记录与脱敏
│       ├── db/                         # 数据访问层
│       │   ├── mod.rs
│       │   ├── pool.rs                 # 连接池（r2d2-sqlite 或自实现）
│       │   ├── migrations.rs           # 迁移管理
│       │   └── migrations/             # 版本化 SQL 文件目录
│       │       ├── 001_init.sql
│       │       └── 002_gitlab_instance_configs.sql
│       ├── models/                     # 领域模型（与前端 TS 类型对应）
│       │   ├── mod.rs
│       │   ├── account.rs
│       │   ├── repository.rs
│       │   ├── clone_task.rs
│       │   ├── git.rs
│       │   ├── settings.rs
│       │   └── operation_log.rs
│       ├── errors.rs                   # 统一错误类型与错误码
│       └── utils/
│           ├── mod.rs
│           ├── path.rs                 # 路径规范化、目录创建
│           ├── process.rs              # 子进程执行与输出读取
│           ├── redact.rs               # 敏感信息脱敏
│           └── time.rs                 # ISO 8601 时间处理
│
├── src/                                # Vue 前端
│   ├── main.ts
│   ├── App.vue
│   ├── router/
│   │   └── index.ts
│   ├── layouts/
│   │   └── AppLayout.vue
│   ├── pages/
│   │   ├── Dashboard.vue
│   │   ├── Accounts.vue
│   │   ├── RemoteRepositories.vue
│   │   ├── CloneCenter.vue
│   │   ├── LocalRepositories.vue
│   │   ├── RepositoryDetail.vue
│   │   ├── Logs.vue
│   │   └── Settings.vue
│   ├── components/
│   │   ├── common/
│   │   │   ├── PlatformBadge.vue
│   │   │   ├── StatusTag.vue
│   │   │   ├── EmptyState.vue
│   │   │   └── ConfirmDangerDialog.vue
│   │   ├── layout/
│   │   │   ├── Sidebar.vue
│   │   │   └── Topbar.vue
│   │   ├── account/
│   │   │   ├── AccountCard.vue
│   │   │   ├── AccountFormDialog.vue
│   │   │   └── AccountSwitcher.vue
│   │   ├── repository/
│   │   │   ├── RemoteRepoTable.vue
│   │   │   ├── LocalRepoTable.vue
│   │   │   ├── RepoDetailDrawer.vue
│   │   │   └── RepoStatusOverview.vue
│   │   ├── clone/
│   │   │   ├── BatchCloneDialog.vue
│   │   │   ├── CloneTaskTable.vue
│   │   │   └── CloneProgress.vue
│   │   └── git/
│   │       ├── GitFileChanges.vue
│   │       ├── DiffViewer.vue
│   │       ├── CommitPanel.vue
│   │       ├── BranchSelector.vue
│   │       └── CommitHistory.vue
│   ├── stores/                         # Pinia stores
│   │   ├── account.ts
│   │   ├── remoteRepository.ts
│   │   ├── localRepository.ts
│   │   ├── cloneTask.ts
│   │   ├── settings.ts
│   │   └── app.ts
│   ├── api/                            # Tauri 命令前端封装
│   │   ├── tauri.ts                    # invoke 与事件订阅辅助
│   │   ├── account.api.ts
│   │   ├── remoteRepository.api.ts
│   │   ├── localRepository.api.ts
│   │   ├── cloneTask.api.ts
│   │   ├── git.api.ts
│   │   ├── settings.api.ts
│   │   └── logs.api.ts
│   ├── types/                          # 与 Rust models 对应的 TS 类型
│   │   ├── account.ts
│   │   ├── repository.ts
│   │   ├── cloneTask.ts
│   │   ├── git.ts
│   │   ├── settings.ts
│   │   └── operationLog.ts
│   └── utils/
│       ├── format.ts
│       └── debounce.ts
│
├── tests/                              # 跨前后端集成与端到端测试
│   ├── integration/                    # 后端集成测试（Rust）
│   │   ├── account_provider_test.rs
│   │   ├── clone_task_test.rs
│   │   ├── repository_scan_test.rs
│   │   └── credential_service_test.rs
│   └── frontend/                       # 前端单元/组件测试（Vitest）
│       ├── components/
│       └── api/
│
├── scripts/                            # 项目辅助脚本
│   ├── check-comment-ratio.sh          # 中文注释比例验证
│   ├── check-no-debug-prints.sh        # 检测遗留调试输出
│   └── check-no-token-leak.sh          # 检测 Token 明文泄漏
│
├── .github/workflows/
│   └── ci.yml                          # 跨平台构建 + 静态分析 + 注释比例
│
├── package.json
├── tsconfig.json
├── vite.config.ts
├── .eslintrc.cjs
├── .prettierrc.json
├── README.md
└── CLAUDE.md                           # 项目级 AI 指引（已存在）
```

**Structure Decision**: 采用 Tauri 标准的 "src/ + src-tauri/" 双子项目结构，
配合根级 tests/ 用于跨前后端集成测试。该结构与产品设计文档 §17 / §19 描述
保持一致。

## Data Model Overview

> 完整 schema 参见产品设计文档 §16；本节为 task 生成所需的精简概览。

主表（SQLite）：

1. **accounts** — 平台账号元信息
2. **gitlab_instance_configs** — 自建 GitLab 实例配置（一对一关联到 accounts）
3. **remote_repositories** — 远程仓库元数据缓存
4. **local_repositories** — 本地仓库管理记录
5. **clone_tasks** — Clone 任务持久化
6. **operation_logs** — 操作日志
7. **settings** — 偏好键值对
8. **schema_migrations** — 迁移版本追踪表

凭据存储（OS 密钥库）：
- 命名空间：`gitview`，键名 `account-token-<accountId>`，值为 Personal Access Token

## Tauri Command Contracts

> 完整命令列表参见产品设计文档 §18；本节列出 V1 范围内全部 **49 个命令 + 3
> 个事件**。命令按业务域分组；命令注册在 tasks.md 的各 User Story 阶段实现。

**账号域 (7)**: `add_account` / `test_account_connection` / `list_accounts` /
`update_account` / `delete_account` / `set_default_account` /
`sync_account_repositories`

**远程仓库域 (5)**: `list_remote_repositories` / `search_remote_repositories` /
`refresh_remote_repositories` / `get_remote_repository_detail` /
`toggle_favorite_remote_repository`

**本地仓库域 (9)**: `add_local_repository` / `scan_local_repositories` /
`list_local_repositories` / `remove_local_repository` /
`refresh_local_repository_status` / `refresh_all_local_repository_status` /
`open_repository_folder` / `open_repository_in_terminal` /
`batch_fetch_repositories`

**Clone 域 (6)**: `create_clone_tasks` / `list_clone_tasks` /
`start_clone_tasks` / `cancel_clone_task` / `retry_clone_task` /
`clear_finished_clone_tasks`

**Git 操作域 (15)**: `git_status` / `git_diff` / `git_stage_file` /
`git_unstage_file` / `git_stage_all` / `git_unstage_all` / `git_commit` /
`git_fetch` / `git_pull` / `git_push` / `git_list_branches` /
`git_checkout_branch` / `git_create_branch` / `git_log` /
`git_discard_changes`

**设置域 (4)**: `get_settings` / `update_settings` / `detect_git` / `set_git_path`

**日志域 (3)**: `list_operation_logs` / `get_operation_log_detail` /
`clear_old_operation_logs`

**事件 (Tauri Event, 3)**: `clone-task-progress`、`clone-task-status-changed`、
`repository-sync-progress`

> 注：`git_discard_changes` 与 `batch_fetch_repositories` 是宪法 Principle III
> 涉及"破坏性/批量"操作，前端调用前 MUST 通过 `ConfirmDangerDialog` 完成
> 二次确认。`git_delete_branch` 不在 V1 范围（归入 V2）。

## Risks

| 风险 | 影响 | 缓解 |
|------|------|------|
| Tauri 2 API 不稳定 | 升级成本 | 锁定 minor 版本；订阅 release notes |
| Linux Secret Service 不可用 | 凭据无法存储 | 启动时检测 `keyring` 可用性；不可用时提示用户安装 `gnome-keyring` 或 `libsecret`；预留加密本地存储 fallback（V2） |
| Git CLI 输出 i18n 差异 | 解析错误 | 强制 `LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`；使用 porcelain 格式 |
| HTTPS Token 泄漏 | 安全事故 | 临时 askpass + 严格脱敏 + CI 扫描 |
| 大仓库 diff 卡死 | UI 卡顿 | 阈值（1 MB）截断 + 异步执行 |
| 跨平台路径差异 | Bug | `std::path::PathBuf` + `dunce::canonicalize` |
| SQLite 多连接锁 | 性能 | WAL 模式 + 单连接池上限 |
| Clone 子进程僵尸 | 资源泄漏 | 任务取消时 `child.kill()` + 等待 `child.wait()` |
| 应用强退后"幽灵任务" | UI 显示 running 但实际无进程 | 启动期回扫 `clone_tasks` 表，将 `status=running` 行统一标记为 `interrupted/failed`（对应 spec SC-013，由 T117 实现） |
| 内网 GitLab 不可达 | API 调用超时反复重试 | reqwest 客户端设固定超时；DNS 解析失败/连接超时映射明确错误码与中文提示 |
| 多账号同时触发同步 | 触发平台限流 | account_service 在 `sync_account_repositories` 内部维护账号粒度互斥锁；同一账号同步任务排队，跨账号允许并行 |

## Rollback Strategy

- **按 Phase 回退**：每个 Phase 完成且测试通过后形成可独立运行的版本；问题发现
  时可回退到上一 Phase 的 Git tag。
- **数据库迁移回退**：每个迁移文件配套 `*_down.sql`（V1 暂不强制实现，但 schema
  保留 `schema_migrations` 表以支持后续）。
- **凭据回退**：删账号操作单独事务；失败时数据库行不删，避免凭据残留。
- **Clone 任务回退**：取消任务时清理半成品目录，事务化失败时数据库标记为
  `cancelled` 但不删目录（避免误删）。

## Complexity Tracking

> 本 plan 未引入超出宪法允许的复杂度，无需填写。

所有违反检查通过；NON-NEGOTIABLE 原则（III、IV）均符合。
