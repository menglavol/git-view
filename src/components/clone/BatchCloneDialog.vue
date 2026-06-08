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
            <code class="path-prefix">{{ prefixFor(r) }}</code>
            <ElInput
              v-model="dirNames[r.id]"
              size="small"
              class="dir-input"
              :class="{ invalid: !isValidDirName(dirNames[r.id] ?? '') }"
              placeholder="目录名"
            />
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
const directoryStrategy = ref<DirectoryStrategy>('by_platform_and_owner');
const concurrency = ref(3);
const autoAddToLocal = ref(true);
const submitting = ref(false);
// 每个仓库的自定义目录名（key = repo.id），dialog 打开时初始化为仓库名
const dirNames = ref<Record<string, string>>({});

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
      for (const r of props.selectedRepos) init[r.id] = r.name;
      dirNames.value = init;
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
    await cloneStore.createAndStart({
      remoteRepositoryIds: props.selectedRepos.map((r) => r.id),
      targetRoot: targetRoot.value.trim(),
      directoryStrategy: directoryStrategy.value,
      concurrency: concurrency.value,
      autoAddToLocal: autoAddToLocal.value,
      // 每仓库自定义目录名；后端再做一次 sanitize 防穿越
      dirNameOverrides: { ...dirNames.value },
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
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.arrow {
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.path-prefix {
  font-family: var(--el-font-family-monospace, monospace);
  color: var(--el-text-color-secondary);
  word-break: break-all;
}

.dir-input {
  width: 160px;
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
