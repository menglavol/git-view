<template>
  <ElTable
    v-loading="loading"
    :data="items"
    style="width: 100%"
    row-key="id"
    :empty-text="emptyText"
    @selection-change="onSelectionChange"
  >
    <ElTableColumn type="selection" width="44" />

    <ElTableColumn label="平台" width="80">
      <template #default="{ row }">
        <ElTag :type="platformTag(row.platform)" size="small">
          {{ platformLabel(row.platform) }}
        </ElTag>
      </template>
    </ElTableColumn>

    <ElTableColumn label="仓库" min-width="240">
      <template #default="{ row }">
        <div class="repo-cell">
          <span class="repo-name" @click="emit('open-detail', row)">
            {{ row.fullName }}
          </span>
          <span v-if="row.description" class="repo-desc">
            {{ row.description }}
          </span>
        </div>
      </template>
    </ElTableColumn>

    <ElTableColumn label="可见性" width="80">
      <template #default="{ row }">
        <ElTag :type="visTag(row.visibility)" size="small" effect="plain">
          {{ visLabel(row.visibility) }}
        </ElTag>
      </template>
    </ElTableColumn>

    <ElTableColumn label="默认分支" width="120">
      <template #default="{ row }">
        {{ row.defaultBranch }}
      </template>
    </ElTableColumn>

    <ElTableColumn label="最近推送" width="160">
      <template #default="{ row }">
        {{ formatTime(row.lastPushedAt) }}
      </template>
    </ElTableColumn>

    <ElTableColumn label="操作" width="240" fixed="right">
      <template #default="{ row }">
        <ElButtonGroup>
          <ElButton size="small" @click="emit('clone', row)"> Clone </ElButton>
          <ElButton size="small" @click="emit('copy-url', { repo: row, type: 'https' })">
            复制
          </ElButton>
          <ElButton
            size="small"
            :type="row.isFavorite ? 'warning' : 'default'"
            @click="emit('toggle-favorite', row)"
          >
            {{ row.isFavorite ? '已收藏' : '收藏' }}
          </ElButton>
        </ElButtonGroup>
      </template>
    </ElTableColumn>
  </ElTable>
</template>

<script setup lang="ts">
import type { RemoteRepository, Visibility } from '@/types/repository';
import type { GitPlatform } from '@/types/account';

defineProps<{
  items: RemoteRepository[];
  loading?: boolean;
  emptyText?: string;
}>();

const emit = defineEmits<{
  (e: 'update:selection', repos: RemoteRepository[]): void;
  (e: 'open-detail', repo: RemoteRepository): void;
  (e: 'clone', repo: RemoteRepository): void;
  (e: 'toggle-favorite', repo: RemoteRepository): void;
  (e: 'copy-url', payload: { repo: RemoteRepository; type: 'https' | 'ssh' }): void;
}>();

function onSelectionChange(rows: RemoteRepository[]): void {
  emit('update:selection', rows);
}

function platformLabel(p: GitPlatform): string {
  return p === 'github' ? 'GitHub' : p === 'gitlab' ? 'GitLab' : 'Gitee';
}

function platformTag(p: GitPlatform): 'info' | 'warning' | 'danger' {
  return p === 'github' ? 'info' : p === 'gitlab' ? 'warning' : 'danger';
}

function visLabel(v: Visibility): string {
  return v === 'public' ? '公开' : v === 'internal' ? '内部' : '私有';
}

function visTag(v: Visibility): 'success' | 'info' | 'warning' {
  return v === 'public' ? 'success' : v === 'internal' ? 'info' : 'warning';
}

function formatTime(iso?: string): string {
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleDateString();
  } catch {
    return iso;
  }
}
</script>

<style scoped>
.repo-cell {
  display: flex;
  flex-direction: column;
}

.repo-name {
  color: var(--el-color-primary);
  cursor: pointer;
  font-weight: 500;
}

.repo-name:hover {
  text-decoration: underline;
}

.repo-desc {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  margin-top: 2px;
}
</style>
