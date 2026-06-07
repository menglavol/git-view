<template>
  <ElDrawer
    v-model="visible"
    direction="rtl"
    :size="drawerSize"
    :title="repo?.fullName || '仓库详情'"
  >
    <div v-if="repo" class="repo-detail">
      <!-- 基本信息 / 提交历史 两 Tab -->
      <ElTabs v-model="activeTab" class="detail-tabs">
        <!-- ===================== 基本信息 ===================== -->
        <ElTabPane label="基本信息" name="info">
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
        </ElTabPane>

        <!-- ===================== 提交历史 ===================== -->
        <ElTabPane label="提交历史" name="history">
          <!-- 选中提交后展示详情面板（带返回），否则展示提交列表 -->
          <div v-if="selectedCommitSha" class="commit-detail-wrap">
            <ElButton text size="small" class="back-btn" @click="backToCommitList">
              ← 返回提交列表
            </ElButton>
            <CommitDetailPanel :detail="commitDetail" :loading="commitDetailLoading" />
          </div>
          <CommitHistory
            v-else
            :key="repo.id"
            :load-page="loadRemoteCommits"
            @select="onSelectCommit"
          />
        </ElTabPane>
      </ElTabs>
    </div>
  </ElDrawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { open as openExternal } from '@tauri-apps/plugin-shell';

import CommitHistory from '@/components/git/CommitHistory.vue';
import CommitDetailPanel from '@/components/git/CommitDetailPanel.vue';
import { remoteRepositoryApi } from '@/api/remoteRepository.api';
import type { RemoteRepository, Visibility } from '@/types/repository';
import type { GitPlatform } from '@/types/account';
import type { CommitDetail, CommitSummary } from '@/types/git';

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

// 当前 Tab：基本信息 / 提交历史
const activeTab = ref('info');
// 提交历史中选中的提交 sha（null 显示列表，否则显示详情面板）
const selectedCommitSha = ref<string | null>(null);
// 选中提交的详情
const commitDetail = ref<CommitDetail | null>(null);
// 提交详情加载中
const commitDetailLoading = ref(false);

// 抽屉宽度：查看提交详情（含 diff）时加宽，便于阅读
const drawerSize = computed(() => (selectedCommitSha.value ? '760px' : '480px'));

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

/** 远程提交分页加载：CommitHistory 的 page 从 0 起，远程 API 从 1 起，故 +1。 */
async function loadRemoteCommits(page: number, pageSize: number): Promise<CommitSummary[]> {
  if (!props.repo) return [];
  const result = await remoteRepositoryApi.listCommits(props.repo.id, page + 1, pageSize);
  // hasNext 由 CommitHistory 用「返回数量 < pageSize」判断，这里只回传列表
  return result.items;
}

/** 点击提交行：记录选中并加载其详情。 */
async function onSelectCommit(sha: string): Promise<void> {
  if (!props.repo) return;
  selectedCommitSha.value = sha;
  commitDetailLoading.value = true;
  commitDetail.value = null;
  try {
    commitDetail.value = await remoteRepositoryApi.getCommitDetail(props.repo.id, sha);
  } catch (e) {
    ElMessage.error(`提交详情加载失败：${e instanceof Error ? e.message : String(e)}`);
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

// 切换仓库时重置 Tab 与提交详情选中态，避免串台
watch(
  () => props.repo?.id,
  () => {
    activeTab.value = 'info';
    selectedCommitSha.value = null;
    commitDetail.value = null;
  },
);
</script>

<style scoped>
.repo-detail {
  padding: 0 8px;
  height: 100%;
}

/* Tab 容器铺满抽屉高度，内容区允许内部滚动 */
.detail-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.detail-tabs :deep(.el-tabs__content) {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.detail-tabs :deep(.el-tab-pane) {
  height: 100%;
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

/* 提交详情视图：返回按钮 + 详情面板纵向铺满并自滚动 */
.commit-detail-wrap {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.commit-detail-wrap .back-btn {
  align-self: flex-start;
  margin-bottom: 4px;
}
.commit-detail-wrap :deep(.commit-detail) {
  flex: 1;
  min-height: 0;
}
</style>
