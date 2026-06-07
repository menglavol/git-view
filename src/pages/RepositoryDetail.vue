<!--
  单仓库工作区页面(US5 完整版,T082 骨架 → T088 接入)。

  布局:
    顶栏 — 仓库名 / BranchSelector / ahead-behind / 远程 URL / Fetch · Pull · Push
    左栏 — GitFileChanges 文件变更列表
    中栏 — DiffViewer + CommitHistory tab 切换
    右栏 — CommitPanel 提交面板

  数据流:
    - 本页集中持有 status / branches / selectedFile / diffResult,作为数据源
    - 子组件通过 props 接收数据,通过 emit 通知父组件触发 gitApi
    - 任何写操作(stage / commit / fetch / pull / push / checkout)成功后
      统一刷新 status 与 branches,确保 UI 实时反映

  T088 错误提示:
    - pull 失败 → ElMessageBox 警告冲突场景
    - push 失败 → ElMessageBox 警告 non-fast-forward / no upstream
    - 错误对象按 GitViewError code 分类提示
-->
<template>
  <div class="page-repository-detail">
    <!-- ============ 顶栏 ============ -->
    <!-- 顶部条带左侧:仓库名 / 分支选择器 / ahead-behind / 远端 URL -->
    <header class="repo-header">
      <div class="repo-summary">
        <!-- 仓库标题:取本地路径末段 -->
        <div class="repo-title-row">
          <el-button text size="small" class="back-btn" @click="goBack">← 返回</el-button>
          <h2 class="repo-name">{{ repoLabel }}</h2>
        </div>
        <div class="repo-meta">
          <!-- 分支选择器组件:脏工作区下自身处理 disable -->
          <BranchSelector
            :branches="branches"
            :current-name="currentBranch"
            :is-dirty="isDirty"
            @switch-local="onSwitchLocalBranch"
            @switch-remote="onSwitchRemoteBranch"
          />
          <!-- ahead/behind 计数 -->
          <span class="ahead-behind">↑ {{ status?.ahead ?? 0 }} / ↓ {{ status?.behind ?? 0 }}</span>
          <!-- 远端 URL -->
          <span class="remote" :title="remoteUrl">{{ remoteUrl || '无远端' }}</span>
        </div>
      </div>
      <!-- 网络操作按钮组:互斥控制避免并发 -->
      <div class="repo-actions">
        <!-- 无 origin 远端时显示「发布到远程」入口（建空仓 + 关联 + push） -->
        <el-button v-if="!remoteUrl" type="primary" @click="openPublishDialog">
          发布到远程
        </el-button>
        <el-button :loading="busyAction === 'fetch'" @click="runFetch">Fetch</el-button>
        <el-button :loading="busyAction === 'pull'" @click="runPull">Pull</el-button>
        <el-button type="primary" :loading="busyAction === 'push'" @click="runPush">Push</el-button>
      </div>
    </header>

    <!-- ============ 主体三栏 ============ -->
    <section class="repo-body">
      <!-- 左栏:文件变更列表 -->
      <aside class="col col-changes">
        <GitFileChanges
          v-if="status"
          :changes="status.changes"
          @view-diff="onViewDiff"
          @stage-file="onStageFile"
          @unstage-file="onUnstageFile"
          @stage-all="onStageAll"
          @unstage-all="onUnstageAll"
          @discard-confirmed="onDiscardConfirmed"
        />
        <div v-else class="placeholder">{{ loadError || '加载中...' }}</div>
        <!-- 文件单击切换 stage,行内"暂存"/"丢弃"按钮已在 GitFileChanges 内部触发事件 -->
        <!-- 选中文件状态:在变更列表下方提示当前查看的 diff 文件 -->
        <div v-if="selectedFile" class="selected-file">
          当前查看:<span class="mono">{{ selectedFile }}</span>
        </div>
      </aside>

      <!-- 中栏:Diff 查看器 + 切换到提交历史 -->
      <main class="col col-diff">
        <!-- 视图切换 tabs:diff vs commit history -->
        <el-tabs v-model="middleTab" class="middle-tabs">
          <el-tab-pane label="Diff" name="diff">
            <DiffViewer :result="diffResult" />
          </el-tab-pane>
          <el-tab-pane label="提交历史" name="history">
            <!-- 选中提交后展示详情面板（带返回），否则展示提交列表 -->
            <div v-if="selectedCommitSha" class="commit-detail-wrap">
              <el-button text size="small" class="back-btn" @click="backToCommitList">
                ← 返回提交列表
              </el-button>
              <CommitDetailPanel :detail="commitDetail" :loading="commitDetailLoading" />
            </div>
            <CommitHistory
              v-else
              :key="repoId"
              :load-page="loadLocalCommits"
              @select="onSelectCommit"
            />
          </el-tab-pane>
        </el-tabs>
      </main>

      <!-- 右栏:Commit 面板 -->
      <aside class="col col-commit">
        <CommitPanel
          ref="commitPanelRef"
          :staged-count="stagedCount"
          :submitting="busyAction === 'commit'"
          @submit="onCommitSubmit"
        />
      </aside>
    </section>

    <!-- 加载/错误提示放底部,避免遮挡主体 -->
    <p v-if="loading && !status" class="loading-hint">正在加载仓库状态...</p>
    <p v-if="loadError" class="error-hint">{{ loadError }}</p>

    <!-- 发布到远程对话框（仅无 origin 时通过顶栏按钮触发） -->
    <PublishToRemoteDialog
      v-model="publishDialogVisible"
      :repo-id="repoId"
      :default-name="repoLabel"
      :current-branch="currentBranch || undefined"
      @published="onPublished"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * RepositoryDetail 完整版(T082 + T083-T088)。
 *
 * 数据中枢职责:
 *   - 拉取 status / branches / diff;
 *   - 监听文件变更列表事件,调用 gitApi 并刷新数据;
 *   - 处理 fetch / pull / push 网络操作 + 中文友好错误提示(T088);
 *   - 处理 commit 流程(包含 5 项前置校验失败时的 Internal 错误提示);
 *   - 处理 BranchSelector 切换分支 + DirtyWorkdir 错误兜底提示。
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ElMessage, ElMessageBox } from 'element-plus';

