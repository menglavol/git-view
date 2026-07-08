---
description: "Task list for feature 002-fix-settings-clone-diff"
---

# Tasks: 修复设置生效 / 克隆选分支 / 变更列表展示

**Input**: Design documents from `/specs/002-fix-settings-clone-diff/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tauri-commands.md, quickstart.md

**Tests**: 未在 spec 中显式要求 TDD；沿用现有 `#[cfg(test)]` 单元测试惯例，仅对新增纯逻辑函数补充单测（不铺开集成测试）。

**Organization**: 任务按用户故事分组，三条故事可独立实现、独立验收。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 可并行（不同文件、无未完成依赖）
- **[Story]**: US1 / US2 / US3，映射 spec.md 用户故事
- 每个任务标注确切文件路径

## Path Conventions

- 后端（Rust / Tauri）：`src-tauri/src/`
- 前端（Vue 3 / TS）：`src/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 本特性无需新建项目脚手架，仅确认工具链就绪。

- [x] T001 确认后端格式化/静态分析可运行：在 `src-tauri/` 下 `cargo fmt --check` 与 `cargo clippy -- -D warnings` 基线通过（记录当前告警数，作为改动后对照）
- [x] T002 [P] 确认前端格式化/静态分析可运行：仓库根 `pnpm prettier --check .` 与 `pnpm eslint . --max-warnings 0` 基线通过

**Checkpoint**: 工具链就绪，可开始实现。

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 三条故事共享的数据与读取能力。US1 依赖「按 repo_id 读取设置组」的辅助，US3 依赖 `branch` 字段落库。此阶段完成前不开始故事实现。

**⚠️ CRITICAL**: 本阶段阻塞所有用户故事。

- [x] T003 [P] 新增 DB 迁移 `src-tauri/src/db/migrations/005_add_clone_task_branch.sql`：为 `clone_tasks` 表增加可空列 `branch TEXT`（旧任务为 NULL，语义=克隆默认分支）
- [x] T004 在 `src-tauri/src/db/migrations.rs` 的 `MIGRATIONS` 数组末尾追加 version 5 条目（`005_add_clone_task_branch`，`include_str!` 引入 T003 的 SQL），不修改已发布历史条目
- [x] T005 在 `src-tauri/src/commands/git.rs` 增加辅助 `fn make_git_cli_from_settings(state: &AppState) -> GitCliService`：读取 `settings_service::get_git` 的 `git_executable_path`，有值则 `GitCliService::with_path(path)`，否则回退现有 `PathBuf::from("git")`；保留旧 `make_git_cli()` 供只读命令使用或一并替换（在函数注释中说明为何区分）

**Checkpoint**: 迁移与设置读取入口就绪，三条故事可并行推进。

---

## Phase 3: User Story 1 - Git 设置真实生效 (Priority: P1) 🎯 MVP

**Goal**: 用户在「设置 → Git」保存的 Git 可执行路径、user.name/email、默认 pull/push 策略在实际 Git 操作中生效。

**Independent Test**: 按 `quickstart.md` §1 —— 将 pull 策略改为 rebase 后触发 pull，操作日志命令串体现 `--rebase`；配置一个自定义 git 路径后 fetch/pull/push 使用该路径；在无仓库级身份的仓库配置 user.name/email 后 commit 校验通过且提交作者正确。

### 后端 · git_cli_service 参数化

- [x] T006 [US1] 在 `src-tauri/src/services/git_cli_service.rs` 将 `pull(&self, repo)` 改为 `pull(&self, repo, strategy: PullStrategy)`：`FfOnly → ["pull","--ff-only"]`、`Rebase → ["pull","--rebase"]`、`Merge → ["pull","--no-ff"]`（或 `--no-edit` 视注释说明）；保留既有分叉/冲突中文错误分支
- [x] T007 [US1] 在同文件将 `push(&self, repo)` 改为 `push(&self, repo, strategy: PushStrategy)`：通过 `-c push.default=<simple|current|upstream>` 注入策略后再 `push`；保留 non-fast-forward / no-upstream / 权限三类中文错误映射
- [x] T008 [US1] 在同文件新增 `ensure_commit_identity(&self, repo, name: Option<&str>, email: Option<&str>)`：当仓库级缺失且设置提供了身份时，通过 `git config user.name/user.email`（仓库级）写入；`pre_commit_check` 前调用，修正"提示配置身份却不写入"的断链（WHY 注释说明优先级：仓库级已有则不覆盖）

### 后端 · 命令层注入设置

- [x] T009 [US1] 在 `src-tauri/src/commands/git.rs` 的 `git_pull` 中读取 `settings_service::get_git().default_pull_strategy` 并传入 `pull(...)`；用 `make_git_cli_from_settings` 构造 CLI；日志命令串按实际策略动态生成（如 `git pull --rebase`）
- [x] T010 [US1] 在同文件 `git_push` 中读取 `default_push_strategy` 传入 `push(...)`；用 `make_git_cli_from_settings` 构造 CLI；日志命令串反映 `push.default`
- [x] T011 [US1] 在同文件 `git_fetch`、`git_checkout_branch`、`git_stage_*`、`git_commit` 等命令统一改用 `make_git_cli_from_settings`，使自定义 Git 路径全面生效（逐个替换 `make_git_cli()` 调用点）
- [x] T012 [US1] 在 `git_commit` 命令中，`pre_commit_check` 之前调用 `ensure_commit_identity`，传入 `settings_service::get_git()` 的 `user_name`/`user_email`
- [x] T013 [P] [US1] 在 `git_cli_service.rs` 的 `#[cfg(test)]` 中补充单测：验证 `PullStrategy`/`PushStrategy` → git 参数数组的映射函数（纯函数，不实际调用 git）

