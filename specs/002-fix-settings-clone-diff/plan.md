# Implementation Plan: 修复设置生效、克隆选分支与变更列表展示

**Branch**: `002-fix-settings-clone-diff` | **Date**: 2026-07-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-fix-settings-clone-diff/spec.md`

## Summary

本特性修复三个独立问题，共享同一套技术栈（Tauri + Rust 后端 / Vue3 前端 + SQLite）：

1. **设置生效（P1，正确性）**：让整个 Git 设置组（Pull/Push 策略、Git 可执行路径、提交身份）在 Git 操作执行阶段被真实读取与应用。核心改造是把「读设置 → 传参 → 拼命令」贯通 `commands/git.rs → services/git_cli_service.rs`，让 `make_git_cli()`、`pull()`、`push()`、`commit()` 不再硬编码。
2. **变更列表展示（P1，可用性）**：改造 `GitFileChanges.vue`，为「已暂存/未暂存」分组增加可折叠标题与分组内部滚动，纯前端 CSS + 少量状态改动。
3. **克隆选分支（P2，能力增强）**：新增「远程仓库分支列表」provider 能力与 IPC 命令；批量克隆对话框为每个仓库增加惰性拉取的分支选择控件；`branch` 字段贯通 payload → DB(clone_tasks) → `git clone -b`。

三块可独立实现、独立验收；建议实现顺序 P1a(设置) → P1b(变更列表) → P2(克隆分支)。

## Technical Context

**Language/Version**: Rust 1.75+（后端 / Tauri 2）、TypeScript 5 + Vue 3.4（前端）

**Primary Dependencies**: Tauri、rusqlite（SQLite）、tokio、async-trait、reqwest（provider HTTP）、Element Plus（前端 UI）、Pinia（状态）

**Storage**: SQLite（`settings` k/v 表、`clone_tasks` 表）；凭据走 OS keyring（本特性不涉及凭据变更）

**Testing**: `cargo test`（Rust 单元/集成，`git_cli_service` 已有解析类测试）、前端手动 GUI 实测（沿用项目既有流程，见 [[feedback_frontend_workflow]]）

**Target Platform**: macOS / Windows / Ubuntu（三平台均需可用，宪法工程约束）

**Project Type**: 桌面应用（Tauri，单仓库 `src/` 前端 + `src-tauri/` 后端）

**Performance Goals**: 变更列表数百文件滚动流畅（无可感知卡顿）；分支列表惰性拉取，打开对话框不因批量拉取卡顿

**Constraints**: 保留既有中文友好错误提示与敏感信息脱敏；设置保存后无需重启即生效；不引入虚拟滚动（问题③）；不新增第三方依赖

**Scale/Scope**: 改动约 3 个后端文件 + 1 条 DB 迁移 + 4~6 个前端文件；单文件保持 ≤500 行、函数 ≤50 行（宪法 Gate I）

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Gate I — 代码质量优先

- [x] 格式化工具：Rust `cargo fmt`；前端 `prettier`。
- [x] 静态分析：Rust `cargo clippy -- -D warnings`；前端 `eslint --max-warnings 0`。
- [x] 长度：改造均为在既有函数内增删或新增小函数（如 `pull_with_strategy`、`list_branches`），预期不突破阈值；`GitFileChanges.vue` 当前 315 行，新增折叠状态后仍 <500 行。
- [x] 无临时调试输出 / 废弃注释 / 无关联 TODO。

### Gate II — 中文注释规范

- [x] 承诺所有新增/修改源文件中文注释比例 ≥ 0.3（见 [[feedback_comment_ratio_gate]]，实际门禁阈值 0.3；不足则补注释而非豁免）。
- [x] 新增公共函数（provider `list_branches`、新 IPC 命令、迁移说明）均加中文文档注释；策略映射、优先级规则等非显然逻辑加 WHY 注释。
- [x] 无自动生成代码，无豁免需求。

### Gate III — 文件操作安全 — **NON-NEGOTIABLE**

- [x] 识别删除点：本特性**不新增**破坏性文件操作。问题①的 discard 二次确认既有逻辑保持不变；克隆失败清理半成品目录（`cleanup_partial_clone`）为既有行为，不改。
- [x] 无新增删除点需确认流程。
- [x] 读/写操作可直接执行（新增 Rust/Vue 源码、追加迁移 SQL）。

### Gate IV — 方案确认优先 — **NON-NEGOTIABLE**

- [x] 本 plan.md 即方案，将呈现给用户等待显式批准后再进入实施（`/speckit-tasks` + 编码）。
- [x] 方案含目标、关键步骤、文件清单、风险、回退（见下文各节）。
- [x] 实施中如需偏离（如发现某平台分支 API 分页/权限特殊）将暂停并二次确认。

### Gate 综合检查

- [x] CI 已有 fmt/clippy/eslint/注释比例门禁，本特性沿用。
- [x] 已关联 `specs/002-fix-settings-clone-diff/plan.md`。
- [x] 跨平台：`git clone -b`、CSS、SQLite 迁移均平台无关；无平台专有代码。

**结论**：无违规，Complexity Tracking 留空。

## Project Structure

### Documentation (this feature)

```text
specs/002-fix-settings-clone-diff/
├── plan.md              # 本文件
├── research.md          # Phase 0：技术决策
├── data-model.md        # Phase 1：数据结构变更
├── quickstart.md        # Phase 1：验收/自测指引
├── contracts/           # Phase 1：IPC 命令契约
│   ├── settings-consumption.md
│   ├── clone-branch.md
│   └── file-changes-ui.md
└── checklists/
    └── requirements.md  # 已由 /speckit-specify 生成