// gitApi:封装 src-tauri/src/commands/git.rs 的 15 个 IPC 命令
import { gitApi } from '@/api/git.api';
// localStore:仓库元信息共享 store
import { useLocalRepositoryStore } from '@/stores/localRepository';
// 子组件挂载
import GitFileChanges from '@/components/git/GitFileChanges.vue';
import DiffViewer from '@/components/git/DiffViewer.vue';
import CommitPanel from '@/components/git/CommitPanel.vue';
import CommitHistory from '@/components/git/CommitHistory.vue';
import CommitDetailPanel from '@/components/git/CommitDetailPanel.vue';
import BranchSelector from '@/components/git/BranchSelector.vue';
import PublishToRemoteDialog from '@/components/repository/PublishToRemoteDialog.vue';
// 类型定义
import type {
  Branch,
  CommitDetail,
  CommitSummary,
  DiffResult,
  FileChange,
  GitStatus,
} from '@/types/git';
import type { LocalRepository } from '@/types/repository';

const route = useRoute();
const router = useRouter();

/** 返回本地仓库列表页（固定跳转，不依赖浏览器历史）。 */
function goBack(): void {
  // 标记本次为「返回」，列表页据此恢复进入详情前的展开状态
  localStore.markRestoreExpandOnReturn();
  void router.push({ name: 'local-repositories' });
}
const localStore = useLocalRepositoryStore();

/** 路由参数 id,对应 local_repositories.id */
const repoId = computed(() => String(route.params.id ?? ''));

// ============ 数据状态 ============
/** 工作区聚合状态 */
const status = ref<GitStatus | null>(null);
/** 分支列表(本地 + 远程) */
const branches = ref<Branch[]>([]);
/** 当前查看 diff 的文件路径;null 时 DiffViewer 显示提示 */
const selectedFile = ref<string | null>(null);
/** Diff 文本结果 */
const diffResult = ref<DiffResult | null>(null);
/** 初始加载标志 */
const loading = ref(false);
/** 加载错误文案,模板显示 */
const loadError = ref('');
/** 当前网络/提交操作的类型,用于按钮 loading 与互斥控制 */
const busyAction = ref<'fetch' | 'pull' | 'push' | 'commit' | ''>('');
/** 中栏 tab 状态:diff vs 提交历史 */
const middleTab = ref<'diff' | 'history'>('diff');
/** 提交历史中选中的提交 sha（null 显示列表，否则显示详情面板） */
const selectedCommitSha = ref<string | null>(null);
/** 选中提交的详情 */
const commitDetail = ref<CommitDetail | null>(null);
/** 提交详情加载中 */
const commitDetailLoading = ref(false);
/** CommitPanel ref,提交成功后调用其 reset() 清空输入 */
const commitPanelRef = ref<InstanceType<typeof CommitPanel> | null>(null);

