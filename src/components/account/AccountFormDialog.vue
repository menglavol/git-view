<!--
  账号添加 / 编辑表单对话框（T037）。

  字段集合：
    - 平台选择：GitHub / GitLab.com / 私有 GitLab / Gitee
    - 通用：Web 地址、API 地址（可选）、Token、备注
    - 私有 GitLab 高级：HTTP 允许、自签名证书允许、系统代理、代理 URL、
      超时、默认 Clone 协议、SSH 别名、API 路径前缀

  交互约束：
    - HTTP 勾选 → 弹出安全提示
    - 「测试连接」成功后才允许「保存」
    - Token 输入框 type="password"，不缓存
-->

<template>
  <ElDialog
    v-model="visible"
    :title="isEditing ? '编辑账号' : '添加账号'"
    width="720px"
    :close-on-click-modal="false"
    @close="onClose"
  >
    <ElForm ref="formRef" :model="form" :rules="rules" label-width="120px" label-position="left">
      <!-- 平台选择 -->
      <ElFormItem label="平台" prop="platformChoice">
        <ElRadioGroup v-model="form.platformChoice" :disabled="isEditing">
          <ElRadioButton value="github">GitHub</ElRadioButton>
          <ElRadioButton value="gitlab_com">GitLab.com</ElRadioButton>
          <ElRadioButton value="gitlab_self">私有 GitLab</ElRadioButton>
          <ElRadioButton value="gitee">Gitee</ElRadioButton>
        </ElRadioGroup>
      </ElFormItem>

      <!-- 通用：Web 地址 -->
      <ElFormItem v-if="needsWebUrl" label="Web 地址" prop="webBaseUrl">
        <ElInput
          v-model="form.webBaseUrl"
          placeholder="https://gitlab.example.com"
          clearable
          @blur="onWebUrlBlur"
        />
      </ElFormItem>

      <!-- 通用：API 地址（可选） -->
      <ElFormItem v-if="needsWebUrl" label="API 地址">
        <ElInput v-model="form.apiBaseUrl" placeholder="留空则自动推导" clearable />
      </ElFormItem>

      <!-- 通用：Token -->
      <ElFormItem label="Token" prop="token">
        <ElInput
          v-model="form.token"
          type="password"
          show-password
          autocomplete="off"
          :placeholder="tokenPlaceholder"
        />
      </ElFormItem>

      <!-- 通用：备注 -->
      <ElFormItem label="备注">
        <ElInput
          v-model="form.remark"
          maxlength="50"
          show-word-limit
          placeholder="给该账号添加易记的备注"
        />
      </ElFormItem>

      <!-- 私有 GitLab 高级 -->
      <template v-if="form.platformChoice === 'gitlab_self'">
        <ElDivider content-position="left">高级（仅自建 GitLab）</ElDivider>

        <ElFormItem label="允许 HTTP">
          <ElSwitch :model-value="form.allowInsecureHttp" @update:model-value="onAllowHttpToggle" />
          <span class="hint">仅在内网调试时开启，生产请使用 HTTPS。</span>
        </ElFormItem>

        <ElFormItem label="允许自签名证书">
          <ElSwitch v-model="form.allowInvalidCerts" />
          <span class="hint">放宽 TLS 校验，仅对该实例生效。</span>
        </ElFormItem>

        <ElFormItem label="使用系统代理">
          <ElSwitch v-model="form.useSystemProxy" />
        </ElFormItem>

        <ElFormItem v-if="!form.useSystemProxy" label="代理 URL">
          <ElInput v-model="form.proxyUrl" placeholder="http://127.0.0.1:7890" clearable />
        </ElFormItem>

        <ElFormItem label="请求超时(秒)">
          <ElInputNumber v-model="form.requestTimeoutSeconds" :min="5" :max="300" :step="5" />
        </ElFormItem>

        <ElFormItem label="默认 Clone 协议">
          <ElRadioGroup v-model="form.defaultCloneProtocol">
            <ElRadio value="https">HTTPS</ElRadio>
            <ElRadio value="ssh">SSH</ElRadio>
          </ElRadioGroup>
        </ElFormItem>

        <ElFormItem label="SSH 主机别名">
          <ElInput v-model="form.sshHostAlias" placeholder="留空则使用 host" clearable />
        </ElFormItem>

        <ElFormItem label="API 路径前缀">
          <ElInput
            v-model="form.apiPathPrefix"
            placeholder="如 /api/v4，留空则自动推导"
            clearable
          />
        </ElFormItem>
      </template>

      <!-- 测试连接结果回显 -->
      <ElAlert
        v-if="testResult"
        :title="`连接成功：${testResult.displayName ?? testResult.username}`"
        type="success"
        :closable="false"
        show-icon
        class="test-alert"
      />
      <ElAlert
        v-if="testError"
        :title="`连接失败：${testError}`"
        type="error"
        :closable="false"
        show-icon
        class="test-alert"
      />
    </ElForm>

    <template #footer>
      <ElButton @click="visible = false">取消</ElButton>
      <ElButton :loading="testing" :disabled="!canTest" @click="onTest"> 测试连接 </ElButton>
      <ElButton type="primary" :loading="saving" :disabled="!canSave" @click="onSave">
        保存
      </ElButton>
    </template>
  </ElDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';