```

### Source Code (repository root)

改动落点（现有目录，不新增顶层结构）：

```text
src-tauri/src/
├── commands/
│   ├── git.rs                 # ① make_git_cli 读设置；pull/push/commit 传策略与身份
│   └── remote_repositories.rs # ② 新增 list_remote_branches 命令
├── services/
│   ├── git_cli_service.rs     # ① pull/push/commit 支持策略与身份注入；clone 支持 -b
│   ├── clone_task_service.rs  # ② payload/建任务/执行透传 branch
│   ├── settings_service.rs    # ①（复用现有 get_git，无需改）
│   ├── provider.rs            # ② trait 新增 list_branches（默认实现）
│   ├── github_service.rs      # ② 实现 list_branches
│   ├── gitlab_service.rs      # ② 实现 list_branches
│   └── gitee_service.rs       # ② 实现 list_branches
├── models/
│   └── clone_task.rs          # ② CloneTask 新增 branch 字段
├── db/
│   ├── migrations.rs          # ② 注册迁移 005
│   └── migrations/005_add_clone_task_branch.sql  # ② 新增列
└── lib.rs                     # ② 注册新命令到 invoke_handler

src/
├── components/
│   ├── git/GitFileChanges.vue # ③ 折叠分组 + 内部滚动
│   └── clone/BatchCloneDialog.vue # ② 每仓库分支选择控件
├── api/
│   ├── remoteRepository.api.ts # ② listRemoteBranches
│   └── cloneTask.api.ts        # ② payload 增 branch
└── types/
    ├── cloneTask.ts            # ② CloneTask/payload 增 branch
    └── repository.ts           # ②（分支列表类型，若需）
```

**Structure Decision**: 沿用既有 Tauri 单仓库分层（commands → services → models/db；前端 pages/components → api → types）。三块改动分别聚焦不同文件，交叉极小。

## Phase 0 摘要（详见 research.md）

- **①设置生效**：在 `commands/git.rs` 各命令内先 `settings_service::get_git(&state.db)` 读设置，用 `git_executable_path` 构造 `GitCliService`，把 pull/push 策略与提交身份透传到 service 层，service 层据策略拼 `--ff-only/--rebase/(merge)`、`git push`、`git -c user.name=.. -c user.email=.. commit`。
- **提交身份优先级**：仓库级已配置则不覆盖，仅当缺失时用设置身份补（通过 `-c` 注入而非写全局 config，避免污染用户全局环境，且随用随取）。
- **②分支列表来源**：三平台均有分支 API（GitHub `/repos/{o}/{r}/branches`、GitLab `/projects/{id}/repository/branches`、Gitee `/repos/{o}/{r}/branches`），分页拉取，复用各 service 既有分页 helper。
- **③展示方式**：纯 CSS——两段包一个 `flex:1; min-height:0; overflow:auto` 滚动容器 + 各分组标题加折叠 toggle（本地 ref 状态），不引入虚拟滚动。

## Phase 1 摘要（详见 data-model.md / contracts / quickstart.md）

- **数据结构**：`CloneTask` + `CreateCloneTasksPayload` + `clone_tasks` 表新增 `branch: Option<String>`；`RemoteBranch { name, is_default }` 新增（provider 返回）。设置模型无需改（字段已在）。
- **IPC 契约**：新增 `list_remote_branches(repo_id) -> Vec<RemoteBranch>`；`create_clone_tasks` payload 增 `branchOverrides: HashMap<repoId, branch>`（缺省=默认分支）。`git_pull/git_push/git_commit` 签名不变（策略从 DB 读取，前端无感）。

## 实施要点与风险

| 关注点 | 要点 | 回退方案 |
|---|---|---|
| ① pull 策略 | `--rebase` 冲突/`merge` 需保留中文冲突提示；`FfOnly` 保持现状 | 保留原 `pull()` 逻辑，仅在读到非默认策略时走新分支 |
| ① 提交身份 | 用 `-c` 注入，避免改用户全局 git config；仓库级已有则不注入 | 若注入引发异常，回退为「仅当缺失时」提示用户 |
| ① git 路径 | `make_git_cli` 需 `State` 才能读 DB；改为 `make_git_cli(&state)` | 读设置失败回退 `"git"`（PATH），不阻断操作 |
| ② 分支拉取 | 惰性、单仓库触发、会话内缓存；失败隔离 | 拉取失败回退「克隆默认分支」，不阻断 |
| ② DB 迁移 | 迁移 005 加 `branch` 列（可空），旧任务兼容 | 列可空，缺省 NULL = 默认分支 |
| ③ 折叠滚动 | 不破坏过滤/stage/discard 既有交互 | 纯样式与本地 ref，风险低 |

## Complexity Tracking

> 无宪法违规，无需填写。