// ============ 派生计算 ============
/** 当前仓库的元信息,缺失时显示 id */
const repoMeta = computed(() =>
  localStore.repositories.find((r: LocalRepository) => r.id === repoId.value),
);
const repoLabel = computed(
  () => repoMeta.value?.localPath?.split('/').pop() ?? repoMeta.value?.localPath ?? repoId.value,
);
const remoteUrl = computed(() => repoMeta.value?.remoteUrl ?? '');
/** 当前分支:优先 status,fallback 到 store 缓存 */
const currentBranch = computed(
  () => status.value?.currentBranch ?? repoMeta.value?.currentBranch ?? '',
);
/** 是否脏工作区:用于 BranchSelector 阻断 */
const isDirty = computed(() => !(status.value?.isClean ?? true));
/** 已暂存文件数:用于 CommitPanel 按钮启用 */
const stagedCount = computed(
  () => (status.value?.changes ?? []).filter((c: FileChange) => c.staged).length,
);

// ============ 数据加载 ============

/** 加载 status + 分支列表,统一处理错误。 */
async function loadAll(): Promise<void> {
  if (!repoId.value) return;
  loading.value = true;
  loadError.value = '';
  try {
    // 并发加载 status 与 branches,加快首屏可见时间
    const [s, b] = await Promise.all([
      gitApi.status(repoId.value),
      gitApi.listBranches(repoId.value).catch(() => [] as Branch[]),
    ]);
    status.value = s;
    branches.value = b;
  } catch (e) {
    loadError.value = `状态加载失败:${formatError(e)}`;
  } finally {
    loading.value = false;
  }
}

/** 重新加载 diff,基于当前 selectedFile;cached 行为留待后续按钮切换。 */
async function reloadDiff(): Promise<void> {
  if (!selectedFile.value) {
    diffResult.value = null;
    return;
  }
  try {
    diffResult.value = await gitApi.diff(repoId.value, selectedFile.value, false);
  } catch (e) {
    diffResult.value = null;
    ElMessage.error(`Diff 加载失败:${formatError(e)}`);
  }
}

// ============ 文件操作 ============

/** 单击文件行:在右侧查看其 diff（切到 Diff tab），不改变暂存状态。 */
async function onViewDiff(path: string): Promise<void> {
  selectedFile.value = path;
  middleTab.value = 'diff';
  await reloadDiff();
}

/** 本地提交分页加载：git_log 映射为通用 CommitSummary 喂给 CommitHistory。 */
async function loadLocalCommits(page: number, pageSize: number): Promise<CommitSummary[]> {
  const list = await gitApi.log(repoId.value, page, pageSize);
  // 本地 CommitInfo → CommitSummary（列表只需这几个字段；本地提交无 htmlUrl）
  return list.map((c) => ({
    sha: c.sha,
    shortSha: c.shortSha,
    summary: c.summary,
    authorName: c.authorName,
    authoredAt: c.authoredAt,
  }));
}

/** 点击提交行：记录选中并加载其详情。 */
async function onSelectCommit(sha: string): Promise<void> {
  selectedCommitSha.value = sha;
  commitDetailLoading.value = true;
  commitDetail.value = null;
  try {
    commitDetail.value = await gitApi.commitDetail(repoId.value, sha);
  } catch (e) {
    ElMessage.error(`提交详情加载失败:${formatError(e)}`);
    selectedCommitSha.value = null;
  } finally {
    commitDetailLoading.value = false;
  }
}

/** 返回提交列表（清除选中与详情）。 */
function backToCommitList(): void {
  selectedCommitSha.value = null;
  commitDetail.value = null;
}

