<!--
  发布本地仓库到远程对话框。

  适用场景：本地仓库尚无 origin 远端。用户选一个已配置账户，在平台创建空仓库，
  后端随即 git remote add + push -u origin <当前分支> 并建立本地↔远程关联。

  字段：目标账户、命名空间（只读，取账户 username）、仓库名（基础合法校验）、
  描述、可见性（internal 仅 GitLab 有意义）、关联协议（默认取设置的默认 Clone 协议）。
-->

<template>
  <!-- 受控对话框：禁止点遮罩关闭，避免误触中断发布；@open 时初始化表单 -->
  <ElDialog
    v-model="visible"
    title="发布到远程"
    width="560px"
    :close-on-click-modal="false"
    @open="onOpen"
  >
    <!-- 表单：左侧标签布局，校验规则见下方 rules -->
    <ElForm ref="formRef" :model="form" :rules="rules" label-width="110px" label-position="left">
      <!-- 目标账户：决定发布到哪个平台、用谁的 token 建仓 -->
      <ElFormItem label="目标账户" prop="accountId">
        <ElSelect
          v-model="form.accountId"
          placeholder="选择要发布到的账户"
          style="width: 100%"
          @change="onAccountChange"
        >
          <!-- 仅列出启用的账户 -->
          <ElOption
            v-for="acc in enabledAccounts"
            :key="acc.id"
            :label="`${platformLabel(acc.platform)} · ${acc.username}`"
            :value="acc.id"
          />
        </ElSelect>
      </ElFormItem>

      <!-- 命名空间：首版仅个人 namespace，只读展示所选账户用户名 -->
      <ElFormItem label="命名空间">
        <span class="namespace">{{ selectedNamespace || '—' }}</span>
      </ElFormItem>

      <!-- 仓库名：默认取本地目录名，需通过合法字符校验 -->
      <ElFormItem label="仓库名" prop="name">
        <ElInput v-model="form.name" placeholder="仅限字母、数字、点、下划线、连字符" clearable />
      </ElFormItem>

      <!-- 描述：可选，最多 200 字 -->
      <ElFormItem label="描述">
        <ElInput
          v-model="form.description"
          type="textarea"
          :rows="2"
          maxlength="200"
          show-word-limit
          placeholder="可选"
        />
      </ElFormItem>

      <!-- 可见性：internal 仅 GitLab 有意义，故按平台动态显示 -->
      <ElFormItem label="可见性">
        <ElRadioGroup v-model="form.visibility">
          <ElRadioButton value="private">私有</ElRadioButton>
          <ElRadioButton value="public">公开</ElRadioButton>
          <!-- 「内部」仅对 GitLab 展示 -->
          <ElRadioButton v-if="selectedPlatform === 'gitlab'" value="internal">
            内部
          </ElRadioButton>
        </ElRadioGroup>
      </ElFormItem>

      <!-- 关联协议：默认取设置里的默认 Clone 协议 -->
      <ElFormItem label="关联协议">
        <ElRadioGroup v-model="form.protocol">
          <ElRadioButton value="https">HTTPS</ElRadioButton>
          <ElRadioButton value="ssh">SSH</ElRadioButton>
        </ElRadioGroup>
      </ElFormItem>

      <!-- 提示将要推送的当前分支 -->
      <ElAlert
        v-if="currentBranch"
        :title="`将推送当前分支：${currentBranch}`"
        type="info"
        :closable="false"
        show-icon
        class="branch-tip"
      />
    </ElForm>

    <!-- 底部操作：取消 / 发布 -->
    <template #footer>
      <ElButton @click="visible = false">取消</ElButton>
      <ElButton type="primary" :loading="publishing" :disabled="!canPublish" @click="onPublish">
        发布
      </ElButton>
    </template>
  </ElDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';

import { localRepositoryApi } from '@/api/localRepository.api';
import { GitViewClientError } from '@/api/tauri';
import { useAccountStore } from '@/stores/account';
import { useSettingsStore } from '@/stores/settings';
import type { GitPlatform } from '@/types/account';
import type { RemoteRepository, Visibility } from '@/types/repository';

// 组件 props：父组件传入目标本地仓库及其默认名 / 当前分支
const props = defineProps<{
  /** 受控显隐：v-model 绑定 */
  modelValue: boolean;
  /** 目标本地仓库 id */
  repoId: string;
  /** 默认仓库名（取本地目录名） */
  defaultName: string;
  /** 当前分支，用于「将推送分支」提示 */
  currentBranch?: string;
}>();

// 事件：v-model 双向 + 发布成功通知父组件刷新
const emit = defineEmits<{
  /** 更新 v-model 显隐 */
  (e: 'update:modelValue', val: boolean): void;
  /** 发布成功，回传新建的远程仓库 */
  (e: 'published', repo: RemoteRepository): void;
}>();

// 账户 store 供账户选择器，设置 store 供默认协议
const accountStore = useAccountStore();
const settingsStore = useSettingsStore();

// 对话框可见性：转发 modelValue
const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
});

