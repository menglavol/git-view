// =====================================================================
// Clone 任务 store
// state：tasks / activeCount / totalProgress
// 订阅 clone-task-progress 与 clone-task-status-changed 事件实时更新
// =====================================================================

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';

import {
  cloneTaskApi,
  type CloneProgressPayload,
  type CloneStatusPayload,
  type CreateCloneTasksPayload,
} from '@/api/cloneTask.api';
import type { CloneTask, CloneTaskStatus } from '@/types/cloneTask';

export const useCloneTaskStore = defineStore('cloneTask', () => {
  const tasks = ref<CloneTask[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  let progressUnlisten: UnlistenFn | null = null;
  let statusUnlisten: UnlistenFn | null = null;

  const activeCount = computed(() => tasks.value.filter((t) => t.status === 'running').length);

  const totalProgress = computed(() => {
    if (tasks.value.length === 0) return 0;
    const sum = tasks.value.reduce((acc, t) => acc + t.progress, 0);
    return Math.floor(sum / tasks.value.length);
  });

  async function fetchAll(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      tasks.value = await cloneTaskApi.list();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function createAndStart(payload: CreateCloneTasksPayload): Promise<CloneTask[]> {
    const created = await cloneTaskApi.create(payload);
    tasks.value = [...created, ...tasks.value];
    await cloneTaskApi.start(
      created.map((t) => t.id),
      payload.autoAddToLocal,
    );
    return created;
  }

  async function cancel(taskId: string): Promise<void> {
    await cloneTaskApi.cancel(taskId);
  }

  async function retry(taskId: string, autoAddToLocal: boolean): Promise<void> {
    await cloneTaskApi.retry(taskId, autoAddToLocal);
    const t = tasks.value.find((x) => x.id === taskId);
    if (t) {
      t.status = 'pending';
      t.progress = 0;
      t.errorMessage = undefined;
    }
  }

  async function clearByStatus(status: CloneTaskStatus): Promise<number> {
    const n = await cloneTaskApi.clearByStatus(status);
    // 原地移除该状态的任务，避免再拉全量
    tasks.value = tasks.value.filter((t) => t.status !== status);
    return n;
  }

  async function startListening(): Promise<void> {
    if (progressUnlisten || statusUnlisten) return;
    progressUnlisten = await cloneTaskApi.onProgress(handleProgress);
    statusUnlisten = await cloneTaskApi.onStatusChanged(handleStatus);
  }

  function stopListening(): void {
    progressUnlisten?.();
    statusUnlisten?.();
    progressUnlisten = null;
    statusUnlisten = null;
  }

  function handleProgress(p: CloneProgressPayload): void {
    const t = tasks.value.find((x) => x.id === p.taskId);
    if (t) {
      t.progress = p.percent;
    }
  }

  function handleStatus(p: CloneStatusPayload): void {
    const t = tasks.value.find((x) => x.id === p.taskId);
    if (t) {
      t.status = p.status;
      t.progress = p.progress;
      t.errorMessage = p.errorMessage;
    }
  }

  return {
    tasks,
    loading,
    error,
    activeCount,
    totalProgress,
    fetchAll,
    createAndStart,
    cancel,
    retry,
    clearByStatus,
    startListening,
    stopListening,
  };
});
