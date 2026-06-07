<template>
  <div class="page-clone-center">
    <div class="page-header">
      <h2 class="page-title">Clone 中心</h2>
      <div class="header-actions">
        <ElButton :disabled="counts.completed === 0" @click="onClear('completed')">
          清空已完成
        </ElButton>
        <ElButton :disabled="counts.failed === 0" @click="onClear('failed')">清空已失败</ElButton>
        <ElButton :disabled="counts.cancelled === 0" @click="onClear('cancelled')">
          清空已取消
        </ElButton>
        <ElButton @click="onRefresh">刷新</ElButton>
      </div>
    </div>

    <div class="stats">
      <ElCard shadow="never" class="stat-card">
        <ElStatistic title="总任务数" :value="store.tasks.length" />
      </ElCard>
      <ElCard shadow="never" class="stat-card">
        <ElStatistic title="进行中" :value="counts.running" />
      </ElCard>
      <ElCard shadow="never" class="stat-card">
        <ElStatistic title="已完成" :value="counts.completed" />
      </ElCard>
      <ElCard shadow="never" class="stat-card">
        <ElStatistic title="失败" :value="counts.failed" />
      </ElCard>
      <ElCard shadow="never" class="stat-card">
        <ElStatistic title="已取消" :value="counts.cancelled" />
      </ElCard>
    </div>

    <ElTable
      v-loading="store.loading"
      :data="store.tasks"
      style="width: 100%"
      empty-text="暂无克隆任务"
      row-key="id"
    >
      <ElTableColumn label="仓库" min-width="200">
        <template #default="{ row }">
          <span class="repo-name">{{ row.repositoryName }}</span>
        </template>
      </ElTableColumn>

      <ElTableColumn label="状态" width="100">
        <template #default="{ row }">
          <ElTag :type="statusTag(row.status)" size="small">
            {{ statusLabel(row.status) }}
          </ElTag>
        </template>
      </ElTableColumn>

      <ElTableColumn label="进度" min-width="200">
        <template #default="{ row }">
          <ElProgress
            :percentage="row.progress"
            :status="progressStatus(row.status)"
            :stroke-width="10"
          />
        </template>
      </ElTableColumn>

      <ElTableColumn label="目标路径" min-width="280" show-overflow-tooltip>
        <template #default="{ row }">
          <code class="path">{{ row.targetPath }}</code>
        </template>
      </ElTableColumn>

      <ElTableColumn label="错误" min-width="160">
        <template #default="{ row }">
          <ElTooltip v-if="row.errorMessage" :content="row.errorMessage" placement="top">
            <span class="error-text">{{ row.errorMessage }}</span>
          </ElTooltip>
          <span v-else>—</span>
        </template>
      </ElTableColumn>

      <ElTableColumn label="操作" width="180" fixed="right">
        <template #default="{ row }">
          <ElButtonGroup>
            <ElButton
              v-if="row.status === 'running'"
              size="small"
              type="warning"
              @click="onCancel(row)"
            >
              取消
            </ElButton>
            <ElButton
              v-if="row.status === 'failed' || row.status === 'cancelled'"
              size="small"
              @click="onRetry(row)"
            >
              重试
            </ElButton>
          </ElButtonGroup>
        </template>
      </ElTableColumn>
    </ElTable>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';

import { useCloneTaskStore } from '@/stores/cloneTask';
import type { CloneTask, CloneTaskStatus } from '@/types/cloneTask';

const store = useCloneTaskStore();

const counts = computed(() => ({
  running: store.tasks.filter((t) => t.status === 'running').length,
  completed: store.tasks.filter((t) => t.status === 'completed').length,
  failed: store.tasks.filter((t) => t.status === 'failed').length,
  cancelled: store.tasks.filter((t) => t.status === 'cancelled').length,
}));

onMounted(async () => {
  try {
    await store.fetchAll();
    await store.startListening();
  } catch (e) {
    ElMessage.error(`加载任务失败：${e instanceof Error ? e.message : String(e)}`);
  }
});

onUnmounted(() => {
  store.stopListening();
});

async function onRefresh(): Promise<void> {
  await store.fetchAll().catch((e) => {
    ElMessage.error(`刷新失败：${e instanceof Error ? e.message : String(e)}`);
  });
}

// 三种可清理的终态文案，用于按钮 / 确认框 / 成功提示
const CLEARABLE_TEXT: Record<'completed' | 'failed' | 'cancelled', string> = {
  completed: '已完成',
  failed: '已失败',
  cancelled: '已取消',
};

async function onClear(status: 'completed' | 'failed' | 'cancelled'): Promise<void> {
  const n = counts.value[status];
  const text = CLEARABLE_TEXT[status];
  // 按钮在该状态计数为 0 时已 disable，这里兜底
  if (n === 0) return;
  try {
    // 删除类操作二次确认（宪法 Principle III）；强调只删任务记录、不动磁盘文件
    await ElMessageBox.confirm(
      `确认清空全部「${text}」的任务记录吗？共 ${n} 条。\n\n注：仅删除任务记录，不影响已克隆到磁盘的文件。`,
      `清空${text}任务`,
      { confirmButtonText: '确认清空', cancelButtonText: '取消', type: 'warning' },
    );
  } catch {
    // 用户取消：ElMessageBox reject，静默返回
    return;
  }
  try {
    const cleared = await store.clearByStatus(status);
    ElMessage.success(`已清理 ${cleared} 条${text}任务`);
  } catch (e) {
    ElMessage.error(`清理失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

async function onCancel(row: CloneTask): Promise<void> {
  try {
    await store.cancel(row.id);
    ElMessage.success('已发送取消信号');
  } catch (e) {
    ElMessage.error(`取消失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

async function onRetry(row: CloneTask): Promise<void> {
  try {
    await store.retry(row.id, true);
    ElMessage.success('已重新入队');
  } catch (e) {
    ElMessage.error(`重试失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

function statusLabel(s: CloneTaskStatus): string {
  return {
    pending: '排队',
    running: '进行中',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消',
  }[s];
}

function statusTag(s: CloneTaskStatus): 'info' | 'success' | 'danger' | 'warning' | 'primary' {
  return {
    pending: 'info' as const,
    running: 'primary' as const,
    completed: 'success' as const,
    failed: 'danger' as const,
    cancelled: 'warning' as const,
  }[s];
}

function progressStatus(s: CloneTaskStatus): 'success' | 'warning' | 'exception' | undefined {
  switch (s) {
    case 'completed':
      return 'success';
    case 'failed':
      return 'exception';
    case 'cancelled':
      return 'warning';
    default:
      return undefined;
  }
}
</script>

<style scoped>
.page-clone-center {
  padding: 8px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.page-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 12px;
  margin-bottom: 16px;
}

.stat-card {
  text-align: center;
}

.repo-name {
  font-weight: 500;
  color: var(--el-color-primary);
}

.path {
  font-family: var(--el-font-family-monospace, monospace);
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.error-text {
  color: var(--el-color-danger);
  font-size: 12px;
  display: inline-block;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
