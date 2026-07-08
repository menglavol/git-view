<template>
  <ElDialog v-model="visible" title="批量克隆配置" width="640px" :close-on-click-modal="false">
    <ElForm label-width="120px" label-position="left">
      <ElFormItem label="选中仓库">
        <span>{{ selectedRepos.length }} 个</span>
      </ElFormItem>

      <ElFormItem label="目标根目录" required>
        <div class="dir-row">
          <ElInput v-model="targetRoot" placeholder="请选择目录" />
          <ElButton @click="onPickDir">选择...</ElButton>
        </div>
      </ElFormItem>

      <ElFormItem label="目录组织方式">
        <ElRadioGroup v-model="directoryStrategy">
          <ElRadio value="flat">扁平 (root/repo)</ElRadio>
          <ElRadio value="by_owner">按所有者 (root/owner/repo)</ElRadio>
          <ElRadio value="by_platform_and_owner">
            按平台与所有者 (root/platform/owner/repo)
          </ElRadio>
        </ElRadioGroup>
      </ElFormItem>

      <ElFormItem label="并发数">
        <ElSlider
          v-model="concurrency"
          :min="1"
          :max="8"
          :step="1"
          show-input
          style="max-width: 360px"
        />
      </ElFormItem>

      <ElFormItem label="完成后自动加入本地仓库">
        <ElSwitch v-model="autoAddToLocal" />
      </ElFormItem>

      <ElDivider />

      <div class="preview">
        <div class="preview-title">目标路径（目录名可编辑）</div>
        <div class="preview-list">
          <div v-for="r in selectedRepos" :key="r.id" class="preview-row">
            <span class="repo" :title="r.fullName">{{ r.fullName }}</span>
            <span class="arrow">→</span>
            <code class="path-prefix" :title="prefixFor(r)">{{ prefixFor(r) }}</code>
            <ElInput
              v-model="dirNames[r.id]"
              size="small"
              class="dir-input"
              :class="{ invalid: !isValidDirName(dirNames[r.id] ?? '') }"
              placeholder="目录名"
            />
            <!-- 分支下拉：可搜索，展开时惰性拉取平台分支列表；默认选仓库默认分支 -->
            <ElSelect
              v-model="branchSel[r.id]"
              size="small"
              class="branch-select"
              filterable
              :loading="branchLoading[r.id]"
              :placeholder="r.defaultBranch || '默认分支'"
              @visible-change="(open: boolean) => open && loadBranches(r)"
            >
              <ElOption
                v-for="b in branchOptions[r.id] ?? [r.defaultBranch]"
                :key="b"
                :label="b"
                :value="b"
              />
            </ElSelect>
          </div>
        </div>
        <div v-if="!allDirNamesValid" class="preview-hint">
          目录名不能为空，且不能包含 / 或 \ 等路径分隔符
        </div>
      </div>
    </ElForm>

    <template #footer>
      <ElButton @click="visible = false">取消</ElButton>
      <ElButton type="primary" :loading="submitting" :disabled="!canSubmit" @click="onConfirm">
        开始克隆
      </ElButton>
    </template>
  </ElDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

import { useCloneTaskStore } from '@/stores/cloneTask';
import { remoteRepositoryApi } from '@/api/remoteRepository.api';
import type { RemoteRepository } from '@/types/repository';
import type { DirectoryStrategy } from '@/types/settings';

const props = defineProps<{
  modelValue: boolean;
  selectedRepos: RemoteRepository[];
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void;
  (e: 'started'): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
});

const cloneStore = useCloneTaskStore();

const targetRoot = ref('');
// 默认目录组织方式为「扁平」(root/repo)：多数用户克到单一工作目录，层级最浅最直观
const directoryStrategy = ref<DirectoryStrategy>('flat');
const concurrency = ref(3);
const autoAddToLocal = ref(true);
const submitting = ref(false);
// 每个仓库的自定义目录名（key = repo.id），dialog 打开时初始化为仓库名
const dirNames = ref<Record<string, string>>({});
// 每个仓库当前选中的克隆分支（key = repo.id），默认取仓库默认分支
const branchSel = ref<Record<string, string>>({});
// 每个仓库已拉取的分支列表（key = repo.id）；未拉取时下拉回退到 [defaultBranch]
const branchOptions = ref<Record<string, string[]>>({});
// 每个仓库分支列表的加载态，避免重复请求与展示 loading
const branchLoading = ref<Record<string, boolean>>({});
// 已成功拉取过分支列表的仓库集合：避免每次展开都重复请求平台 API
const branchLoaded = ref<Record<string, boolean>>({});

/**
 * 惰性拉取某仓库的分支列表（下拉展开时触发）。
 *
 * 已加载过或正在加载则跳过；失败时静默回退到仅默认分支（不阻断克隆），
 * 保证即使平台分支接口不可用，用户仍能按默认分支克隆。
 */