**Checkpoint**: US1 独立可用 —— Git 设置组四类字段均在操作中生效。

---

## Phase 4: User Story 2 - 变更列表完整展示且可折叠 (Priority: P1)

**Goal**: 单仓库工作区「已暂存/未暂存」分段在文件多时不再被裁剪，各段内部独立滚动；分组标题可点击折叠/展开。

**Independent Test**: 按 `quickstart.md` §3 —— 造 50+ 变更文件，打开工作区能滚动查看全部文件；点击「已暂存/未暂存」标题可折叠该段，折叠态在本次会话内切换文件后保持。

- [x] T014 [US2] 在 `src/components/git/GitFileChanges.vue` 增加折叠状态：`ref` 记录 staged/unstaged 两段的展开态（默认展开），标题 `<h4>` 改为可点击折叠头（加 aria 属性、指针光标、展开/折叠图标）
- [x] T015 [US2] 在同文件调整模板：折叠时隐藏对应 `<ul class="file-list">`，展开时显示；保留计数徽标 `({{ staged.length }})` 常驻可见
- [x] T016 [US2] 在同文件 `<style scoped>` 修复溢出：`.git-file-changes` 保持 flex 纵向；为两个 `.section` 设 `flex: 1; min-height: 0; display:flex; flex-direction:column`，`.file-list` 设 `overflow-y: auto; flex: 1`，使每段内部独立滚动而非撑破父容器（WHY 注释：父列 `.col` 为 `overflow:hidden`）
- [x] T017 [P] [US2] 复核 `src/pages/RepositoryDetail.vue` 左栏容器：确认 `.col` 的 `min-height:0` 链路完整传导到 `GitFileChanges`，必要时为其挂载容器补 `flex:1; min-height:0`（仅在 T016 验证仍溢出时改动）

**Checkpoint**: US2 独立可用 —— 变更列表完整可见、可折叠、内部滚动。

---

## Phase 5: User Story 3 - 克隆时选择分支 (Priority: P2)

**Goal**: 批量克隆对话框中每个仓库可选择要克隆的分支（默认选默认分支），克隆使用 `git clone --branch`。

**Independent Test**: 按 `quickstart.md` §2 —— 选中若干仓库打开批量克隆，展开某仓库可见其分支下拉（从平台 API 拉取、可搜索、默认选中默认分支），克隆完成后该仓库本地 HEAD 指向所选分支。

### 后端 · Provider 分支列表 API

- [x] T018 [US3] 在 `src-tauri/src/services/provider.rs` 的 `GitHostingProvider` trait 增加 `async fn list_branches(&self, repo: &RemoteRepository) -> Result<Vec<String>>`，默认实现返回 `Err(Internal("该平台暂不支持分支列表"))`（或返回仅含 `default_branch` 的单元素回退，按 research.md 决策注释说明）
- [x] T019 [P] [US3] 在 `src-tauri/src/services/github_service.rs` 实现 `list_branches`：调用 `GET /repos/{owner}/{repo}/branches`（分页聚合），映射为分支名列表，复用现有错误脱敏
- [x] T020 [P] [US3] 在 `src-tauri/src/services/gitlab_service.rs` 实现 `list_branches`：调用 `GET /projects/{id}/repository/branches`，映射分支名列表
- [x] T021 [P] [US3] 在 `src-tauri/src/services/gitee_service.rs` 实现 `list_branches`：调用 `GET /repos/{owner}/{repo}/branches`，映射分支名列表
- [x] T022 [US3] 在 `src-tauri/src/commands/remote_repositories.rs` 新增命令 `list_remote_branches(state, repo_id) -> Result<Vec<String>>`：取仓库 + `provider_for_account`，调用 `provider.list_branches`；在 `src-tauri/src/lib.rs` 注册该命令到 `invoke_handler`

### 后端 · 克隆携带分支

