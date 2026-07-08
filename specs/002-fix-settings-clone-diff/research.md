# Phase 0 Research: 修复设置生效、克隆选分支与变更列表展示

本文件记录三块修复的关键技术决策，解决 plan.md Technical Context 中的未知项。

---

## R1. 设置项如何在 Git 操作执行阶段生效（问题①）

### 现状（根因）

- `commands/git.rs:52` `make_git_cli()` 恒 `GitCliService::with_path(PathBuf::from("git"))`，忽略 `git_executable_path` 设置。
- `services/git_cli_service.rs:396` `pull()` 硬编码 `git pull --ff-only`。
- `git_cli_service.rs:424` `push()` 裸 `git push`（依赖用户全局 `push.default`，非应用设置）。
- `commands/settings.rs:31` `update_settings` 仅落库，不产生任何 git config 副作用；`pre_commit_check` 只检查 `git config user.name` 是否存在——设置里的身份从未写入 git，故设置身份不满足校验。

### 决策

**在 command 层读设置、透传到 service 层按策略拼命令**，不在 `update_settings` 时写全局 git config。

- **Git 路径**：`make_git_cli` 改为接收 `&AppState`（或路径参数），内部 `settings_service::get_git(&state.db)?.git_executable_path` → 有值用之、无值回退 `"git"`。读设置失败也回退 `"git"`，绝不阻断操作。
- **Pull 策略**：`pull()` 增参 `strategy: PullStrategy`。`FfOnly` → `["pull","--ff-only"]`（现状）；`Rebase` → `["pull","--rebase"]`；`Merge` → `["pull","--no-rebase"]`。冲突/分叉的中文提示逻辑保留（rebase 冲突同样命中 `conflict` 分支）。
- **Push 策略**：`push()` 增参 `strategy: PushStrategy`，用 `["-c","push.default=<simple|current|upstream>","push"]` 注入，避免依赖用户全局配置。
- **提交身份**：`commit()` 在检测到仓库缺失 `user.name/email` 且设置中有值时，用 `["-c","user.name=..","-c","user.email=..","commit",...]` 注入；`pre_commit_check` 相应放宽：仓库已配置 **或** 设置中已配置即视为通过。

### 提交身份优先级（关键决策）

**仓库级已有配置优先；设置身份仅在仓库未配置时填补。**

- 理由：用户可能为某仓库刻意设置了不同身份（如工作/开源分身），全局设置不应覆盖；用 `-c` 临时注入而非 `git config --global`，避免污染用户全局环境，且「随用随取」保证设置改动立即生效（FR-006）。
- 与 spec FR-005 一致。

### 备选与否决

- **否决 A：`update_settings` 时写 `git config --global`**。会污染用户全局 git 环境、与「仓库级优先」冲突、且删除设置需反向清理，副作用面大。
- **否决 B：只修 pull/push 策略两项**。用户举例虽是 pull 策略，但根因是「整组只存不用」，半修复会遗留 git 路径/身份同类 bug（用户选择「整个 Git 设置组」）。

---

## R2. 远程仓库分支列表来源与克隆选分支（问题②）

### 现状（根因）

- `git_cli_service.rs:182` clone 命令无 `--branch`；`clone_repository` 无 branch 参数。
- `models/clone_task.rs` `CloneTask`、`clone_task_service.rs:147` `CreateCloneTasksPayload` 均无 branch。
- `provider.rs` `GitHostingProvider` trait 无「列分支」方法；`RemoteRepository` 有 `default_branch`。

### 决策

**新增 provider `list_branches` 能力 + IPC 命令 + branch 字段贯通全链路。**

- **Provider trait**：新增
  ```rust
  async fn list_branches(&self, repo: &RemoteRepository) -> Result<Vec<RemoteBranch>>
  ```
  默认实现返回 `Err(Internal("该平台暂不支持分支列表"))`；三平台各自实现。
