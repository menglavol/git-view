<!--
  单仓库工作区页面（T082 三栏骨架）。

  布局：
    顶栏 — 仓库名 / 当前分支 / ahead-behind / 远程 URL / Fetch · Pull · Push
    左栏 — 文件变更列表（占位，T083 GitFileChanges 替换）
    中栏 — Diff 查看区（占位，T084 DiffViewer 替换）
    右栏 — Commit 面板（占位，T085 CommitPanel 替换）

  说明：
    本骨架仅承接 T082 验收（路由可达、三栏可见）；列表/Diff/Commit/
    BranchSelector/CommitHistory 等具体组件由后续任务 T083-T087 实现。
    页面打开时调用 git_status 拉取一次状态用于顶栏展示。
-->
<template>
  <div class="page-repository-detail">
    <!-- ============ 顶栏：仓库元信息 + 网络操作按钮 ============ -->
    <!-- 顶部条带左侧展示仓库标题/分支/同步指标,右侧是 Fetch/Pull/Push 三个网络按钮 -->
    <header class="repo-header">
      <!-- 仓库元信息分组：仓库名为 h2 标题,下方是 meta 行 -->
      <div class="repo-summary">
        <!-- 仓库标题：取本地路径末段作为显示名,避免直接展示完整路径过长 -->
        <h2 class="repo-name">{{ repoLabel }}</h2>
        <!-- meta 行：分支名 / ahead-behind / 远端 URL 三段并列 -->
        <div class="repo-meta">
          <!-- 当前分支显示;脏 / detached 时仍会展示原分支名,空时显示破折号 -->
          <span class="branch">分支：{{ currentBranch || '—' }}</span>
          <!-- ahead/behind 计数;此处用上下箭头表达"领先/落后远端" -->
          <span class="ahead-behind">↑ {{ status?.ahead ?? 0 }} / ↓ {{ status?.behind ?? 0 }}</span>
          <!-- 远端 URL;鼠标 hover 工具提示展示完整地址,文本过长则截断 -->
          <span class="remote" :title="remoteUrl">{{ remoteUrl || '无远端' }}</span>
        </div>
      </div>
      <!-- 网络操作按钮组：使用 loading 互斥控制,避免并发触发同类 git 网络命令 -->
      <div class="repo-actions">
        <el-button :loading="busyAction === 'fetch'" @click="runFetch">Fetch</el-button>
        <el-button :loading="busyAction === 'pull'" @click="runPull">Pull</el-button>
        <el-button type="primary" :loading="busyAction === 'push'" @click="runPush">Push</el-button>
      </div>
    </header>

    <!-- ============ 主体三栏：变更列表 / Diff / Commit ============ -->
    <!-- 三栏使用 Grid 布局,后续 T083-T085 会把 placeholder 替换为真实组件 -->
    <section class="repo-body">
      <!-- 左栏：文件变更列表占位 (T083 GitFileChanges 接入点) -->
      <aside class="col col-changes">
        <!-- 栏标题：显式标记列功能,便于后续组件迁移时定位 -->
        <h3 class="col-title">变更</h3>
        <div class="placeholder">
          <p>共 {{ status?.changes.length ?? 0 }} 个变更</p>
          <p class="hint">GitFileChanges 组件将在 T083 实现</p>
        </div>
      </aside>

      <!-- 中栏：Diff 查看区占位 (T084 DiffViewer 接入点) -->
      <main class="col col-diff">
        <h3 class="col-title">Diff</h3>
        <div class="placeholder">
          <p>选择左侧文件后显示统一 diff</p>
          <p class="hint">DiffViewer 组件将在 T084 实现</p>
        </div>
      </main>

      <!-- 右栏：Commit 面板占位 (T085 CommitPanel 接入点) -->
      <aside class="col col-commit">
        <h3 class="col-title">提交</h3>
        <div class="placeholder">
          <p>填写 message + description 后提交</p>
          <p class="hint">CommitPanel 组件将在 T085 实现</p>
        </div>
      </aside>
    </section>

    <!-- 状态加载提示与错误提示:放页面底部,不遮挡主体 -->
    <p v-if="loading" class="loading-hint">正在加载仓库状态...</p>
    <p v-if="loadError" class="error-hint">{{ loadError }}</p>
  </div>
</template>

<script setup lang="ts">
/**
 * RepositoryDetail 页面脚本（T082）。
 * 职责：
 *   - 读取路由参数 id 作为 LocalRepository.id
 *   - 拉取仓库基本信息（local store）+ 工作区状态（git_status）
 *   - 触发 Fetch / Pull / Push 网络操作，并在错误时给出中文提示
 *   - 子组件挂载点：T083-T087 后续替换 placeholder
 */
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { ElMessage } from 'element-plus';

// gitApi：封装 src-tauri/src/commands/git.rs 的 15 个 IPC 命令
import { gitApi } from '@/api/git.api';
// localStore：提供已加入 GitView 的本地仓库元信息列表
import { useLocalRepositoryStore } from '@/stores/localRepository';
// 类型声明：与 Rust 端 GitStatus / LocalRepository 模型一一对应
import type { GitStatus } from '@/types/git';
import type { LocalRepository } from '@/types/repository';

// route：取出动态参数 id；store：仓库元信息共享数据源
const route = useRoute();
const localStore = useLocalRepositoryStore();

/** 路由参数 id 对应 local_repositories.id。 */
const repoId = computed(() => String(route.params.id ?? ''));