import {
  accountApi,
  type AddAccountPayload,
  type AddGitLabInstanceConfigPayload,
  type UserProfile,
} from '@/api/account.api';
import type { Account, GitPlatform } from '@/types/account';

/** 表单内部用的平台选择（含 GitLab 公有/私有细分）。 */
type PlatformChoice = 'github' | 'gitlab_com' | 'gitlab_self' | 'gitee';

interface FormState {
  platformChoice: PlatformChoice;
  webBaseUrl: string;
  apiBaseUrl: string;
  token: string;
  remark: string;
  // GitLab 自建实例字段
  allowInsecureHttp: boolean;
  allowInvalidCerts: boolean;
  useSystemProxy: boolean;
  proxyUrl: string;
  requestTimeoutSeconds: number;
  defaultCloneProtocol: 'https' | 'ssh';
  sshHostAlias: string;
  apiPathPrefix: string;
}

const props = defineProps<{
  modelValue: boolean;
  editingAccount?: Account | null;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void;
  (e: 'saved', account: Account): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v),
});

const isEditing = computed(() => !!props.editingAccount);

const formRef = ref<FormInstance>();
const testing = ref(false);
const saving = ref(false);
const testResult = ref<UserProfile | null>(null);
const testError = ref<string | null>(null);

/** 创建表单初始值。 */
function makeInitialForm(): FormState {
  return {
    platformChoice: 'github',
    webBaseUrl: '',
    apiBaseUrl: '',
    token: '',
    remark: '',
    allowInsecureHttp: false,
    allowInvalidCerts: false,
    useSystemProxy: true,
    proxyUrl: '',
    requestTimeoutSeconds: 30,
    defaultCloneProtocol: 'https',
    sshHostAlias: '',
    apiPathPrefix: '',
  };
}

const form = reactive<FormState>(makeInitialForm());

/** 选择会影响是否需要"Web 地址"字段。 */
const needsWebUrl = computed(
  () => form.platformChoice === 'gitlab_self' || form.platformChoice === 'gitlab_com',
);

const tokenPlaceholder = computed(() => {
  switch (form.platformChoice) {
    case 'github':
      return 'ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
    case 'gitlab_com':
    case 'gitlab_self':
      return 'glpat-xxxxxxxxxxxxxxxxxxxx'; // allow-token-pattern: UI placeholder 示例
    case 'gitee':
      return 'Gitee 个人访问令牌';
    default:
      return '';
  }
});

/** 校验规则（动态：webBaseUrl 仅在 GitLab 路径下必填）。 */
const rules = computed<FormRules>(() => ({
  platformChoice: [{ required: true, message: '请选择平台', trigger: 'change' }],
  token: [{ required: true, message: '请输入 Token', trigger: 'blur' }],
  webBaseUrl:
    form.platformChoice === 'gitlab_self'
      ? [{ required: true, message: '请输入实例 Web 地址', trigger: 'blur' }]
      : [],
}));

/** 是否可点击"测试连接"。 */
const canTest = computed(() => {
  if (!form.token.trim()) return false;
  if (form.platformChoice === 'gitlab_self' && !form.webBaseUrl.trim()) {
    return false;
  }
  return true;
});

/** 是否可点击"保存"（必须先测试通过）。 */
const canSave = computed(() => canTest.value && testResult.value !== null);

/** 弹窗打开时根据 editingAccount 预填或重置。 */
watch(
  () => props.modelValue,
  (open) => {
    if (!open) return;
    Object.assign(form, makeInitialForm());
    testResult.value = null;
    testError.value = null;
    if (props.editingAccount) {
      hydrateFromAccount(props.editingAccount);
    }
  },
);

