# Contracts: Tauri 命令与前端 API 契约

本文件定义本特性涉及的 Tauri command 签名（Rust 后端 ↔ 前端 IPC）与前端 API/类型契约。
契约以「现状 → 变更后」对照方式呈现，标注新增（NEW）、修改（MOD）、不变（KEEP）。

命名约定：Rust 端 snake_case 参数经 Tauri 自动转换为前端 camelCase；返回结构体用
`#[serde(rename_all = "camelCase")]` 统一。

---

## 1. 问题① — Git 设置生效

### 1.1 `git_pull`（MOD — 行为变更，签名不变）

```rust
#[tauri::command]
pub async fn git_pull(state: State<'_, AppState>, repo_id: String) -> Result<String>;
```

- **签名不变**：前端调用方式 `gitApi.pull(repoId)` 保持兼容。
- **行为变更**：不再硬编码 `--ff-only`；改为读取 `settings.git.defaultPullStrategy`，
  映射为对应 git 参数：
  - `ff_only` → `git pull --ff-only`（现状默认，保持不变）
  - `rebase` → `git pull --rebase`
  - `merge` → `git pull --no-rebase`
- **日志 command 字段**：由固定字符串 `"git pull --ff-only"` 改为反映实际策略的字符串
  （如 `"git pull --rebase"`），便于操作日志审计。
- **错误映射**：保留现有 non-fast-forward / conflict 中文提示；rebase/merge 冲突复用
  conflict 分支提示。

### 1.2 `git_push`（MOD — 行为变更，签名不变）

```rust
#[tauri::command]
pub async fn git_push(state: State<'_, AppState>, repo_id: String) -> Result<String>;
```

- **行为变更**：读取 `settings.git.defaultPushStrategy`，通过 `git -c push.default=<value> push`
  注入策略（`simple` / `current` / `upstream`）。
- **日志 command 字段**：反映实际策略。
- **错误映射**：保留现有 rejected / no-upstream / permission 三类中文提示。

### 1.3 `make_git_cli`（MOD — 内部函数，非 command）

```rust
// 现状
fn make_git_cli() -> GitCliService;
// 变更后
fn make_git_cli(state: &AppState) -> Result<GitCliService>;
```

- **变更**：读取 `settings.git.gitExecutablePath`；为 `Some(path)` 时用
  `GitCliService::with_path(path)`，否则回退 `PathBuf::from("git")`（现状行为）。
- **影响范围**：所有 `git.rs` 内调用点（stage/unstage/commit/fetch/pull/push/checkout/
  create_branch 等）改为传入 `state` 并处理 `Result`。
- 读设置失败（DB 异常）时回退默认 `"git"`，不阻断 Git 操作。

### 1.4 提交身份注入（MOD — GitCliService::commit 相关）

- **策略**：提交前若 `settings.git.userName` / `userEmail` 有值，且仓库/全局 git config
  未配置对应项，则通过 `git -c user.name=<v> -c user.email=<v> commit` 注入。
- **优先级**：仓库级/全局 git config 已配置时 **不覆盖**（尊重用户既有配置）；设置身份
  仅作为「填补空缺」来源。
- **pre_commit_check 调整**：现状在 config 缺失时直接报错「请在设置中配置 Git 身份」。
  变更后：若设置身份存在，则视为已满足校验（因为提交时会注入），不再误报。

---

## 2. 问题② — 克隆时选择分支

### 2.1 `list_remote_branches`（NEW command）

```rust
#[tauri::command]
pub async fn list_remote_branches(
    state: State<'_, AppState>,
    repo_id: String,
) -> Result<Vec<BranchRef>>;
```

- **用途**：拉取指定远程仓库的分支列表，供克隆对话框「分支选择」使用。
- **实现**：`account_service::provider_for_account` → `provider.list_branches(&repo)`。
- **返回**：`BranchRef { name: String, is_default: bool }`，默认分支置顶或标记。
- **错误**：平台不支持时返回 `Internal`；网络失败复用现有映射。前端捕获后允许用户
  回退到「使用默认分支」。

