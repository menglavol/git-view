<!--
  本地仓库管理页面（T073 / US4）。
  功能：
    - 顶部工具栏：添加 / 扫描 / 刷新所有 / 批量 Fetch / 批量移除
    - 主体：LocalRepoTable 展示已加入仓库列表
    - 底部：上次批量 Fetch 的成功/失败摘要
  约束：
    - 删除类操作（移除）使用 ElMessageBox.confirm 二次确认（宪法 Principle III）
    - 仅移除数据库记录，不删除磁盘文件——这点在确认弹窗里明确告知用户
-->
<template>
  <div class="page-local-repositories">
    <!-- 顶部标题与主要操作区 -->
    <div class="page-header">
      <h2 class="page-title">本地仓库</h2>
      <div class="header-actions">
        <!-- 添加单个：弹目录选择对话框 -->
        <ElButton @click="onAdd">添加仓库</ElButton>
        <!-- 扫描父目录：递归识别其中所有 Git 仓库 -->
        <ElButton :loading="store.scanning" @click="onScan">扫描父目录</ElButton>
        <!-- 刷新全部：重跑 git status 更新表格状态 -->
        <ElButton :loading="store.refreshing" @click="onRefreshAll">刷新所有状态</ElButton>
        <!-- 批量 Fetch：仅在有选中行时可点 -->
        <ElButton
          type="primary"
          :disabled="selection.length === 0"
          :loading="store.fetching"
          @click="onBatchFetch"
        >
          批量 Fetch ({{ selection.length }})
        </ElButton>
        <!-- 批量移除：仅在有选中行时可点，触发二次确认 -->
        <ElButton type="danger" plain :disabled="selection.length === 0" @click="onBatchRemove">
          批量移除
        </ElButton>
      </div>
    </div>

    <!-- 选中数量提示条：仅在有选中时显示 -->
    <div v-if="selection.length > 0" class="selection-bar">
      已选 {{ selection.length }} 个仓库
    </div>

    <!-- 仓库列表：所有交互都通过事件向上抛 -->
    <LocalRepoTable
      :items="store.repositories"
      :loading="store.scanning || store.refreshing"
      empty-text="尚未添加任何本地仓库，请点击「添加仓库」或「扫描父目录」"
      @update:selection="selection = $event"
      @open-detail="onOpenDetail"
      @refresh="onRefreshOne"
      @fetch="onFetchOne"
      @open-folder="onOpenFolder"
      @open-terminal="onOpenTerminal"
      @remove="onRemove"
    />

    <!-- 状态总览：单选时显示，验证 RepoStatusOverview 组件可用 -->
    <div v-if="selection.length === 1" class="overview-area">
      <h3 class="overview-title">所选仓库状态</h3>
      <RepoStatusOverview :repo="selection[0]" />
    </div>

    <!-- 上次批量 Fetch 摘要：失败明细列在下方，便于用户排查 -->
    <div v-if="store.lastFetchSummary" class="fetch-summary">
      <h3 class="overview-title">上次批量 Fetch</h3>
      <p>
        共 {{ store.lastFetchSummary.total }} 个，成功
        <span class="ok">{{ store.lastFetchSummary.success }}</span>
        ，失败
        <span class="fail">{{ store.lastFetchSummary.failed }}</span>
      </p>
      <!-- 失败明细：路径 + 已脱敏的错误信息 -->
      <ul v-if="store.lastFetchSummary.failures.length > 0" class="fail-list">
        <li v-for="f in store.lastFetchSummary.failures" :key="f.repoId">
          <span class="path">{{ f.repoName }}</span>
          <span class="err">{{ f.error }}</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
// =====================================================================
// 本地仓库管理页面脚本（T073 / US4）。
// 职责：
//   - 组合工具栏与列表，承接用户操作并调用 store action
//   - 处理对话框（添加 / 扫描）、二次确认（移除）、消息提示
//   - 维护 selection 状态供批量按钮判定
// 注意：本页面是 LocalRepositories 唯一持有副作用与 UI 提示的层；
//       子组件 LocalRepoTable / RepoStatusOverview 只做展示与事件透传。
// =====================================================================
import { onMounted, ref } from 'vue'; // Vue 组合式 API：生命周期 + 响应式
import { ElMessage, ElMessageBox } from 'element-plus'; // 消息提示与确认对话框
import { open as openDialog } from '@tauri-apps/plugin-dialog'; // 系统目录选择