// 表单字段类型
interface FormState {
  /** 目标账户 id */
  accountId: string;
  /** 远程仓库名 */
  name: string;
  /** 仓库描述（可空） */
  description: string;
  /** 可见性 */
  visibility: Visibility;
  /** 关联协议 */
  protocol: 'https' | 'ssh';
}

// 表单引用（用于触发 validate）
const formRef = ref<FormInstance>();
// 发布中：按钮 loading + 防重复提交
const publishing = ref(false);
// 表单数据
const form = reactive<FormState>({
  accountId: '',
  name: '',
  description: '',
  visibility: 'private',
  protocol: 'https',
});

// 仅启用的账户可作为发布目标
const enabledAccounts = computed(() => accountStore.accounts.filter((a) => a.enabled));

// 当前所选账户
const selectedAccount = computed(() => enabledAccounts.value.find((a) => a.id === form.accountId));
// 所选账户平台：决定是否展示 internal 可见性
const selectedPlatform = computed<GitPlatform | undefined>(() => selectedAccount.value?.platform);
// 命名空间：首版个人 namespace，取所选账户用户名供确认
const selectedNamespace = computed(() => selectedAccount.value?.username ?? '');

// 平台对仓库名合法字符的最小公约集
const NAME_PATTERN = /^[A-Za-z0-9._-]+$/;

// 校验规则：账户必选 + 仓库名非空且合法
const rules = computed<FormRules>(() => ({
  accountId: [{ required: true, message: '请选择目标账户', trigger: 'change' }],
  name: [
    { required: true, message: '请输入仓库名', trigger: 'blur' },
    { pattern: NAME_PATTERN, message: '仅允许字母、数字、点、下划线和连字符', trigger: 'blur' },
  ],
}));

// 「发布」按钮是否可用（账户已选 + 仓库名合法）
const canPublish = computed(() => !!form.accountId && NAME_PATTERN.test(form.name.trim()));

// 平台展示名映射
function platformLabel(p: GitPlatform): string {
  switch (p) {
    case 'github':
      return 'GitHub';
    case 'gitlab':
      return 'GitLab';
    case 'gitee':
      return 'Gitee';
    default:
      return p;
  }
}

// 打开对话框时初始化：确保账户 / 设置已加载，并回填默认值
async function onOpen(): Promise<void> {
  // 重置发布态
  publishing.value = false;
  // 账户列表为空时拉取，供选择器使用
  if (accountStore.accounts.length === 0) {
    await accountStore.loadAccounts().catch(() => undefined);
  }
  // 加载设置以取默认 Clone 协议
  await settingsStore.load().catch(() => undefined);

  // 仓库名默认取本地目录名
  form.name = props.defaultName;
  // 描述清空
  form.description = '';
  // 可见性默认私有
  form.visibility = 'private';
  // 协议默认取设置里的默认 Clone 协议
  form.protocol = settingsStore.general.defaultCloneProtocol;

  // 默认选默认账户（须启用），否则退化为首个启用账户
  const def = accountStore.defaultAccountId;
  const defEnabled = !!def && enabledAccounts.value.some((a) => a.id === def);
  form.accountId = defEnabled ? (def as string) : (enabledAccounts.value[0]?.id ?? '');
}

// 切换账户后：新平台非 GitLab 而当前选了 internal 时回退为 private
function onAccountChange(): void {
  if (form.visibility === 'internal' && selectedPlatform.value !== 'gitlab') {
    form.visibility = 'private';
  }
}

// 点击「发布」：校验后调后端命令，按结果分级提示
async function onPublish(): Promise<void> {
  // 无表单引用直接返回（理论不会发生）
  if (!formRef.value) return;
  // 先跑校验，未通过则不提交
  const ok = await formRef.value.validate().catch(() => false);
  if (!ok) return;
  // 进入发布态
  publishing.value = true;
  try {
    // 调后端：建仓 → 关联 → push
    const repo = await localRepositoryApi.publish({
      repoId: props.repoId,
      accountId: form.accountId,
      name: form.name.trim(),
      description: form.description.trim() || undefined,
      visibility: form.visibility,
      protocol: form.protocol,
    });
    // 成功：提示并通知父组件刷新，然后关闭
    ElMessage.success(`已发布到 ${repo.fullName}`);
    emit('published', repo);
    visible.value = false;
  } catch (e) {
    // 规范化错误消息
    const message = e instanceof Error ? e.message : String(e);
    // 重名给简短 toast；其余（含「远程已建但推送失败」的进度说明）用 alert 完整展示
    if (e instanceof GitViewClientError && e.code === 'RepoNameTaken') {
      ElMessage.error(message);
    } else {
      await ElMessageBox.alert(message, '发布失败', { type: 'warning' }).catch(() => undefined);
    }
  } finally {
    // 无论成败退出发布态
    publishing.value = false;
  }
}
</script>

<style scoped>
.namespace {
  color: var(--el-text-color-regular); /* 常规文本色 */
  font-family: var(--el-font-family); /* 与全局字体一致 */
}

.branch-tip {
  margin-top: 4px; /* 与上方表单项留出间距 */
}
</style>