function hydrateFromAccount(account: Account): void {
  form.platformChoice = inferChoice(account.platform, account.webBaseUrl);
  form.webBaseUrl = account.webBaseUrl;
  form.apiBaseUrl = account.apiBaseUrl;
  form.remark = account.remark ?? '';
  // 编辑模式不展示原 token；用户需重新输入
}

function inferChoice(platform: GitPlatform, webUrl: string): PlatformChoice {
  if (platform === 'github') return 'github';
  if (platform === 'gitee') return 'gitee';
  // gitlab
  try {
    const u = new URL(webUrl);
    return u.hostname === 'gitlab.com' ? 'gitlab_com' : 'gitlab_self';
  } catch {
    return 'gitlab_com';
  }
}

/** Web URL 失去焦点时自动推导默认值（仅 GitLab 自建场景）。 */
function onWebUrlBlur(): void {
  if (form.platformChoice !== 'gitlab_self') return;
  if (!form.apiBaseUrl.trim() && form.webBaseUrl.trim()) {
    // 简单推导：交给后端在测试时再次确认
    form.apiBaseUrl = `${form.webBaseUrl.replace(/\/+$/, '')}/api/v4`;
  }
}

/** 允许 HTTP 切换时弹安全提示。 */
async function onAllowHttpToggle(value: boolean | string | number): Promise<void> {
  const newVal = Boolean(value);
  if (!newVal) {
    form.allowInsecureHttp = false;
    return;
  }
  try {
    await ElMessageBox.confirm(
      'HTTP 通信不加密，可能导致 Token 泄露。仅在内网调试时开启。是否继续？',
      '安全提示',
      { type: 'warning', confirmButtonText: '继续', cancelButtonText: '取消' },
    );
    form.allowInsecureHttp = true;
  } catch {
    form.allowInsecureHttp = false;
  }
}

/** 把表单装配为 AddAccountPayload。 */
function buildPayload(): AddAccountPayload {
  const platform: GitPlatform = (() => {
    switch (form.platformChoice) {
      case 'github':
        return 'github';
      case 'gitee':
        return 'gitee';
      default:
        return 'gitlab';
    }
  })();

  const webBaseUrl = (() => {
    switch (form.platformChoice) {
      case 'github':
        return 'https://github.com';
      case 'gitlab_com':
        return 'https://gitlab.com';
      case 'gitee':
        return 'https://gitee.com';
      default:
        return form.webBaseUrl.trim();
    }
  })();

  const instanceConfig: AddGitLabInstanceConfigPayload | undefined =
    form.platformChoice === 'gitlab_self'
      ? {
          allowInsecureHttp: form.allowInsecureHttp,
          allowInvalidCerts: form.allowInvalidCerts,
          useSystemProxy: form.useSystemProxy,
          proxyUrl: form.proxyUrl.trim() || undefined,
          requestTimeoutSeconds: form.requestTimeoutSeconds,
          defaultCloneProtocol: form.defaultCloneProtocol,
          sshHostAlias: form.sshHostAlias.trim() || undefined,
          apiPathPrefix: form.apiPathPrefix.trim() || undefined,
        }
      : undefined;

  return {
    platform,
    webBaseUrl,
    apiBaseUrl: form.apiBaseUrl.trim() || undefined,
    token: form.token,
    remark: form.remark.trim() || undefined,
    instanceConfig,
  };
}

async function onTest(): Promise<void> {
  if (!formRef.value) return;
  const ok = await formRef.value.validate().catch(() => false);
  if (!ok) return;
  testing.value = true;
  testResult.value = null;
  testError.value = null;
  try {
    const payload = buildPayload();
    const profile = await accountApi.test(payload);
    testResult.value = profile;
  } catch (e) {
    testError.value = e instanceof Error ? e.message : String(e);
  } finally {
    testing.value = false;
  }
}

async function onSave(): Promise<void> {
  if (!canSave.value) return;
  saving.value = true;
  try {
    const payload = buildPayload();
    const account = await accountApi.add(payload);
    ElMessage.success('账号已添加');
    emit('saved', account);
    visible.value = false;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(`保存失败：${msg}`);
  } finally {
    saving.value = false;
  }
}

function onClose(): void {
  // 关闭时强制清空 token 字段，避免内存残留
  form.token = '';
  testResult.value = null;
  testError.value = null;
}
</script>

<style scoped>
.hint {
  margin-left: 12px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.test-alert {
  margin-top: 8px;
}
</style>