/** stage 单文件:成功后刷新 status 与当前 diff（不改 selectedFile，避免正在看的 diff 被切走） */
async function onStageFile(path: string): Promise<void> {
  try {
    await gitApi.stageFile(repoId.value, path);
    await Promise.all([loadAll(), reloadDiff()]);
  } catch (e) {
    ElMessage.error(`暂存失败:${formatError(e)}`);
  }
}

/** unstage 单文件 */
async function onUnstageFile(path: string): Promise<void> {
  try {
    await gitApi.unstageFile(repoId.value, path);
    await Promise.all([loadAll(), reloadDiff()]);
  } catch (e) {
    ElMessage.error(`取消暂存失败:${formatError(e)}`);
  }
}

/** 全部 stage:对工作区全部变更生效 */
async function onStageAll(): Promise<void> {
  try {
    await gitApi.stageAll(repoId.value);
    await loadAll();
  } catch (e) {
    ElMessage.error(`全部暂存失败:${formatError(e)}`);
  }
}

/** 全部 unstage:清空整个暂存区 */
async function onUnstageAll(): Promise<void> {
  try {
    await gitApi.unstageAll(repoId.value);
    await loadAll();
  } catch (e) {
    ElMessage.error(`全部取消暂存失败:${formatError(e)}`);
  }
}

/** discard 已通过子组件 ConfirmDangerDialog 二次确认;此处直接调用并刷新 */
async function onDiscardConfirmed(paths: string[]): Promise<void> {
  try {
    await gitApi.discardChanges(repoId.value, paths, true);
    ElMessage.success(`已丢弃 ${paths.length} 个文件的变更`);
    await Promise.all([loadAll(), reloadDiff()]);
  } catch (e) {
    ElMessage.error(`丢弃失败:${formatError(e)}`);
  }
}

// ============ 提交 ============

/** CommitPanel emit('submit') 回调:执行 commit 并刷新 */
async function onCommitSubmit(payload: { message: string; description?: string }): Promise<void> {
  if (busyAction.value) return;
  busyAction.value = 'commit';
  try {
    await gitApi.commit(repoId.value, payload.message, payload.description);
    ElMessage.success('提交成功');
    commitPanelRef.value?.reset();
    await loadAll();
  } catch (e) {
    // 后端可能返回 Internal(中文阻断原因) 或 GitCommand 等;统一展示给用户
    ElMessage.error(`提交失败:${formatError(e)}`);
  } finally {
    busyAction.value = '';
  }
}

// ============ 网络操作 + T088 错误提示 ============

/** 通用执行器:Fetch / Pull / Push */
async function runNetwork(kind: 'fetch' | 'pull' | 'push'): Promise<void> {
  if (busyAction.value) return;
  busyAction.value = kind;
  try {
    if (kind === 'fetch') await gitApi.fetch(repoId.value);
    else if (kind === 'pull') await gitApi.pull(repoId.value);
    else await gitApi.push(repoId.value);
    ElMessage.success(`${kind.toUpperCase()} 成功`);
    await loadAll();
  } catch (e) {
    // T088:Pull/Push 失败按场景给中文友好提示
    handleNetworkError(kind, e);
  } finally {
    busyAction.value = '';
  }
}
const runFetch = (): Promise<void> => runNetwork('fetch');
const runPull = (): Promise<void> => runNetwork('pull');
const runPush = (): Promise<void> => runNetwork('push');

