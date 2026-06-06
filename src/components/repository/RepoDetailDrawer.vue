<template>
  <ElDrawer v-model="visible" direction="rtl" size="480px" :title="repo?.fullName || '仓库详情'">
    <div v-if="repo" class="repo-detail">
      <div class="section">
        <div class="section-title">基本信息</div>
        <div class="field">
          <span class="label">平台</span>
          <ElTag :type="platformTag(repo.platform)" size="small">
            {{ platformLabel(repo.platform) }}
          </ElTag>
        </div>
        <div class="field">
          <span class="label">所有者</span>
          <span>{{ repo.owner }}</span>
        </div>
        <div class="field">
          <span class="label">默认分支</span>
          <span>{{ repo.defaultBranch }}</span>
        </div>
        <div class="field">
          <span class="label">可见性</span>
          <ElTag :type="visTag(repo.visibility)" size="small" effect="plain">
            {{ visLabel(repo.visibility) }}
          </ElTag>
        </div>
        <div v-if="repo.description" class="field">
          <span class="label">描述</span>
          <span class="desc">{{ repo.description }}</span>
        </div>
        <div class="field">
          <span class="label">最近推送</span>
          <span>{{ formatTime(repo.lastPushedAt) }}</span>
        </div>
        <div class="field">
          <span class="label">同步时间</span>
          <span>{{ formatTime(repo.syncedAt) }}</span>
        </div>
      </div>

      <div class="section">
        <div class="section-title">克隆地址</div>
        <div class="field">
          <span class="label">HTTPS</span>
          <code class="url">{{ repo.cloneUrl }}</code>
          <ElButton size="small" @click="onCopy(repo.cloneUrl)">复制</ElButton>
        </div>
        <div v-if="repo.sshUrl" class="field">
          <span class="label">SSH</span>
          <code class="url">{{ repo.sshUrl }}</code>
          <ElButton size="small" @click="onCopy(repo.sshUrl)">复制</ElButton>
        </div>
      </div>

      <div class="section actions">
        <ElButton type="primary" @click="emit('clone', repo)">Clone 到本地</ElButton>
        <ElButton @click="onOpenWeb(repo.htmlUrl)">打开网页</ElButton>
        <ElButton
          :type="repo.isFavorite ? 'warning' : 'default'"
          @click="emit('toggle-favorite', repo)"
        >
          {{ repo.isFavorite ? '取消收藏' : '收藏' }}
        </ElButton>
      </div>
    </div>
  </ElDrawer>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { ElMessage } from 'element-plus';
import { open as openExternal } from '@tauri-apps/plugin-shell';

import type { RemoteRepository, Visibility } from '@/types/repository';
import type { GitPlatform } from '@/types/account';

const props = defineProps<{
  modelValue: boolean;
  repo: RemoteRepository | null;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void;
  (e: 'clone', repo: RemoteRepository): void;
  (e: 'toggle-favorite', repo: RemoteRepository): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
});

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
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function onCopy(url: string): void {
  void navigator.clipboard.writeText(url).then(() => {
    ElMessage.success('已复制');
  });
}

function onOpenWeb(url: string): void {
  // htmlUrl 理论上必有；为空时给明确提示而非静默失败
  if (!url) {
    ElMessage.warning('该仓库没有可打开的网页地址');
    return;
  }
  // 用系统默认浏览器打开：Tauri webview 不支持 window.open 打开外链，
  // 必须走 shell 插件的 open（capabilities 已授权 shell:allow-open）
  void openExternal(url).catch((e) => {
    ElMessage.error(`打开网页失败：${e instanceof Error ? e.message : String(e)}`);
  });
}
</script>

<style scoped>
.repo-detail {
  padding: 0 8px;
}

.section {
  margin-bottom: 24px;
}

.section-title {
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.field {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
  font-size: 13px;
}

.label {
  width: 80px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.desc {
  color: var(--el-text-color-regular);
  line-height: 1.6;
}

.url {
  flex: 1;
  font-family: var(--el-font-family-monospace, monospace);
  font-size: 12px;
  background: var(--el-fill-color-light);
  padding: 4px 8px;
  border-radius: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
</style>