- [x] T023 [US3] 在 `src-tauri/src/models/clone_task.rs` 的 `CloneTask` 增加 `#[serde(skip_serializing_if="Option::is_none")] pub branch: Option<String>` 字段（含中文注释）
- [x] T024 [US3] 在 `src-tauri/src/services/clone_task_service.rs` 的 `CreateCloneTasksPayload` 增加 `#[serde(default)] pub branches: HashMap<String, String>`（key=remoteRepositoryId）；`create_clone_tasks` 写入 `clone_tasks.branch` 列并回填 `CloneTask.branch`；调整 INSERT 语句与行映射 SELECT
- [x] T025 [US3] 在 `src-tauri/src/services/git_cli_service.rs` 的 `clone_repository` 增加 `branch: Option<&str>` 参数（或在调用处组装 args）：`Some(b)` 时追加 `--branch <b>`；更新调用点签名与 `#[allow(clippy::too_many_arguments)]` 注释
- [x] T026 [US3] 在 `src-tauri/src/services/clone_task_service.rs` 的 `start_clone_tasks`/`run_one` 执行链把任务的 `branch` 透传给 `clone_repository`；日志命令串体现 `--branch`

### 前端 · 分支选择 UI

- [x] T027 [P] [US3] 在 `src/types/cloneTask.ts` 为 `CloneTask` 增加可选 `branch?: string`；在 `CreateCloneTasksPayload` 增加 `branches?: Record<string,string>`
- [x] T028 [P] [US3] 在 `src/api/remoteRepository.api.ts` 增加 `listBranches(repoId): Promise<string[]>`（invoke `list_remote_branches`）
- [x] T029 [US3] 在 `src/components/clone/BatchCloneDialog.vue` 为每个选中仓库增加分支下拉（`el-select` filterable）：展开/聚焦时惰性调用 `listBranches`，加载态与失败回退（回退到该仓库 `defaultBranch`）；默认选中 `defaultBranch`
- [x] T030 [US3] 在同文件 `confirm` 组装 payload 时带上 `branches`（仅收集与默认分支不同或已显式选择的项，减少无谓传参，按 data-model 语义 NULL=默认分支）

**Checkpoint**: US3 独立可用 —— 可按分支克隆。

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T031 [P] 更新 `docs/user-guide.md`：§6 补充 pull/push 策略与 Git 身份「在设置中配置后即生效」；§4 批量克隆补充「可选择克隆分支」
- [x] T032 [P] 校验中文注释比例：对所有新增/修改的 `.rs` 与 `.vue` 文件确认单文件 `中文注释行/非空代码行 >= 0.3`（宪法 Principle II）
- [x] T033 运行后端质量门禁：`src-tauri/` 下 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` 全绿
- [x] T034 [P] 运行前端质量门禁：`pnpm prettier --check .`、`pnpm eslint . --max-warnings 0`、`pnpm build`（或 `vue-tsc`）通过
- [ ] T035 按 `quickstart.md` 三节手动验收 US1/US2/US3，逐条勾选验收标准

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 无依赖，立即可做
- **Foundational (Phase 2)**: 依赖 Setup；阻塞所有故事（T005 供 US1，T003/T004 供 US3）
- **User Stories (Phase 3/4/5)**: 均依赖 Phase 2 完成
  - US1 与 US2 完全独立（后端 vs 前端），可并行
  - US2 纯前端，甚至可在 Phase 2 完成前独立推进（不依赖 T003–T005）
  - US3 依赖 T003/T004（迁移）与 T005 无关但同属命令层
- **Polish (Phase 6)**: 依赖目标故事完成

### User Story Dependencies

- **US1 (P1)**: 依赖 T005；与 US2/US3 无交叉
- **US2 (P1)**: 仅前端单文件为主，独立
- **US3 (P2)**: 依赖 T003/T004；provider 三平台实现相互独立

### Within Each User Story

- US1：service 参数化（T006–T008）先于命令层注入（T009–T012）
- US3：trait 定义（T018）先于三平台实现（T019–T021）；模型/payload（T023–T024）先于克隆执行透传（T025–T026）；后端命令（T022）先于前端 api/UI（T028–T030）

### Parallel Opportunities

- T001 / T002 并行
- T019 / T020 / T021 三平台实现并行
- T027 / T028 前端类型与 api 并行
- US1（后端）与 US2（前端）跨故事并行
- Polish 中 T031 / T032 / T034 并行

---

## Parallel Example: User Story 3 Provider 实现

```text
# T018 完成 trait 后，三平台实现并行：
Task: "github_service.rs 实现 list_branches"
Task: "gitlab_service.rs 实现 list_branches"
Task: "gitee_service.rs 实现 list_branches"
```

---

## Implementation Strategy

### MVP First

1. Phase 1 Setup → Phase 2 Foundational
2. Phase 3 US1（Git 设置生效）→ 按 quickstart §1 验收 → 可交付
3. 并行推进 Phase 4 US2（纯前端，低风险高感知）

### Incremental Delivery

1. Setup + Foundational → 基础就绪
2. US1 → 独立验收 → 交付（MVP）
3. US2 → 独立验收 → 交付
4. US3 → 独立验收 → 交付

---

## Notes

- [P] = 不同文件、无未完成依赖
- 每完成一个任务或逻辑组即提交（遵循 git_safety，先切分支——本特性分支 `002-fix-settings-clone-diff` 已就绪）
- DB 迁移（T003/T004）为向后兼容的加列，旧库自动补列、旧任务 branch=NULL=默认分支
- 无文件删除类破坏性操作；如后续需清理，按宪法 Principle III 单独请求确认