/** T088:网络错误的中文友好提示,基于错误信息关键词分类。 */
function handleNetworkError(kind: 'fetch' | 'pull' | 'push', e: unknown): void {
  const msg = formatError(e);
  const lower = msg.toLowerCase();
  // Pull:冲突场景需要外部工具解决
  if (kind === 'pull' && lower.includes('冲突')) {
    void ElMessageBox.alert(
      'Pull 后存在冲突,请使用外部工具(如 VS Code 或 git mergetool)解决冲突后再提交。',
      'Pull 冲突',
      { confirmButtonText: '我知道了', type: 'warning' },
    );
    return;
  }
  // Pull:分叉无法快进
  if (kind === 'pull' && (lower.includes('分叉') || lower.includes('fast-forward'))) {
    void ElMessageBox.alert(
      '本地与远程分支已分叉,无法快进合并。请使用外部工具解决冲突后再继续。',
      'Pull 失败',
      { confirmButtonText: '我知道了', type: 'warning' },
    );
    return;
  }
  // Push:被拒绝(non-fast-forward)
  if (kind === 'push' && (lower.includes('被拒绝') || lower.includes('rejected'))) {
    void ElMessageBox.alert(
      '推送被拒绝(远程有新提交),请先 Pull 远程更新后再推送。',
      'Push 被拒绝',
      { confirmButtonText: '我知道了', type: 'warning' },
    );
    return;
  }
  // Push:缺少 upstream
  if (kind === 'push' && lower.includes('upstream')) {
    void ElMessageBox.alert(
      '当前分支没有 upstream,请运行 git push -u origin <branch> 设置后再推送。',
      'Push 失败',
      { confirmButtonText: '我知道了', type: 'warning' },
    );
    return;
  }
  // 兜底:普通 toast
  ElMessage.error(`${kind.toUpperCase()} 失败:${msg}`);
}

// ============ 发布到远程 ============

/** 发布对话框可见性（仅无 origin 远端时顶栏入口可用）。 */
const publishDialogVisible = ref(false);

/** 打开「发布到远程」对话框。 */
function openPublishDialog(): void {
  publishDialogVisible.value = true;
}

/** 发布成功后刷新本地仓库元信息（remoteUrl 更新后入口自动隐藏）与工作区状态。 */
async function onPublished(): Promise<void> {
  await localStore.fetchAll().catch(() => undefined);
  await loadAll();
}

// ============ 分支切换 ============

/** 切换本地分支:DirtyWorkdir 时按钮已被 BranchSelector disable,这里兜底处理后端返回。 */
async function onSwitchLocalBranch(name: string): Promise<void> {
  try {
    await gitApi.checkoutBranch(repoId.value, name);
    ElMessage.success(`已切换到 ${name}`);
    await loadAll();
  } catch (e) {
    const msg = formatError(e);
    if (msg.includes('未提交变更')) {
      ElMessage.warning('存在未提交变更,请先提交或暂存后再切换分支');
    } else {
      ElMessage.error(`切换分支失败:${msg}`);
    }
  }
}

/** 从远程分支 checkout 出新本地分支(自动 upstream):remoteName 如 origin/feature-x */
async function onSwitchRemoteBranch(remoteName: string): Promise<void> {
  // 去掉 origin/ 前缀作为新本地分支名;若用户名不合法 git 会自报错
  const localName = remoteName.replace(/^origin\//, '').replace(/^remotes\/[^/]+\//, '');
  try {
    await gitApi.createBranch(repoId.value, localName, true);
    ElMessage.success(`已从 ${remoteName} 创建本地分支 ${localName}`);
    await loadAll();
  } catch (e) {
    ElMessage.error(`创建本地分支失败:${formatError(e)}`);
  }
}

// ============ 工具函数 ============

/** 错误对象规范化为字符串,兼容 Error / GitViewClientError / 任意 */
function formatError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === 'string') return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

// ============ 生命周期 ============

onMounted(() => {
  if (localStore.repositories.length === 0) {
    void localStore.fetchAll();
  }
  void loadAll();
});

/** repoId 变化时重置选中文件并重新加载 */
watch(repoId, () => {
  selectedFile.value = null;
  diffResult.value = null;
  selectedCommitSha.value = null;
  commitDetail.value = null;
  void loadAll();
});

/** 当 selectedFile 变化时自动加载 diff */
watch(selectedFile, () => {
  void reloadDiff();
});
</script>

<style scoped>
/* ===== 整体页面:纵向排布顶栏 + 主体 ===== */
.page-repository-detail {
  display: flex; /* flex 用于纵向布局 */
  flex-direction: column; /* 顶栏在上,主体在下 */
  height: 100%; /* 占满父高 */
  padding: 8px; /* 页面四周留白 */
  gap: 8px; /* 顶栏与主体间隔 */
}

