<!--
  首页仪表盘页面（Phase 10 / T107，对应 spec FR-059）。

  定位：应用启动后的默认落地页，把分散在各 store 的关键状态聚合成一屏概览，
  让用户一眼看清「账号 / 仓库 / 克隆」三大块的规模与待办，并提供常用操作的快捷入口。

  设计要点：
    - 纯聚合视图：数据全部来自现有 store（account / remoteRepository /
      localRepository / cloneTask），本页不持有业务数据，只做读取与展示。
    - 因仪表盘常是首屏，onMounted 时并行拉取四个 store；用 allSettled 让任一失败
      不拖累其余卡片（缺哪块就显示 0 / 空态，而非整页报错）。
    - 不接入 i18n：按项目既定范围，i18n 仅覆盖 Settings 子树，本页用中文字面量。
-->
<template>
  <!-- 页面容器：加载期间整屏 loading 遮罩 -->
  <div v-loading="loading" class="page-dashboard">
    <!-- 顶部标题栏：标题 + 刷新按钮 -->
    <div class="dash-header">
      <h1 class="dash-title">首页</h1>
      <!-- 刷新：重新拉取四个 store，供用户在别处改动后回到首页手动同步 -->
      <el-button :loading="loading" @click="loadAll">刷新</el-button>
    </div>

    <!-- ===================== 8 个统计卡片 ===================== -->
    <!-- 响应式：窄屏 2 列、常规 4 列；共 8 项排两行 -->
    <el-row :gutter="16" class="stat-row">
      <el-col v-for="stat in stats" :key="stat.label" :xs="12" :sm="6" class="stat-col">
        <el-card shadow="hover" class="stat-card">
          <!-- el-statistic 负责数字格式化与标题排版 -->
          <el-statistic :title="stat.label" :value="stat.value" />
        </el-card>
      </el-col>
    </el-row>

    <!-- ===================== 最近仓库 / 最近克隆任务 ===================== -->
    <el-row :gutter="16" class="recent-row">
      <!-- 左：最近仓库（按创建时间倒序取前 5，点击行进入工作区） -->
      <el-col :xs="24" :md="12">
        <el-card shadow="never" class="recent-card">
          <template #header>
            <div class="card-head">
              <span>最近仓库</span>
              <!-- 「查看全部」跳到本地仓库列表 -->
              <el-button link type="primary" @click="go('local-repositories')">
                查看全部
              </el-button>
            </div>
          </template>

          <!-- 无本地仓库时的空态 -->
          <el-empty v-if="recentRepos.length === 0" description="还没有本地仓库" :image-size="80" />
          <!-- 行可点击：进入对应仓库工作区 -->
          <el-table
            v-else
            :data="recentRepos"
            size="small"
            class="clickable-table"
            @row-click="openRepo"
          >
            <el-table-column label="名称" min-width="140">
              <template #default="{ row }">
                <!-- 用路径末段目录名作展示名，比主键直观 -->
                <span class="repo-name">{{ baseName(row.localPath) }}</span>
              </template>
            </el-table-column>
            <el-table-column label="分支" min-width="120">
              <template #default="{ row }">
                <!-- 无分支信息（如刚加入未刷新）时占位短横 -->
                <span>{{ row.currentBranch || '—' }}</span>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag size="small" :type="repoStatusTag(row.status)">
                  {{ repoStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>

      <!-- 右：最近克隆任务（按创建时间倒序取前 5） -->
      <el-col :xs="24" :md="12">
        <el-card shadow="never" class="recent-card">
          <template #header>
            <div class="card-head">
              <span>最近克隆任务</span>
              <!-- 「查看全部」跳到 Clone 中心 -->
              <el-button link type="primary" @click="go('clone-center')">查看全部</el-button>
            </div>
          </template>

          <!-- 无克隆任务时的空态 -->
          <el-empty v-if="recentTasks.length === 0" description="还没有克隆任务" :image-size="80" />
          <el-table v-else :data="recentTasks" size="small">
            <el-table-column label="仓库" min-width="140">
              <template #default="{ row }">
                <span class="repo-name">{{ row.repositoryName }}</span>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="90">
              <template #default="{ row }">
                <el-tag size="small" :type="cloneStatusTag(row.status)">
                  {{ cloneStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="进度" width="100">
              <template #default="{ row }">
                <!-- 仅运行中显示进度条，其余状态进度无意义只显占位 -->
                <el-progress
                  v-if="row.status === 'running'"
                  :percentage="row.progress"
                  :stroke-width="10"
                />
                <span v-else>{{ row.status === 'completed' ? '100%' : '—' }}</span>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- ===================== 底部快捷入口 ===================== -->
    <el-card shadow="never" class="actions-card">
      <template #header>
        <span>快捷操作</span>
      </template>
      <!-- 入口按钮：导航到对应功能页（实际操作在目标页触发，首页只做跳转） -->
      <div class="actions-bar">
        <el-button @click="go('accounts')">添加账号</el-button>
        <el-button @click="go('remote-repositories')">从远程 Clone</el-button>
        <el-button @click="go('local-repositories')">添加本地仓库</el-button>
        <el-button @click="go('local-repositories')">扫描目录</el-button>
        <el-button @click="go('settings')">打开设置</el-button>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
/**
 * 首页仪表盘脚本（T107）。
 *
 * 职责仅三件：① 进页/刷新时并行加载四个 store；② 由 store 数据派生统计与最近列表；
 * ③ 把快捷入口与「最近仓库」行点击映射到路由跳转。不含任何业务写操作。
 */
import { computed, onMounted, ref } from 'vue';
import { ElMessage } from 'element-plus';
import { useRouter } from 'vue-router';

import { useAccountStore } from '@/stores/account';
import { useCloneTaskStore } from '@/stores/cloneTask';
import { useLocalRepositoryStore } from '@/stores/localRepository';
import { useRemoteRepositoryStore } from '@/stores/remoteRepository';
import type { CloneTaskStatus } from '@/types/cloneTask';
import type { LocalRepository, RepositoryStatus } from '@/types/repository';

const router = useRouter();
const accountStore = useAccountStore();
const remoteStore = useRemoteRepositoryStore();
const localStore = useLocalRepositoryStore();
const cloneStore = useCloneTaskStore();

// 整页加载态：任一 store 拉取期间为 true，结束（无论成败）置回 false。
const loading = ref(false);

// 收藏远程仓库数（单独抽出便于统计卡片复用）。
const favoriteCount = computed(() => remoteStore.repositories.filter((r) => r.isFavorite).length);
// 有未提交变更的本地仓库数（status 为 dirty 即工作区有改动）。
const dirtyCount = computed(
  () => localStore.repositories.filter((r) => r.status === 'dirty').length,
);
// 待同步仓库数：与远端有落差（领先 / 落后 / 已分叉），提醒用户 push/pull。
const needSyncCount = computed(
  () =>
    localStore.repositories.filter((r) => ['ahead', 'behind', 'diverged'].includes(r.status))
      .length,
);

// 8 个统计指标：用数组驱动模板 v-for，避免重复的卡片样板代码。
const stats = computed(() => [
  { label: '账号', value: accountStore.accounts.length },
  { label: '远程仓库', value: remoteStore.repositories.length },
  { label: '收藏仓库', value: favoriteCount.value },
  { label: '本地仓库', value: localStore.repositories.length },
  { label: '未提交变更', value: dirtyCount.value },
  { label: '待同步', value: needSyncCount.value },
  { label: '运行中克隆', value: cloneStore.activeCount },
  { label: '克隆任务', value: cloneStore.tasks.length },
]);

// 最近仓库：按创建时间倒序取前 5（后端无 last_opened_at，以创建时间近似「最近」）。
// 先 slice 复制再 sort，避免就地排序污染 store 里的原始顺序。
const recentRepos = computed(() =>
  [...localStore.repositories].sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, 5),
);
// 最近克隆任务：同样按创建时间倒序取前 5。
const recentTasks = computed(() =>
  [...cloneStore.tasks].sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, 5),
);

/** 取路径末段作展示名：兼容 Unix `/` 与 Windows `\` 分隔符。 */
function baseName(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

/** 本地仓库状态 → el-tag 颜色（仅作视觉区分）。 */
function repoStatusTag(status: RepositoryStatus): 'success' | 'info' | 'warning' | 'danger' {
  if (status === 'clean') return 'success'; // 干净
  if (status === 'diverged') return 'danger'; // 已分叉，最需关注
  if (status === 'unknown') return 'info'; // 未知（未刷新 / 无远程）
  return 'warning'; // dirty / ahead / behind 均需用户处理
}

/** 本地仓库状态 → 中文短标签。 */
function repoStatusLabel(status: RepositoryStatus): string {
  const map: Record<RepositoryStatus, string> = {
    clean: '干净',
    dirty: '有变更',
    ahead: '待推送',
    behind: '待拉取',
    diverged: '已分叉',
    unknown: '未知',
  };
  return map[status];
}

/** 克隆任务状态 → el-tag 颜色。 */
function cloneStatusTag(status: CloneTaskStatus): 'success' | 'info' | 'warning' | 'danger' {
  if (status === 'completed') return 'success';
  if (status === 'failed') return 'danger';
  if (status === 'running') return 'warning';
  return 'info'; // pending / cancelled
}

/** 克隆任务状态 → 中文短标签。 */
function cloneStatusLabel(status: CloneTaskStatus): string {
  const map: Record<CloneTaskStatus, string> = {
    pending: '等待中',
    running: '进行中',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消',
  };
  return map[status];
}

/** 快捷入口：按路由名跳转到对应功能页。 */
function go(name: string): void {
  void router.push({ name });
}

/** 点击「最近仓库」某行：进入该仓库的工作区详情页。 */
function openRepo(row: LocalRepository): void {
  void router.push({ name: 'repository-detail', params: { id: row.id } });
}

/**
 * 并行加载四个 store。
 *
 * 用 allSettled 而非 all：某个 store 拉取失败（如网络相关）不应让其余卡片一起空白；
 * 失败时仅弹一次汇总提示，页面照常展示已成功的部分。
 */
async function loadAll(): Promise<void> {
  loading.value = true;
  try {
    const results = await Promise.allSettled([
      accountStore.loadAccounts(),
      remoteStore.fetchList(),
      localStore.fetchAll(),
      cloneStore.fetchAll(),
    ]);
    // 有任一子任务失败就提示用户（不阻断其余数据展示）
    if (results.some((r) => r.status === 'rejected')) {
      ElMessage.warning('部分数据加载失败，可点击「刷新」重试');
    }
  } finally {
    loading.value = false;
  }
}

// 进入首页即加载一次聚合数据
onMounted(loadAll);
</script>

<style scoped>
/* 页面容器：统一内边距 */
.page-dashboard {
  padding: 16px 24px; /* 上下 16、左右 24 */
}

/* 顶部标题栏：标题左、刷新按钮右 */
.dash-header {
  display: flex; /* 横排 */
  align-items: center; /* 垂直居中 */
  justify-content: space-between; /* 两端对齐 */
  margin-bottom: 16px; /* 与统计卡片留间距 */
}

/* 页面主标题字号 */
.dash-title {
  margin: 0; /* 去掉 h1 默认外边距 */
  font-size: 20px; /* 主标题字号 */
}

/* 统计卡片行：底部留出与下方区块的间距 */
.stat-row {
  margin-bottom: 8px; /* 行间距 */
}
/* 每个统计卡片所在列：底部留间距以适配换行的两行布局 */
.stat-col {
  margin-bottom: 16px; /* 两行卡片之间的纵向间距 */
}
/* 统计卡片：固定最小高度让两行卡片视觉对齐 */
.stat-card {
  text-align: center; /* 数字与标题居中 */
}

/* 最近区块行：与底部快捷操作卡片留间距 */
.recent-row {
  margin-bottom: 16px; /* 与下方间距 */
}
/* 最近列表卡片：等高便于左右对齐 */
.recent-card {
  height: 100%; /* 撑满列高，左右卡片底部齐平 */
}

/* 卡片头部：标题左、「查看全部」右 */
.card-head {
  display: flex; /* 横排 */
  align-items: center; /* 垂直居中 */
  justify-content: space-between; /* 两端对齐 */
}

/* 可点击表格：行悬浮显示手型，提示可进入详情 */
.clickable-table :deep(.el-table__row) {
  cursor: pointer; /* 手型光标 */
}

/* 仓库 / 任务名：加粗突出主标识 */
.repo-name {
  font-weight: 600; /* 加粗 */
}

/* 快捷操作卡片内的按钮栏：横向排布并允许换行 */
.actions-bar {
  display: flex; /* 横排 */
  flex-wrap: wrap; /* 窄屏换行 */
  gap: 12px; /* 按钮间距 */
}
</style>