- **平台 API**（分页拉全量，复用各 service 既有分页/请求 helper）：
  - GitHub：`GET /repos/{owner}/{repo}/branches?per_page=100&page=N`
  - GitLab：`GET /projects/{id}/repository/branches?per_page=100&page=N`（id 用 url-encoded path 或数值 id）
  - Gitee：`GET /repos/{owner}/{repo}/branches`（Gitee 分支接口一次性返回，无需分页）
  - 默认分支通过 `repo.default_branch` 标记 `is_default`。
- **IPC 命令**：`remote_repositories.rs` 新增 `list_remote_branches(repo_id) -> Vec<RemoteBranch>`，模式完全对齐既有 `list_remote_commits`（取 repo → `provider_for_account` → 调 provider）。
- **克隆链路透传 branch**：
  - `CreateCloneTasksPayload` 增 `branch_overrides: HashMap<String, String>`（key=remoteRepositoryId，缺省=默认分支），与既有 `dir_name_overrides` 同构。
  - `create_clone_tasks` 写入每任务 `branch`（None=默认分支）。
  - `CloneTask` + `clone_tasks` 表增 `branch: Option<String>`（迁移 005）。
  - `start_clone_task` 执行时把 `task.branch` 传入 `clone_repository`，后者在 `Some(b)` 时追加 `["--branch", b]`。

### 惰性拉取与失败隔离（FR-013/FR-014）

- 前端仅在用户**展开某仓库分支控件**时才调 `list_remote_branches`；结果在对话框会话内按 repoId 缓存。
- 单仓库拉取失败 → 该控件显示中文错误 + 「克隆默认分支」回退，不影响其他仓库。

### 备选与否决

- **否决 A：打开对话框即并发拉取所有选中仓库分支**。批量几十仓库会明显卡顿且浪费 API 配额（FR-013）。
- **否决 B：全局共用一个分支名（文本输入）**。无法应对不同仓库分支不同（用户选择「每仓库·API 拉列表」）。
- **否决 C：`git ls-remote` 拉分支**。需凭据注入且慢，平台 REST API 更快且已有 provider 基建。

---

## R3. 变更列表完整展示与折叠（问题③）

### 现状（根因）

- `GitFileChanges.vue` `.git-file-changes` 为 `flex column; height:100%`，内含 toolbar + 两个 `.section`；`.section`/`.file-list` **无 `overflow`、无 `flex`**，随内容无限撑高。
- 父列 `RepositoryDetail.vue:590` `.col { overflow:hidden }` → 溢出被裁剪、无滚动条 → 文件多时看不全。
- 分组标题 `<h4 class="section-title">` 为纯文本，不可折叠（用户所称「下拉框」= 折叠 toggle）。

### 决策

**纯前端：折叠分组头 + 分组内部滚动，不引入虚拟滚动。**

- 结构：toolbar 固定在顶部；下方新增一个 `flex:1; min-height:0; overflow:auto` 的滚动容器承载两个 `.section`；或让每个 `.file-list` 各自 `overflow:auto` 并分配弹性高度。采用「整体滚动容器 + 折叠」最简且符合直觉。
- 折叠：`section-title` 改为可点击（加箭头图标 + `role/aria`），本地 `ref` 记录 `stagedCollapsed`/`unstagedCollapsed`；折叠时隐藏 `.file-list`，标题保留计数。
- 不引入虚拟滚动：用户选择「折叠+内部滚动」；数百文件原生滚动足够流畅（FR-024）。若未来出现上千文件卡顿，再评估复用列表虚拟滚动（远程/本地仓库列表方案）。

### 备选与否决

- **否决 A：直接引入虚拟滚动**。改动更大、与折叠交互叠加复杂度高，当前规模无必要（用户明确选 A）。
- **否决 B：给父 `.col` 开 `overflow:auto`**。会让整列（含 toolbar/分支器等）一起滚动，破坏三栏布局意图；应在组件内部消化滚动。

---

## 依赖与跨平台确认

- 无新增第三方依赖（provider HTTP 复用 reqwest；UI 复用 Element Plus 折叠/滚动能力或原生 CSS）。
- `git clone --branch`、SQLite 迁移、CSS 均平台无关，满足三平台约束。