### 2.2 `GitHostingProvider::list_branches`（NEW trait 方法）

```rust
async fn list_branches(&self, repo: &RemoteRepository) -> Result<Vec<BranchRef>> {
    // 默认实现：仅返回仓库默认分支（保证未实现平台仍可用）
    Ok(vec![BranchRef { name: repo.default_branch.clone(), is_default: true }])
}
```

- GitHub：`GET /repos/{owner}/{repo}/branches`（分页，取前若干页）
- GitLab：`GET /projects/{id}/repository/branches`
- Gitee：`GET /repos/{owner}/{repo}/branches`
- 各实现把响应映射为 `BranchRef`，标记与 `repo.default_branch` 相同者为 `is_default`。

### 2.3 `create_clone_tasks` / `CreateCloneTasksPayload`（MOD）

```rust
pub struct CreateCloneTasksPayload {
    pub remote_repository_ids: Vec<String>,
    pub target_root: String,
    pub directory_strategy: DirectoryStrategy,
    pub concurrency: Option<u8>,
    pub auto_add_to_local: bool,
    #[serde(default)]
    pub dir_name_overrides: HashMap<String, String>,
    // NEW：每仓库选择的分支（key = remoteRepositoryId）。缺省项回退默认分支。
    #[serde(default)]
    pub branch_overrides: HashMap<String, String>,
}
```

- **CloneTask 模型（MOD）**：新增 `pub branch: Option<String>` 字段（`None` = 默认分支）。
- **DB（MOD）**：`clone_tasks` 表新增 `branch TEXT`（迁移 005，可空，旧行为 NULL）。

### 2.4 `clone_repository`（MOD — GitCliService）

```rust
pub async fn clone_repository<F>(
    &self,
    remote_url: &str,
    target_path: &Path,
    branch: Option<&str>,   // NEW
    preserve_target_dir: bool,
    credentials: Option<CredentialInjection>,
    extra_env: &[(String, String)],
    progress: F,
    cancel_token: CancellationToken,
) -> Result<()>;
```

- **行为**：`branch` 为 `Some(b)` 时在命令中追加 `--branch <b>`；`None` 时保持现状
  （克隆远端默认分支）。其余参数与逻辑不变。

---

## 3. 前端契约

### 3.1 类型（`src/types/`）

```typescript
// settings.ts — 无新增字段（PullStrategy/PushStrategy 已存在），仅确保 UI 生效

// cloneTask.ts（MOD）
export interface CloneTask {
  // ...现有字段
  branch?: string; // NEW：克隆分支，缺省表示默认分支
}

// repository.ts 或 remoteRepository（NEW）
export interface BranchRef {
  name: string;
  isDefault: boolean;
}

// cloneTask.api.ts — CreateCloneTasksPayload（MOD）
export interface CreateCloneTasksPayload {
  // ...现有字段
  branchOverrides?: Record<string, string>; // NEW
}
```

### 3.2 API（`src/api/`）

```typescript
// remoteRepository.api.ts（NEW 方法）
listBranches(repoId: string): Promise<BranchRef[]> {
  return invokeCmd<BranchRef[]>('list_remote_branches', { repoId });
}
```

### 3.3 组件契约

- **`BatchCloneDialog.vue`（MOD）**：每个选中仓库行新增分支选择器（可搜索下拉），
  懒加载 `listBranches`，默认选中 `isDefault` 项；加载中/失败有占位与回退。
  提交时把非默认选择写入 `branchOverrides`。
- **`GitFileChanges.vue`（MOD）**：`.section` 改为可折叠分组（点击标题切换），
  `.file-list` 容器 `flex:1; overflow:auto; min-height:0`，实现分组内部滚动。
  折叠状态用组件内 `ref` 保存（仅会话内，不持久化）。
