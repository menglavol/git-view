# Phase 1 Data Model: 修复设置生效、克隆选分支与变更列表展示

记录本特性涉及的数据结构变更。**问题①③不新增/修改持久化结构**（①复用既有 `GitSettings`，③纯 UI）；仅**问题②**新增字段与一张迁移。

---

## 1. GitSettings（问题①：无结构变更，仅消费方式变更）

现有 `src-tauri/src/models/settings.rs` 已含全部字段，本特性不改结构，仅改「谁读它」：

| 字段 | 类型 | 现状 | 本特性后 |
|---|---|---|---|
| `git_executable_path` | `Option<String>` | 仅 detect/set 用 | 所有 git 操作构造 CLI 时读取 |
| `user_name` / `user_email` | `Option<String>` | 只落库 | commit 时按优先级注入 |
| `default_pull_strategy` | `PullStrategy` | 只落库 | pull 时决定合并方式 |
| `default_push_strategy` | `PushStrategy` | 只落库 | push 时注入 `push.default` |

枚举 `PullStrategy {FfOnly,Rebase,Merge}` / `PushStrategy {Simple,Current,Upstream}` 已存在，不变。

---

## 2. RemoteBranch（问题②：新增，非持久化）

Provider 返回给前端的分支项，仅用于克隆对话框选择，不入库。

```rust
// src-tauri/src/services/provider.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBranch {
    /// 分支名（如 main / develop / feature/x）
    pub name: String,
    /// 是否为该仓库默认分支（前端据此默认选中并排首位）
    pub is_default: bool,
}
```

前端对应 `src/types/repository.ts`：

```ts
export interface RemoteBranch {
  name: string;
  isDefault: boolean;
}
```

**校验/规则**：
- 列表非空时至少一项 `isDefault=true`（由 `repo.default_branch` 匹配确定）。
- 前端展示：默认分支置顶并预选；支持输入过滤（分支多时）。

---

## 3. CloneTask（问题②：新增 branch 字段）

`src-tauri/src/models/clone_task.rs` `CloneTask` 结构新增：

```rust
    /// 要克隆的分支；None = 克隆该仓库默认分支（与旧行为一致）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
```

前端 `src/types/cloneTask.ts` `CloneTask` 增 `branch?: string`。

**状态/生命周期**：不变（pending→running→completed/failed/cancelled）。branch 在建任务时确定，此后只读。

---

## 4. CreateCloneTasksPayload（问题②：新增 branch 覆盖表）

`src-tauri/src/services/clone_task_service.rs`：

```rust
    /// 每个仓库选定的克隆分支（key = remoteRepositoryId）。
    /// 缺省 / 无对应项 = 克隆该仓库默认分支。与 dir_name_overrides 同构。
    #[serde(default)]
    pub branch_overrides: HashMap<String, String>,
```

前端 `src/api/cloneTask.api.ts` `CreateCloneTasksPayload` 增 `branchOverrides: Record<string,string>`。

**建任务规则**：对每个 repo_id，`branch = branch_overrides.get(id)`（去空白后非空才用），否则 `None`。

---

## 5. clone_tasks 表迁移（问题②：迁移 005）

`src-tauri/src/db/migrations/005_add_clone_task_branch.sql`：

```sql
-- 为克隆任务增加"目标分支"列；NULL = 克隆仓库默认分支（兼容旧任务）
ALTER TABLE clone_tasks ADD COLUMN branch TEXT;
```

在 `src-tauri/src/db/migrations.rs` `MIGRATIONS` 末尾追加 version=5 条目（不改历史条目）。

**兼容性**：列可空，旧任务读出 `branch=NULL` → 默认分支，行为与升级前一致。

---

## 6. 文件变更 UI 状态（问题③：仅前端本地状态，无数据结构）

`GitFileChanges.vue` 新增本地 `ref`（不持久化，仅组件内）：

| 状态 | 类型 | 说明 |
|---|---|---|
| `stagedCollapsed` | `ref<boolean>` | 「已暂存」分组折叠状态，默认展开 |
| `unstagedCollapsed` | `ref<boolean>` | 「未暂存」分组折叠状态，默认展开 |

`FileChange` 类型不变。折叠状态默认仅在当前工作区会话内保持（切仓库/重启不持久化）。