/** 工作区状态（首次进入页面时拉取一次，后续由 fetch/pull/push 后刷新）。 */
const status = ref<GitStatus | null>(null);
/** 顶栏状态加载中标志，避免重复请求并支撑骨架屏 */
const loading = ref(false);
/** 加载失败时记录错误文本，模板内显式展示，避免静默失败 */
const loadError = ref<string>('');
/** 当前正在执行的网络操作类型；用于按钮 loading 与互斥控制 */
const busyAction = ref<'fetch' | 'pull' | 'push' | ''>('');

/** 从 store 中读取当前仓库的基础元信息（路径、远端 URL、本地分支等）。 */
const repoMeta = computed(() =>
  localStore.repositories.find((r: LocalRepository) => r.id === repoId.value),
);
// 仓库展示名：取本地路径最后一段，回退到完整路径，再回退到 id
const repoLabel = computed(
  () => repoMeta.value?.localPath?.split('/').pop() ?? repoMeta.value?.localPath ?? repoId.value,
);
// 远端 URL 用于顶栏标题工具提示展示
const remoteUrl = computed(() => repoMeta.value?.remoteUrl ?? '');
// 当前分支优先取 status 解析结果，fallback 到 store 缓存的字段
const currentBranch = computed(
  () => status.value?.currentBranch ?? repoMeta.value?.currentBranch ?? '',
);

/** 加载仓库状态，统一封装错误展示。 */
async function loadStatus(): Promise<void> {
  // 路由尚未注入 id 时跳过，避免对空字符串发起后端请求
  if (!repoId.value) return;
  loading.value = true;
  loadError.value = '';
  try {
    status.value = await gitApi.status(repoId.value);
  } catch (e) {
    // 错误对象可能不是 Error 实例（Tauri 透传的 GitViewClientError），统一转字符串
    loadError.value = `状态加载失败：${e instanceof Error ? e.message : String(e)}`;
  } finally {
    loading.value = false;
  }
}

/** 通用网络操作执行器：统一处理 busy 标志、错误展示与状态刷新。 */
async function runNetworkAction(
  kind: 'fetch' | 'pull' | 'push',
  fn: () => Promise<string>,
): Promise<void> {
  // 已有操作进行中时直接忽略点击，避免并发触发 git 网络命令
  if (busyAction.value) return;
  busyAction.value = kind;
  try {
    await fn();
    ElMessage.success(`${kind.toUpperCase()} 成功`);
    // 成功后重新拉取 status，让 ahead/behind 指标即时反映远端变化
    await loadStatus();
  } catch (e) {
    // 失败时不阻塞操作（busy 标志在 finally 中清空），让用户能立即重试
    ElMessage.error(`${kind.toUpperCase()} 失败：${e instanceof Error ? e.message : String(e)}`);
  } finally {
    busyAction.value = '';
  }
}

// 三个网络操作的薄包装：保留显式命名以便模板里语义清晰
const runFetch = (): Promise<void> => runNetworkAction('fetch', () => gitApi.fetch(repoId.value));
const runPull = (): Promise<void> => runNetworkAction('pull', () => gitApi.pull(repoId.value));
const runPush = (): Promise<void> => runNetworkAction('push', () => gitApi.push(repoId.value));

onMounted(() => {
  // 若本地仓库 store 尚未加载，先拉一次列表以拿到当前仓库的元信息
  if (localStore.repositories.length === 0) {
    void localStore.fetchAll();
  }
  // 不等 store 加载结果即可发起 status 请求；二者互不依赖，并行更快
  void loadStatus();
});
</script>

<style scoped>
/* ===== 整体页面：纵向排布顶栏 + 主体 ===== */
/* 使用 flex column 让顶栏自适应高度，主体 flex:1 填满剩余空间 */
.page-repository-detail {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 8px;
  gap: 8px;
}

/* ===== 顶栏：仓库元信息 + 网络操作按钮 ===== */
/* 横向布局：左侧元信息靠左，右侧按钮组靠右 */
.repo-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--el-bg-color-page);
  border-radius: 4px;
}
/* 仓库名：较大字号、加粗效果由 h2 自带 */
.repo-name {
  font-size: 16px;
  margin: 0 0 4px 0;
}
/* 元信息行：分支、ahead/behind、远端 URL 用 gap 间隔 */
.repo-meta {
  display: flex;
  gap: 16px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}
/* 远端 URL 较长时截断显示，hover 通过 title 属性看完整 */
.repo-meta .remote {
  max-width: 280px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ===== 三栏主体：使用 CSS Grid，左中右比例 1:2:1 ===== */
/* min-height:0 让 Grid 子元素允许 overflow 滚动 */
.repo-body {
  display: grid;
  grid-template-columns: 1fr 2fr 1fr;
  gap: 8px;
  flex: 1;
  min-height: 0;
}
/* 单栏容器：内部独立滚动，背景色与顶栏一致形成视觉分组 */
.col {
  display: flex;
  flex-direction: column;
  background: var(--el-bg-color-page);
  border-radius: 4px;
  padding: 8px;
  overflow: auto;
}
/* 栏标题：紧贴顶部、字号小于页面标题以建立信息层级 */
.col-title {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: var(--el-text-color-primary);
}
/* 占位文案：次要颜色，避免与未来真实数据产生视觉冲突 */
.placeholder {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
/* 占位 hint 行：斜体提示后续 Phase 实现，便于设计师与开发对齐进度 */
.placeholder .hint {
  margin-top: 4px;
  font-style: italic;
  color: var(--el-text-color-placeholder);
}

/* ===== 加载与错误提示：放在主体下方而非 overlay，避免遮挡数据 ===== */
.loading-hint {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
/* 错误用主题红色，区别于加载提示 */
.error-hint {
  color: var(--el-color-danger);
  font-size: 13px;
}
</style>
