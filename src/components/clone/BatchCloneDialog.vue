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
        <div class="preview-title">目标路径预览</div>
        <ul class="preview-list">
          <li v-for="p in previews" :key="p.id">
            <span class="repo">{{ p.fullName }}</span>
            <span class="arrow">→</span>
            <code class="path">{{ p.path }}</code>
          </li>
        </ul>
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

const canSubmit = computed(
  () => targetRoot.value.trim().length > 0 && props.selectedRepos.length > 0,
);

const previews = computed(() => {
  const root = targetRoot.value.replace(/\/+$/, '') || '<root>';
  return props.selectedRepos.slice(0, 5).map((r) => ({
    id: r.id,
    fullName: r.fullName,
    path: computePreviewPath(root, r, directoryStrategy.value),
  }));
});

watch(
  () => props.modelValue,
  (v) => {
    if (v) {
      submitting.value = false;
    }
  },
);

function computePreviewPath(
  root: string,
  repo: RemoteRepository,
  strategy: DirectoryStrategy,
): string {
  switch (strategy) {
    case 'flat':
      return `${root}/${repo.name}`;
    case 'by_owner':
      return `${root}/${repo.owner}/${repo.name}`;
    case 'by_platform_and_owner':
      return `${root}/${repo.platform}/${repo.owner}/${repo.name}`;
  }
}

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
  list-style: none;
  padding: 0;
  margin: 0;
  font-size: 12px;
}

.preview-list li {
  display: flex;
  gap: 8px;
  align-items: baseline;
  padding: 2px 0;
}

.repo {
  color: var(--el-color-primary);
}

.arrow {
  color: var(--el-text-color-secondary);
}

.path {
  font-family: var(--el-font-family-monospace, monospace);
  color: var(--el-text-color-regular);
  word-break: break-all;
}
</style>
