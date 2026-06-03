// =====================================================================
// 操作日志 store（T097 / US6）
// =====================================================================
// 职责：作为 Logs 页面的状态中枢，封装：
//   1) 筛选条件（操作类型 / 状态 / 关键字）与分页（page / pageSize）
//   2) 当前页日志列表 / 加载状态 / 错误的响应式管理
//   3) 对 logsApi 的 action 包装（带 loading 切换、错误归一化）
// 设计原则（与 localRepository store 一致）：
//   - 组件不直接调底层 API，统一经由 store action
//   - actions 在 try/finally 中切换 loading 标志，避免 UI 挂起
//   - 后端 list 不返回总数：以「当前页是否满页」推断是否还有下一页
// =====================================================================

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { logsApi } from '@/api/logs.api';
import type { LogFilter, OperationLog, OperationStatus, OperationType } from '@/types/operationLog';

export const useOperationLogStore = defineStore('operationLog', () => {
  // -------------------------------------------------------------------
  // 筛选条件：均为独立 ref，便于模板直接 v-model 绑定
  // -------------------------------------------------------------------
  /** 操作类型筛选（空数组表示不限类型） */
  const operationTypes = ref<OperationType[]>([]);
  /** 结果状态筛选（空数组表示不限状态） */
  const statuses = ref<OperationStatus[]>([]);
  /** 关键字（模糊匹配 target；空串表示不过滤） */
  const keyword = ref('');

  // -------------------------------------------------------------------
  // 分页与列表状态
  // -------------------------------------------------------------------
  /** 当前页码（从 0 开始） */
  const page = ref(0);
  /** 每页条数（与后端默认一致） */
  const pageSize = ref(50);
  /** 当前页日志列表（与后端 SELECT 结果对齐） */
  const logs = ref<OperationLog[]>([]);
  /** 列表查询进行中：表格 loading 遮罩 */
  const loading = ref(false);
  /** 清理日志进行中：禁用「清理」按钮，避免重复提交 */
  const clearing = ref(false);
  /** 最近一次错误（已脱敏）：UI 顶部提示可订阅 */
  const error = ref<string | null>(null);

  /** 是否存在上一页：仅当不在首页时为真。 */
  const hasPrevPage = computed(() => page.value > 0);
  /** 是否可能存在下一页：当前页满页时推断仍有更多（后端不返回总数）。 */
  const hasNextPage = computed(() => logs.value.length === pageSize.value);

  /** 把当前筛选条件组装为后端 LogFilter（keyword 空串归一化为 undefined）。 */
  function buildFilter(): LogFilter {
    return {
      operationTypes: operationTypes.value,
      statuses: statuses.value,
      keyword: keyword.value.trim() === '' ? undefined : keyword.value.trim(),
      page: page.value,
      pageSize: pageSize.value,
    };
  }

  /** 按当前筛选 + 分页查询日志，覆盖 logs。 */
  async function fetchList(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      logs.value = await logsApi.list(buildFilter());
    } catch (e) {
      // 错误归一化为字符串以便 UI 直接渲染
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** 应用筛选条件变化：回到第一页再查询（避免停留在越界页码）。 */
  async function applyFilters(): Promise<void> {
    page.value = 0;
    await fetchList();
  }

  /** 重置全部筛选并回到第一页。 */
  async function resetFilters(): Promise<void> {
    operationTypes.value = [];
    statuses.value = [];
    keyword.value = '';
    page.value = 0;
    await fetchList();
  }

  /** 上一页（已在首页时为空操作）。 */
  async function prevPage(): Promise<void> {
    if (!hasPrevPage.value) return;
    page.value -= 1;
    await fetchList();
  }

  /** 下一页（无更多数据时为空操作）。 */
  async function nextPage(): Promise<void> {
    if (!hasNextPage.value) return;
    page.value += 1;
    await fetchList();
  }

  /**
   * 清理日志（前端 UI 须先做二次确认）。
   *
   * @param beforeDays 缺省清空全部；传 n 删除 n 天前的日志。
   * @returns 删除的行数。
   */
  async function clearOld(beforeDays?: number): Promise<number> {
    clearing.value = true;
    error.value = null;
    try {
      const removed = await logsApi.clearOld(beforeDays);
      // 清理后回到第一页重新拉取，保证列表与库内一致
      page.value = 0;
      await fetchList();
      return removed;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      clearing.value = false;
    }
  }

  /** 查询单条日志详情（用于详情对话框，确保拿到最新数据）。 */
  async function getDetail(id: string): Promise<OperationLog | null> {
    return logsApi.getDetail(id);
  }

  return {
    operationTypes,
    statuses,
    keyword,
    page,
    pageSize,
    logs,
    loading,
    clearing,
    error,
    hasPrevPage,
    hasNextPage,
    fetchList,
    applyFilters,
    resetFilters,
    prevPage,
    nextPage,
    clearOld,
    getDetail,
  };
});