import LocalRepoTable from '@/components/repository/LocalRepoTable.vue'; // 列表组件
import RepoStatusOverview from '@/components/repository/RepoStatusOverview.vue'; // 状态总览组件
import { useLocalRepositoryStore } from '@/stores/localRepository'; // Pinia store
import type { LocalRepository } from '@/types/repository'; // 仓库类型定义

// 仓库 store：state 与 actions 都从此获取，UI 不直接调 API
const store = useLocalRepositoryStore();

/** 当前表格的选中行；用于批量 Fetch / 批量移除入口的状态判定 */
const selection = ref<LocalRepository[]>([]); // 初始为空数组

// 挂载时拉一次列表；失败 toast 提示但不阻塞页面
onMounted(() => {
  // void 显式忽略 Promise，避免 ESLint no-floating-promises 警告
  void store.fetchAll().catch((e) => {
    ElMessage.error(`加载本地仓库失败：${e instanceof Error ? e.message : String(e)}`);
  });
});

// ---------------------------------------------------------------------
// 入口动作：添加 / 扫描 / 刷新
// ---------------------------------------------------------------------

/** 「添加仓库」：弹出目录选择对话框，让用户挑一个仓库根目录。 */
async function onAdd(): Promise<void> {
  try {
    // tauri-plugin-dialog 选目录；多选关闭
    const selected = await openDialog({ directory: true, multiple: false });
    // 用户取消时 selected 为 null / 数组——只处理单字符串路径
    if (typeof selected !== 'string') return;
    await store.addByPath(selected);
    ElMessage.success('已添加仓库');
  } catch (e) {
    // 后端可能因「非 Git 仓库」「路径不存在」等抛错
    ElMessage.error(`添加失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 「扫描父目录」：让用户选一个父目录，递归识别其中所有 Git 仓库。 */
async function onScan(): Promise<void> {
  try {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected !== 'string') return;
    // 默认 max_depth=5（由后端 command 默认值控制），覆盖常见嵌套布局
    const count = await store.scanRoot(selected);
    ElMessage.success(`扫描完成，新增 ${count} 个仓库`);
  } catch (e) {
    ElMessage.error(`扫描失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 「刷新所有状态」：对全部仓库重新跑一遍 git status。 */
async function onRefreshAll(): Promise<void> {
  try {
    // 后端顺序执行避免大量并发 git 子进程
    await store.refreshAll();
    ElMessage.success('状态已刷新');
  } catch (e) {
    ElMessage.error(`刷新失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

// ---------------------------------------------------------------------
// 单行操作：刷新 / Fetch / 打开目录 / 打开终端 / 移除 / 查看详情
// ---------------------------------------------------------------------

/** 单仓库详情：当前 V1 阶段路由占位，留作 US5 RepositoryDetail.vue 接入。 */
function onOpenDetail(repo: LocalRepository): void {
  // US5 时此处将 router.push(`/repositories/${repo.id}`)
  ElMessage.info(`仓库详情功能将在 US5 实现：${repo.localPath}`);
}

/** 单仓库刷新：原地更新表格中对应行。 */
async function onRefreshOne(repo: LocalRepository): Promise<void> {
  try {
    // store.refreshOne 内部维护 refreshing 标志，UI 自动遮罩
    await store.refreshOne(repo.id);
  } catch (e) {
    ElMessage.error(`刷新失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 单仓库 Fetch：复用批量接口（数组长度 1），保证后端路径一致。 */
async function onFetchOne(repo: LocalRepository): Promise<void> {
  try {
    // 仅传入一个 id，复用批量 fetch 后端逻辑，无需单独实现
    await store.batchFetch([repo.id]);
    ElMessage.success('Fetch 完成');
  } catch (e) {
    ElMessage.error(`Fetch 失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 在系统文件管理器中打开仓库目录（macOS Finder / Win Explorer / Linux XDG）。 */
async function onOpenFolder(repo: LocalRepository): Promise<void> {
  try {
    // store.openFolder 内部按平台 spawn 不同进程
    await store.openFolder(repo.id);
  } catch (e) {
    // 失败原因通常是磁盘目录被外部删除
    ElMessage.error(`无法打开目录：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 在系统终端中打开仓库目录（macOS Terminal / Win WT / Linux gnome-terminal 等）。 */
async function onOpenTerminal(repo: LocalRepository): Promise<void> {
  try {
    // Linux 平台后端三档兜底：gnome-terminal → konsole → xterm
    await store.openTerminal(repo.id);
  } catch (e) {
    ElMessage.error(`无法打开终端：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 单仓库移除：弹 ElMessageBox 二次确认，强调不删磁盘文件。 */
async function onRemove(repo: LocalRepository): Promise<void> {
  try {
    await ElMessageBox.confirm(
      `确认从列表中移除「${repo.localPath}」吗？\n\n注意：此操作仅删除 GitView 中的记录，磁盘上的文件不会被删除。`,
      '移除本地仓库', // 弹窗标题
      {
        confirmButtonText: '确认移除', // 主按钮文案
        cancelButtonText: '取消', // 取消按钮文案
        type: 'warning', // 黄色警示图标
      },
    );
  } catch {
    // 用户取消时 ElMessageBox 会 reject——静默返回
    return;
  }
  try {
    await store.removeById(repo.id);
    ElMessage.success('已移除');
  } catch (e) {
    ElMessage.error(`移除失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

// ---------------------------------------------------------------------
// 批量操作
// ---------------------------------------------------------------------

/** 批量 Fetch 选中仓库；摘要写入 store.lastFetchSummary 供 UI 展示。 */
async function onBatchFetch(): Promise<void> {
  // 兜底防护：父按钮已 disable，但显式判断更清晰
  if (selection.value.length === 0) return;
  try {
    const summary = await store.batchFetch(selection.value.map((r) => r.id));
    // 全部成功用 success；有失败用 warning，避免假装通过
    if (summary.failed === 0) {
      ElMessage.success(`已 Fetch ${summary.success} 个仓库`);
    } else {
      ElMessage.warning(`完成：成功 ${summary.success}，失败 ${summary.failed}`);
    }
  } catch (e) {
    ElMessage.error(`批量 Fetch 失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 批量移除：与单条移除共用确认文案模板，强调不删磁盘文件。 */
async function onBatchRemove(): Promise<void> {
  const n = selection.value.length;
  if (n === 0) return;
  try {
    await ElMessageBox.confirm(
      `确认从列表中移除选中的 ${n} 个仓库吗？\n\n注意：此操作仅删除 GitView 中的记录，磁盘上的文件不会被删除。`,
      '批量移除', // 弹窗标题
      {
        confirmButtonText: `确认移除 ${n} 个`, // 主按钮带计数
        cancelButtonText: '取消', // 取消按钮
        type: 'warning', // 警示样式
      },
    );
  } catch {
    // 用户取消
    return;
  }
  // 顺序删除以避免并发对 SQLite 写锁的争抢
  let ok = 0; // 成功计数器
  for (const repo of selection.value) {
    try {
      await store.removeById(repo.id); // 单条 DELETE
      ok += 1;
    } catch {
      // 单条失败不阻塞后续：写入 store.error 但循环继续
    }
  }
  ElMessage.success(`已移除 ${ok} 个仓库`); // 即使部分失败也展示已成功数量
}
</script>

<style scoped>
/* 页面根容器：统一 padding 与列表对齐 */
.page-local-repositories {
  padding: 16px;
}

/* 顶部条：标题在左，操作按钮组在右 */
.page-header {
  align-items: center;
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
}

/* 页面标题：与设计系统其他页面字号一致 */
.page-title {
  font-size: 20px;
  margin: 0;
}

/* 顶部按钮组：横向 flex + 小间距 */
.header-actions {
  display: flex;
  gap: 8px;
}

/* 选中行数提示条：浅色背景突出但不打扰 */
.selection-bar {
  background: var(--el-color-info-light-9);
  border-radius: 4px;
  color: var(--el-text-color-regular);
  font-size: 13px;
  margin-bottom: 12px;
  padding: 6px 12px;
}

/* 总览 / 摘要区共用顶部留白 */
.overview-area,
.fetch-summary {
  margin-top: 16px;
}

/* 子区域小标题样式 */
.overview-title {
  color: var(--el-text-color-regular);
  font-size: 14px;
  margin: 0 0 8px;
}

/* 成功计数着绿色加粗，便于扫读 */
.fetch-summary .ok {
  color: var(--el-color-success);
  font-weight: 600;
}

/* 失败计数着红色加粗 */
.fetch-summary .fail {
  color: var(--el-color-danger);
  font-weight: 600;
}

/* 失败明细列表容器：浅灰背景 + 小字号 */
.fail-list {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  font-size: 12px;
  list-style: none;
  margin: 8px 0 0;
  padding: 8px 12px;
}

/* 失败列表条目：路径与错误左右排列 */
.fail-list li {
  display: flex;
  gap: 12px;
  padding: 2px 0;
}

/* 路径用等宽字体便于辨识 */
.fail-list .path {
  color: var(--el-text-color-regular);
  font-family: var(--el-font-family-mono, monospace);
}

/* 错误文案红色提示 */
.fail-list .err {
  color: var(--el-color-danger);
}
</style>
