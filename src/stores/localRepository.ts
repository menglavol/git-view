// =====================================================================
// 本地仓库 store（T073 / US4）
// =====================================================================
// 职责：作为 LocalRepositories 页面与下属组件的状态中枢，封装：
//   1) 仓库列表 / 加载状态 / 错误的响应式管理
//   2) 对底层 localRepositoryApi 的 action 包装（带 loading 切换、错误归一化）
//   3) 跨组件共享的批量 Fetch 汇总结果（lastFetchSummary）
// 设计原则：
//   - state 仅为「最近一次操作的状态快照」，组件不直接读底层 API
//   - actions 内统一在 try/finally 中切换 loading 标志，避免 UI 出现挂起
//   - 错误统一抓取后写入 error 并再次 throw，便于上层 UI 弹 ElMessage
// =====================================================================

import { defineStore } from 'pinia';
import { ref } from 'vue';

import { localRepositoryApi } from '@/api/localRepository.api';
import type { BatchFetchSummary, LocalRepository } from '@/types/repository';

// Pinia 组合式 store：返回 setup 风格的 ref / function，
// 模板 / 组件可直接解构使用而不丢失响应性。
export const useLocalRepositoryStore = defineStore('localRepository', () => {
  /** 已加入的本地仓库列表（与后端 SELECT 结果对齐） */
  const repositories = ref<LocalRepository[]>([]);
  /** 扫描父目录进行中：避免重复触发扫描或在过程中切换其他工具栏入口 */
  const scanning = ref(false);
  /** 刷新工作区状态进行中（单仓库或全量）：用于表格 loading 遮罩 */
  const refreshing = ref(false);
  /** 批量 Fetch 进行中：禁用「批量 Fetch」按钮，避免重复提交 */
  const fetching = ref(false);
  /** 最近一次错误（已脱敏）：UI 顶部 banner 可订阅展示 */
  const error = ref<string | null>(null);
  /** 最近一次批量 Fetch 的结果摘要，前端可在工具栏下显示成功/失败计数 */
  const lastFetchSummary = ref<BatchFetchSummary | null>(null);

  /** 拉取全部本地仓库，覆盖 repositories（页面 onMounted 与多数 action 完成后调用）。 */
  async function fetchAll(): Promise<void> {
    error.value = null;
    try {
      // 列表查询是只读 SELECT，不切 refreshing 标志
      repositories.value = await localRepositoryApi.list();
    } catch (e) {
      // 错误归一化为字符串以便 UI 直接渲染
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  /** 添加单个本地仓库（路径由 dialog 选定后传入）。 */
  async function addByPath(path: string): Promise<LocalRepository> {
    error.value = null;
    const repo = await localRepositoryApi.add(path);
    // 添加后若已存在则后端返回旧记录；统一以 fetchAll 拉取全量，避免重复推入
    await fetchAll();
    return repo;
  }

  /** 扫描父目录批量添加；scanning 状态在期间为 true。 */
  async function scanRoot(root: string, maxDepth?: number): Promise<number> {
    scanning.value = true;
    error.value = null;
    try {
      const added = await localRepositoryApi.scan(root, maxDepth);
      await fetchAll();
      return added.length;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      scanning.value = false;
    }
  }

  /** 从列表移除（前端 UI 须先做二次确认）。 */
  async function removeById(id: string): Promise<void> {
    await localRepositoryApi.remove(id);
    // 原地过滤而非 fetchAll，减少一次网络往返；后端已保证删除成功
    repositories.value = repositories.value.filter((r) => r.id !== id);
  }

  /** 刷新单仓库状态，原地替换列表中对应项。 */
  async function refreshOne(id: string): Promise<void> {
    refreshing.value = true;
    try {
      const updated = await localRepositoryApi.refreshOne(id);
      // 用 findIndex 而非 filter+push，保持列表原顺序
      const idx = repositories.value.findIndex((r) => r.id === id);
      if (idx >= 0) repositories.value[idx] = updated;
    } finally {
      refreshing.value = false;
    }
  }

  /** 刷新全部仓库状态（后端顺序执行，避免大量 git 子进程）。 */
  async function refreshAll(): Promise<void> {
    refreshing.value = true;
    try {
      repositories.value = await localRepositoryApi.refreshAll();
    } finally {
      refreshing.value = false;
    }
  }

  /** 批量 Fetch 选中仓库；汇总写入 lastFetchSummary 供 UI 显示。 */
  async function batchFetch(ids: string[]): Promise<BatchFetchSummary> {
    fetching.value = true;
    error.value = null;
    try {
      const summary = await localRepositoryApi.batchFetch(ids);
      lastFetchSummary.value = summary;
      // Fetch 后远端状态可能变化（ahead/behind），同步刷新一次状态
      await refreshAll();
      return summary;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      fetching.value = false;
    }
  }

  /** 在系统文件管理器中打开仓库目录。 */
  async function openFolder(id: string): Promise<void> {
    await localRepositoryApi.openFolder(id);
  }

  /** 在系统终端中打开仓库目录。 */
  async function openTerminal(id: string): Promise<void> {
    await localRepositoryApi.openTerminal(id);
  }

  return {
    repositories,
    scanning,
    refreshing,
    fetching,
    error,
    lastFetchSummary,
    fetchAll,
    addByPath,
    scanRoot,
    removeById,
    refreshOne,
    refreshAll,
    batchFetch,
    openFolder,
    openTerminal,
  };
});