async function loadBranches(repo: RemoteRepository): Promise<void> {
  if (branchLoaded.value[repo.id] || branchLoading.value[repo.id]) return;
  branchLoading.value[repo.id] = true;
  try {
    const list = await remoteRepositoryApi.listBranches(repo.id);
    // 平台返回空列表时回退默认分支，避免下拉无任何选项
    branchOptions.value[repo.id] = list.length > 0 ? list : [repo.defaultBranch];
    branchLoaded.value[repo.id] = true;
  } catch {
    // 失败回退：仅保留默认分支，且不标记已加载以便用户重试
    branchOptions.value[repo.id] = [repo.defaultBranch];
  } finally {
    branchLoading.value[repo.id] = false;
  }
}

/** 目录名合法性：非空、非 . / ..、不含路径分隔符（与后端 sanitize_dir_name 对齐）。 */
function isValidDirName(name: string): boolean {
  const t = name.trim();
  return t.length > 0 && t !== '.' && t !== '..' && !t.includes('/') && !t.includes('\\');
}

// 所有目录名均合法时才允许提交
const allDirNamesValid = computed(() =>
  props.selectedRepos.every((r) => isValidDirName(dirNames.value[r.id] ?? '')),
);

const canSubmit = computed(
  () =>
    targetRoot.value.trim().length > 0 && props.selectedRepos.length > 0 && allDirNamesValid.value,
);

/** 计算某仓库目标路径的前缀（不含末段目录名），随根目录与组织方式变化。 */
function prefixFor(repo: RemoteRepository): string {
  const root = targetRoot.value.replace(/[/\\]+$/, '') || '<root>';
  switch (directoryStrategy.value) {
    case 'flat':
      return `${root}/`;
    case 'by_owner':
      return `${root}/${repo.owner}/`;
    case 'by_platform_and_owner':
      return `${root}/${repo.platform}/${repo.owner}/`;
  }
}

watch(
  () => props.modelValue,
  (v) => {
    if (v) {
      submitting.value = false;
      // 重置目录名为各仓库的默认名（仓库名）
      const init: Record<string, string> = {};
      // 分支选择默认取各仓库的默认分支；分支列表与加载态一并清空，下次展开重新拉取
      const branchInit: Record<string, string> = {};
      for (const r of props.selectedRepos) {
        init[r.id] = r.name;
        branchInit[r.id] = r.defaultBranch;
      }
      dirNames.value = init;
      branchSel.value = branchInit;
      branchOptions.value = {};
      branchLoading.value = {};
      branchLoaded.value = {};
    }
  },
);

async function onPickDir(): Promise<void> {
  try {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === 'string') {
      targetRoot.value = selected;
    }
  } catch (e) {
    ElMessage.error(`选择目录失败：${e instanceof Error ? e.message : String(e)}`);
  }
}

async function onConfirm(): Promise<void> {
  if (!canSubmit.value) return;
  submitting.value = true;
  try {
    // 仅收集「显式选了且与默认分支不同」的项：与默认分支相同则不传，
    // 后端 NULL 语义即克隆默认分支，减少无谓传参。
    const branches: Record<string, string> = {};
    for (const r of props.selectedRepos) {
      const sel = branchSel.value[r.id];
      if (sel && sel !== r.defaultBranch) branches[r.id] = sel;
    }
    await cloneStore.createAndStart({
      remoteRepositoryIds: props.selectedRepos.map((r) => r.id),
      targetRoot: targetRoot.value.trim(),
      directoryStrategy: directoryStrategy.value,
      concurrency: concurrency.value,
      autoAddToLocal: autoAddToLocal.value,
      // 每仓库自定义目录名；后端再做一次 sanitize 防穿越
      dirNameOverrides: { ...dirNames.value },
      // 每仓库要克隆的分支（仅非默认项）；空对象后端按默认分支处理
      branches,
    });
    ElMessage.success(`已创建 ${props.selectedRepos.length} 个克隆任务`);
    visible.value = false;
    emit('started');
  } catch (e) {
    ElMessage.error(`创建任务失败：${e instanceof Error ? e.message : String(e)}`);
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.dir-row {
  display: flex;
  gap: 8px;
  width: 100%;
}

.preview {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
}

.preview-title {
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--el-text-color-primary);
}

.preview-list {
  font-size: 12px;
  max-height: 240px;
  overflow-y: auto;
}

.preview-row {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 3px 0;
}

.repo {
  color: var(--el-color-primary);
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.arrow {
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

/* 路径前缀是各行共享的根目录：单行省略号 + hover 看完整值，*/
/* 不再 break-all 挤成竖排窄条（父容器宽度紧张时旧写法会逐字换行）。*/
.path-prefix {
  font-family: var(--el-font-family-monospace, monospace);
  color: var(--el-text-color-secondary);
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

/* 目录名是唯一可编辑项：占据剩余弹性空间，保证输入区足够宽 */
.dir-input {
  flex: 1;
  min-width: 120px;
}

/* 分支下拉固定宽度，避免与目录名输入框互相挤压 */
.branch-select {
  width: 150px;
  flex-shrink: 0;
}

.dir-input.invalid :deep(.el-input__wrapper) {
  box-shadow: 0 0 0 1px var(--el-color-danger) inset;
}

.preview-hint {
  color: var(--el-color-danger);
  font-size: 12px;
  margin-top: 8px;
}
</style>