/* ===== 顶栏 ===== */
.repo-header {
  display: flex; /* 横排:左信息 + 右按钮 */
  align-items: center; /* 垂直居中 */
  justify-content: space-between; /* 两端对齐 */
  padding: 8px 12px; /* 内边距 */
  background: var(--el-bg-color-page); /* 浅背景区分 */
  border-radius: 4px; /* 圆角 */
}
.repo-title-row {
  display: flex; /* 返回按钮与仓库名同一行 */
  align-items: center; /* 垂直居中 */
  gap: 8px; /* 按钮与标题间距 */
}
.back-btn {
  padding: 0; /* 文字按钮去内边距，更像返回链接 */
}
.repo-name {
  font-size: 16px; /* 标题字号 */
  margin: 0 0 4px 0; /* 与 meta 行留间距 */
}
.repo-meta {
  display: flex; /* 横排各段元信息 */
  gap: 16px; /* 段间距 */
  align-items: center; /* 垂直居中 */
  font-size: 13px; /* 元信息字号 */
  color: var(--el-text-color-secondary); /* 次色 */
}
.repo-meta .remote {
  max-width: 280px; /* 限宽避免挤压其他元素 */
  white-space: nowrap; /* 不换行 */
  overflow: hidden; /* 溢出隐藏 */
  text-overflow: ellipsis; /* 末尾省略号 */
}

/* ===== 三栏主体 ===== */
.repo-body {
  display: grid; /* Grid 实现 1:2:1 三栏 */
  grid-template-columns: 1fr 2fr 1fr; /* 中栏更宽承载 diff */
  gap: 8px; /* 列间距 */
  flex: 1; /* 占据剩余空间 */
  min-height: 0; /* 允许 Grid 子元素 overflow */
}
/* 单栏:背景与内边距统一 */
.col {
  display: flex; /* 纵向布局 */
  flex-direction: column; /* 子元素上下堆叠 */
  background: var(--el-bg-color-page); /* 浅背景区分 */
  border-radius: 4px; /* 圆角 */
  padding: 8px; /* 栏内边距 */
  overflow: hidden; /* 隐藏溢出,内部自滚动 */
  min-height: 0; /* 允许 flex 子元素滚动 */
}

/* 中栏 tabs 占满整个栏高 */
.middle-tabs {
  flex: 1; /* 占满父栏 */
  display: flex; /* flex 用于嵌套 */
  flex-direction: column; /* 上为 tab header,下为内容 */
  min-height: 0; /* 允许内部滚动 */
}

/* 提交详情视图:返回按钮 + 详情面板纵向铺满并自滚动 */
.commit-detail-wrap {
  display: flex; /* 纵向布局 */
  flex-direction: column; /* 返回按钮在上、详情在下 */
  height: 100%; /* 占满 tab 内容区 */
  min-height: 0; /* 允许详情面板内部滚动 */
}
.commit-detail-wrap .back-btn {
  align-self: flex-start; /* 返回按钮靠左 */
  margin-bottom: 4px; /* 与详情面板留间距 */
}
.commit-detail-wrap :deep(.commit-detail) {
  flex: 1; /* 详情面板占满剩余高度 */
  min-height: 0; /* 允许其内部 overflow 滚动 */
}
.middle-tabs :deep(.el-tabs__content) {
  flex: 1; /* 内容区占满 */
  min-height: 0; /* 允许滚动 */
  overflow: auto; /* 自动滚动 */
}
.middle-tabs :deep(.el-tab-pane) {
  height: 100%; /* tab 内容占满 */
}

/* 当前查看文件提示:左栏底部 */
.selected-file {
  font-size: 12px; /* 小字提示 */
  color: var(--el-text-color-secondary); /* 次色 */
  padding: 4px 0; /* 上下间距 */
  border-top: 1px solid var(--el-border-color-lighter); /* 顶部分割线 */
  margin-top: 4px; /* 与列表留间距 */
}
.mono {
  font-family: var(--el-font-family-mono, monospace); /* 等宽路径 */
}

/* 占位文案 */
.placeholder {
  color: var(--el-text-color-secondary); /* 次色 */
  font-size: 13px; /* 提示字号 */
  text-align: center; /* 居中 */
  padding: 16px; /* 留白 */
}

/* 底部加载/错误提示 */
.loading-hint {
  color: var(--el-text-color-secondary); /* 次色 */
  font-size: 12px; /* 小字 */
}
.error-hint {
  color: var(--el-color-danger); /* 危险红 */
  font-size: 13px; /* 略大字号便于读 */
}
</style>
