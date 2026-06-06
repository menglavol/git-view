<!--
  设置中心页面（US7 / T102）。

  五个 Tab：通用 / Git / 网络 / 外部工具 / 账号与安全。
  设计要点：
    - 编辑走「本地表单副本」而非直接改 store：用户改动到点「保存」才原子写库，
      期间可随时「重置」丢弃；避免每改一个字段就触发一次 IPC 写入。
    - 主题为唯一的「改即预览」字段：选中即调 appStore.applyTheme 实时切换外观，
      让用户立刻看到效果；其余字段一律到保存才生效（语言在保存时随副作用切换）。
    - 「账号与安全」Tab 只显示凭据存在性（已存储 / 缺失），绝不显示 Token 明文；
      凭据缺失提示用户用「重新验证」修复，删除凭据走 ConfirmDangerDialog 二次确认。
    - 文案统一走 i18n（仅本页子树接入 t()，见 src/i18n）。
-->
<template>
  <!-- 页面容器：加载设置时整体 loading 遮罩 -->
  <div v-loading="settingsStore.loading" class="page-settings">
    <!-- 顶部标题栏：标题 + 重置 / 保存按钮 -->
    <div class="settings-header">
      <h1 class="settings-title">{{ t('settings.title') }}</h1>
      <div class="header-actions">
        <!-- 重置：把本地表单回滚到 store 当前快照，并复原主题预览 -->
        <el-button :disabled="settingsStore.saving" @click="onReset">
          {{ t('common.reset') }}
        </el-button>
        <!-- 保存：原子写库；saving 期间禁用并显示「保存中」 -->
        <el-button type="primary" :loading="settingsStore.saving" @click="onSave">
          {{ settingsStore.saving ? t('common.saving') : t('common.save') }}
        </el-button>
      </div>
    </div>

    <!-- 五 Tab 主体；切到「账号与安全」时懒加载账号与凭据状态 -->
    <el-tabs v-model="activeTab" class="settings-tabs" @tab-change="onTabChange">
      <!-- ========================= 通用 ========================= -->
      <el-tab-pane :label="t('settings.tabs.general')" name="general">
        <el-form label-width="200px" label-position="right" class="settings-form">
          <!-- 默认仓库根目录：输入 + 浏览（系统目录选择器） -->
          <el-form-item :label="t('settings.general.repoBaseDir')">
            <el-input
              v-model="form.general.defaultRepoBaseDir"
              :placeholder="t('settings.general.repoBaseDirPlaceholder')"
            >
              <template #append>
                <el-button @click="pickRepoBaseDir">{{ t('common.browse') }}</el-button>
              </template>
            </el-input>
          </el-form-item>

          <!-- 默认克隆协议：HTTPS / SSH 单选 -->
          <el-form-item :label="t('settings.general.cloneProtocol')">
            <el-radio-group v-model="form.general.defaultCloneProtocol">
              <el-radio-button value="https">{{ t('settings.protocol.https') }}</el-radio-button>
              <el-radio-button value="ssh">{{ t('settings.protocol.ssh') }}</el-radio-button>
            </el-radio-group>
          </el-form-item>

          <!-- 默认并发数：1-8；改动需重启生效，附提示 -->
          <el-form-item :label="t('settings.general.concurrency')">
            <el-input-number v-model="form.general.defaultConcurrency" :min="1" :max="8" />
            <span class="field-hint">{{ t('settings.general.concurrencyHint') }}</span>
          </el-form-item>

          <!-- 目录组织方式：三选一 -->
          <el-form-item :label="t('settings.general.directoryStrategy')">
            <el-select v-model="form.general.directoryStrategy" class="wide-select">
              <el-option value="flat" :label="t('settings.strategy.flat')" />
              <el-option value="by_owner" :label="t('settings.strategy.byOwner')" />
              <el-option
                value="by_platform_and_owner"
                :label="t('settings.strategy.byPlatformAndOwner')"
              />
            </el-select>
          </el-form-item>

          <!-- 主题：唯一「改即预览」字段，change 立即 applyTheme -->
          <el-form-item :label="t('settings.general.theme')">
            <el-select v-model="form.general.theme" class="narrow-select" @change="onThemePreview">
              <el-option value="auto" :label="t('settings.theme.auto')" />
              <el-option value="light" :label="t('settings.theme.light')" />
              <el-option value="dark" :label="t('settings.theme.dark')" />
            </el-select>
          </el-form-item>

          <!-- 语言：保存后随副作用切换 i18n locale -->
          <el-form-item :label="t('settings.general.language')">
            <el-select v-model="form.general.language" class="narrow-select">
              <el-option value="zh_cn" :label="t('settings.language.zhCn')" />
              <el-option value="en_us" :label="t('settings.language.enUs')" />
            </el-select>
          </el-form-item>

          <!-- 启动时打开上次仓库 -->
          <el-form-item :label="t('settings.general.openLastRepo')">
            <el-switch v-model="form.general.openLastRepoOnStartup" />
          </el-form-item>

          <!-- 启动时自动检查仓库状态 -->
          <el-form-item :label="t('settings.general.autoCheckStatus')">
            <el-switch v-model="form.general.autoCheckRepoStatus" />
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ========================= Git ========================= -->
      <el-tab-pane :label="t('settings.tabs.git')" name="git">
        <el-form label-width="200px" label-position="right" class="settings-form">
          <!-- Git 可执行路径：输入 + 「检测 Git」；非空时检测=校验该路径并持久化 -->
          <el-form-item :label="t('settings.git.executablePath')">
            <el-input
              v-model="form.git.gitExecutablePath"
              :placeholder="t('settings.git.executablePathPlaceholder')"
            >
              <template #append>
                <el-button :loading="settingsStore.detecting" @click="onDetectGit">
                  {{
                    settingsStore.detecting ? t('settings.git.detecting') : t('settings.git.detect')
                  }}
                </el-button>
              </template>
            </el-input>
          </el-form-item>

          <!-- 检测结果回显：成功显示版本，失败显示警告 -->
          <el-form-item v-if="gitDetection" label=" ">
            <el-alert
              v-if="gitDetection.found"
              type="success"
              :closable="false"
              show-icon
              :title="`${t('settings.git.detected')}：${gitDetection.version ?? ''}`"
            />
            <el-alert
              v-else
              type="warning"
              :closable="false"
              show-icon
              :title="t('settings.git.notDetected')"
            />
          </el-form-item>

          <!-- 提交身份 user.name；留空则沿用 git 全局配置 -->
          <el-form-item :label="t('settings.git.userName')">
            <el-input v-model="form.git.userName" />
          </el-form-item>

          <!-- 提交身份 user.email -->
          <el-form-item :label="t('settings.git.userEmail')">
            <el-input v-model="form.git.userEmail" />
          </el-form-item>

          <!-- 默认 Pull 策略 -->
          <el-form-item :label="t('settings.git.pullStrategy')">
            <el-select v-model="form.git.defaultPullStrategy" class="wide-select">
              <el-option value="ff_only" :label="t('settings.pull.ffOnly')" />
              <el-option value="rebase" :label="t('settings.pull.rebase')" />
              <el-option value="merge" :label="t('settings.pull.merge')" />
            </el-select>
          </el-form-item>

          <!-- 默认 Push 策略 -->
          <el-form-item :label="t('settings.git.pushStrategy')">
            <el-select v-model="form.git.defaultPushStrategy" class="wide-select">
              <el-option value="simple" :label="t('settings.push.simple')" />
              <el-option value="current" :label="t('settings.push.current')" />
              <el-option value="upstream" :label="t('settings.push.upstream')" />
            </el-select>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ========================= 网络 ========================= -->
      <el-tab-pane :label="t('settings.tabs.network')" name="network">
        <el-form label-width="200px" label-position="right" class="settings-form">
          <!-- 跟随系统代理：开启后手填的代理地址将被忽略，故置灰下方两项 -->
          <el-form-item :label="t('settings.network.useSystemProxy')">
            <el-switch v-model="form.network.useSystemProxy" />
            <span class="field-hint">{{ t('settings.network.useSystemProxyHint') }}</span>
          </el-form-item>

          <!-- HTTP 代理：跟随系统时禁用 -->
          <el-form-item :label="t('settings.network.httpProxy')">
            <el-input
              v-model="form.network.httpProxy"
              :disabled="form.network.useSystemProxy"
              :placeholder="t('settings.network.proxyPlaceholder')"
            />
          </el-form-item>

          <!-- HTTPS 代理：跟随系统时禁用 -->
          <el-form-item :label="t('settings.network.httpsProxy')">
            <el-input
              v-model="form.network.httpsProxy"
              :disabled="form.network.useSystemProxy"
              :placeholder="t('settings.network.proxyPlaceholder')"
            />
          </el-form-item>

          <!-- API 请求超时（秒） -->
          <el-form-item :label="t('settings.network.apiTimeout')">
            <el-input-number v-model="form.network.apiTimeoutSecs" :min="1" :max="600" />
            <span class="field-hint">{{ t('common.seconds') }}</span>
          </el-form-item>

          <!-- 克隆超时（秒） -->
          <el-form-item :label="t('settings.network.cloneTimeout')">
            <el-input-number v-model="form.network.cloneTimeoutSecs" :min="1" :max="3600" />
            <span class="field-hint">{{ t('common.seconds') }}</span>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ========================= 外部工具 ========================= -->
      <el-tab-pane :label="t('settings.tabs.externalTools')" name="externalTools">
        <el-form label-width="200px" label-position="right" class="settings-form">
          <!-- 默认编辑器命令（如 code / cursor） -->
          <el-form-item :label="t('settings.externalTools.editor')">
            <el-input
              v-model="form.externalTools.editorCommand"
              :placeholder="t('settings.externalTools.editorPlaceholder')"
            />
          </el-form-item>

          <!-- 默认终端命令 -->
          <el-form-item :label="t('settings.externalTools.terminal')">
            <el-input v-model="form.externalTools.terminalCommand" />
          </el-form-item>

          <!-- 默认文件管理器命令 -->
          <el-form-item :label="t('settings.externalTools.fileManager')">
            <el-input v-model="form.externalTools.fileManagerCommand" />
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- ===================== 账号与安全 ===================== -->
      <el-tab-pane :label="t('settings.tabs.accountSecurity')" name="security">
        <!-- 安全声明：仅显示存在性，绝不显示明文 -->
        <p class="security-desc">{{ t('settings.security.description') }}</p>

        <!-- 无账号空态：引导去账号管理添加 -->
        <el-empty
          v-if="accountStore.accounts.length === 0"
          :description="t('settings.security.noAccounts')"
        />

        <!-- 账号凭据状态表 -->
        <el-table v-else :data="accountStore.accounts" class="security-table">
          <!-- 账号列：用户名 + 显示名 -->
          <el-table-column :label="t('settings.security.columnAccount')" min-width="180">
            <template #default="{ row }">
              <span class="account-name">{{ row.username }}</span>
              <span v-if="row.displayName" class="account-display">（{{ row.displayName }}）</span>
            </template>
          </el-table-column>

          <!-- 平台列：彩色 Tag -->
          <el-table-column :label="t('settings.security.columnPlatform')" width="120">
            <template #default="{ row }">
              <el-tag size="small" :type="platformTagType(row.platform)">
                {{ row.platform }}
              </el-tag>
            </template>
          </el-table-column>

          <!-- 凭据状态列：检查中 / 已存储（绿）/ 缺失（红，附 tooltip） -->
          <el-table-column :label="t('settings.security.columnCredential')" width="160">
            <template #default="{ row }">
              <el-tag v-if="credStatus[row.id] === 'checking'" size="small" type="info">
                {{ t('settings.security.checking') }}
              </el-tag>
              <el-tag v-else-if="credStatus[row.id] === 'stored'" size="small" type="success">
                {{ t('settings.security.credStored') }}
              </el-tag>
              <el-tooltip v-else :content="t('settings.security.credMissingHint')" placement="top">
                <el-tag size="small" type="danger">
                  {{ t('settings.security.credMissing') }}
                </el-tag>
              </el-tooltip>
            </template>
          </el-table-column>

          <!-- 操作列：重新验证 / 删除凭据 -->
          <el-table-column :label="t('settings.security.columnActions')" width="220">
            <template #default="{ row }">
              <el-button size="small" @click="openRevalidate(row)">
                {{ t('settings.security.revalidate') }}
              </el-button>
              <!-- 删除凭据：凭据不存在时禁用（无可删） -->
              <el-button
                size="small"
                type="danger"
                plain
                :disabled="credStatus[row.id] !== 'stored'"
                @click="openDeleteCredential(row)"
              >
                {{ t('settings.security.deleteCredential') }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <!-- 重新验证对话框：输入新 Token → 测试连接通过后保存到 keyring -->
    <el-dialog
      v-model="revalidateVisible"
      :title="t('settings.security.revalidateTitle')"
      width="480px"
      :close-on-click-modal="false"
    >
      <!-- 当前账号信息提示 -->
      <p v-if="revalidateAccount" class="revalidate-account">
        {{ revalidateAccount.username }} · {{ revalidateAccount.platform }}
      </p>
      <el-form label-position="top">
        <el-form-item :label="t('settings.security.tokenLabel')">
          <!-- Token 输入：password 类型，绝不缓存到本地存储 -->
          <el-input
            v-model="revalidateToken"
            type="password"
            show-password
            :placeholder="t('settings.security.tokenPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button :disabled="revalidating" @click="revalidateVisible = false">
          {{ t('common.cancel') }}
        </el-button>
        <el-button
          type="primary"
          :loading="revalidating"
          :disabled="!revalidateToken.trim()"
          @click="onRevalidateConfirm"
        >
          {{ t('settings.security.testAndSave') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 删除凭据的危险确认：要求输入用户名作为二次确认关键词 -->
    <ConfirmDangerDialog
      v-model:visible="deleteCredVisible"
      :title="t('settings.security.deleteConfirmTitle')"
      :message="t('settings.security.deleteConfirmMessage')"
      :recoverability-hint="t('settings.security.deleteConfirmHint')"
      :confirm-keyword="deleteCredAccount?.username ?? ''"
      :confirm-button-text="t('settings.security.deleteCredential')"
      :loading="deletingCred"
      @confirm="onDeleteCredentialConfirm"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * 设置页脚本（US7 / T102）。
 *
 * 状态结构：
 *   - form：本地可编辑副本（SettingsForm），与 store.settings 解耦；
 *     可选字符串字段在副本里规约为非空 string（默认 ''），保证 v-model 类型干净；
 *     保存时再经 toSettings() 把空串还原成 undefined，让后端存 None 而非空串。
 *   - credStatus：账号 → 凭据状态（checking/stored/missing），账号与安全 Tab 用。
 */
import { onMounted, reactive, ref } from 'vue';
import { ElMessage } from 'element-plus';
import { useI18n } from 'vue-i18n';

import ConfirmDangerDialog from '@/components/common/ConfirmDangerDialog.vue';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

import { accountApi } from '@/api/account.api';
import { useAccountStore } from '@/stores/account';
import { useAppStore } from '@/stores/app';
import { useSettingsStore } from '@/stores/settings';
import type { Account, GitPlatform } from '@/types/account';
import type {
  GeneralSettings,
  GitDetectionResult,
  PullStrategy,
  PushStrategy,
  Settings,
  Theme,
} from '@/types/settings';

const { t } = useI18n();
const settingsStore = useSettingsStore();
const appStore = useAppStore();
const accountStore = useAccountStore();

// 凭据状态三态：用本地副本而非直接改 store，便于断言展示与回填。
type CredState = 'checking' | 'stored' | 'missing';

/**
 * 本地表单类型：把 Settings 里的可选字符串字段「拍平」为必填 string。
 *
 * 理由：el-input 的 v-model 期望 string，绑定 `string | undefined` 会触发 TS 报错；
 * 统一在副本里用 '' 占位，提交时再由 toSettings() 把空串折回 undefined。
 */
interface SettingsForm {
  general: GeneralSettings;
  git: {
    gitExecutablePath: string;
    userName: string;
    userEmail: string;
    defaultPullStrategy: PullStrategy;
    defaultPushStrategy: PushStrategy;
  };
  network: {
    httpProxy: string;
    httpsProxy: string;
    useSystemProxy: boolean;
    apiTimeoutSecs: number;
    cloneTimeoutSecs: number;
  };
  externalTools: {
    editorCommand: string;
    terminalCommand: string;
    fileManagerCommand: string;
  };
}

// 当前激活的 Tab；默认进入「通用」。
const activeTab = ref('general');
// Git 检测结果回显（null 表示尚未检测）。
const gitDetection = ref<GitDetectionResult | null>(null);
// 账号凭据状态映射；账号与安全 Tab 首次激活时填充。
const credStatus = reactive<Record<string, CredState>>({});
// 账号与安全 Tab 是否已加载过（避免每次切 Tab 都重复请求）。
let securityLoaded = false;

// 重新验证对话框状态
const revalidateVisible = ref(false); // 对话框是否可见
const revalidateAccount = ref<Account | null>(null); // 当前正在重新验证的账号
const revalidateToken = ref(''); // 用户输入的新 Token（仅内存，不落本地存储）
const revalidating = ref(false); // 测试+保存进行中，禁用按钮防重复提交

// 删除凭据确认对话框状态
const deleteCredVisible = ref(false); // 危险确认对话框是否可见
const deleteCredAccount = ref<Account | null>(null); // 当前待删除凭据的账号
const deletingCred = ref(false); // 删除请求进行中，按钮显示 loading

// 把空白字符串折成 undefined：用于可选字段，避免把 "" 当成有效路径/代理存进后端。
function blankToUndef(s: string): string | undefined {
  const trimmed = s.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

/** 由后端 Settings 构造本地表单副本（可选字段缺失时回退 ''）。 */
function fromSettings(s: Settings): SettingsForm {
  return {
    general: { ...s.general }, // 通用组无可选字符串字段，整组浅拷贝即可
    git: {
      gitExecutablePath: s.git.gitExecutablePath ?? '', // 缺省路径回退空串，供 input 绑定
      userName: s.git.userName ?? '', // 缺省身份回退空串
      userEmail: s.git.userEmail ?? '', // 缺省邮箱回退空串
      defaultPullStrategy: s.git.defaultPullStrategy, // 枚举必有值，直接搬运
      defaultPushStrategy: s.git.defaultPushStrategy, // 枚举必有值，直接搬运
    },
    network: {
      httpProxy: s.network.httpProxy ?? '', // 缺省代理回退空串
      httpsProxy: s.network.httpsProxy ?? '', // 缺省代理回退空串
      useSystemProxy: s.network.useSystemProxy, // 布尔必有值
      apiTimeoutSecs: s.network.apiTimeoutSecs, // 数值必有值
      cloneTimeoutSecs: s.network.cloneTimeoutSecs, // 数值必有值
    },
    externalTools: {
      editorCommand: s.externalTools.editorCommand ?? '', // 缺省命令回退空串
      terminalCommand: s.externalTools.terminalCommand ?? '', // 缺省命令回退空串
      fileManagerCommand: s.externalTools.fileManagerCommand ?? '', // 缺省命令回退空串
    },
  };
}

/** 把本地表单副本折回后端 Settings（空串转 undefined）。 */
function toSettings(f: SettingsForm): Settings {
  return {
    general: { ...f.general }, // 通用组原样回写
    git: {
      gitExecutablePath: blankToUndef(f.git.gitExecutablePath), // 空串→undefined，后端存 None
      userName: blankToUndef(f.git.userName), // 空串→undefined
      userEmail: blankToUndef(f.git.userEmail), // 空串→undefined
      defaultPullStrategy: f.git.defaultPullStrategy, // 枚举原样回写
      defaultPushStrategy: f.git.defaultPushStrategy, // 枚举原样回写
    },
    network: {
      httpProxy: blankToUndef(f.network.httpProxy), // 空串→undefined
      httpsProxy: blankToUndef(f.network.httpsProxy), // 空串→undefined
      useSystemProxy: f.network.useSystemProxy, // 布尔原样回写
      apiTimeoutSecs: f.network.apiTimeoutSecs, // 数值原样回写
      cloneTimeoutSecs: f.network.cloneTimeoutSecs, // 数值原样回写
    },
    externalTools: {
      editorCommand: blankToUndef(f.externalTools.editorCommand), // 空串→undefined
      terminalCommand: blankToUndef(f.externalTools.terminalCommand), // 空串→undefined
      fileManagerCommand: blankToUndef(f.externalTools.fileManagerCommand), // 空串→undefined
    },
  };
}

// 本地表单副本：用 store 当前快照初始化（store 已含首屏默认值；onMounted 再刷新）。
const form = reactive<SettingsForm>(fromSettings(settingsStore.settings));

/** 用给定 Settings 覆盖本地表单（加载 / 重置时调用）。 */
function syncForm(s: Settings): void {
  Object.assign(form, fromSettings(s));
}

/** 平台 → Tag 颜色：仅作视觉区分，无业务含义。 */
function platformTagType(platform: GitPlatform): 'success' | 'warning' | 'danger' {
  if (platform === 'github') return 'success';
  if (platform === 'gitlab') return 'warning';
  return 'danger';
}

// ---------------------------------------------------------------------
// 通用 Tab 交互
// ---------------------------------------------------------------------

/** 选择默认仓库根目录：调系统目录选择器，取消时不改动。 */
async function pickRepoBaseDir(): Promise<void> {
  const selected = await openDialog({ directory: true, multiple: false });
  // 用户取消返回 null；多选返回数组——本场景只接受单字符串路径。
  if (typeof selected === 'string') {
    form.general.defaultRepoBaseDir = selected;
  }
}

/** 主题「改即预览」：立即应用，让用户直观看到深/浅色切换；保存才持久化。 */
function onThemePreview(theme: Theme): void {
  appStore.applyTheme(theme);
}

// ---------------------------------------------------------------------
// Git Tab 交互
// ---------------------------------------------------------------------

/**
 * 「检测 Git」按钮。
 *
 * - 输入框非空：把它当作用户指定的路径去「校验 + 持久化」（setGitPath）；
 *   失败说明路径无效，提示用户而非静默回退。
 * - 输入框为空：走自动探测（detectGit），从 PATH / 常见位置找 git。
 * 检测到身份且本地表单对应字段为空时顺手回填，省去手动输入。
 */
async function onDetectGit(): Promise<void> {
  const path = form.git.gitExecutablePath.trim();
  try {
    const result = path ? await settingsStore.setGitPath(path) : await settingsStore.detectGit();
    gitDetection.value = result;
    // 同步到 app store：让首次启动引导的路由守卫感知最新 git 状态（T109），
    // 否则用户在此修好路径后，守卫仍用启动时的过期结果把他弹回引导页。
    appStore.gitDetection = result;
    if (result.found) {
      // 回填探测到的路径与身份（仅在用户未手填时填充，避免覆盖用户输入）
      if (result.path) form.git.gitExecutablePath = result.path;
      if (result.userName && !form.git.userName) form.git.userName = result.userName;
      if (result.userEmail && !form.git.userEmail) form.git.userEmail = result.userEmail;
    }
  } catch (e) {
    // setGitPath 校验失败（路径不可执行）会抛错，明确提示用户
    gitDetection.value = { found: false };
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
}

// ---------------------------------------------------------------------
// 顶部保存 / 重置
// ---------------------------------------------------------------------

/** 保存：把本地副本折回 Settings 原子写库，成功 / 失败均给 Message 反馈。 */
async function onSave(): Promise<void> {
  try {
    await settingsStore.save(toSettings(form));
    ElMessage.success(t('settings.saveSuccess'));
  } catch (e) {
    ElMessage.error(`${t('settings.saveFailed')}：${e instanceof Error ? e.message : String(e)}`);
  }
}

/** 重置：丢弃未保存改动，回滚到 store 快照，并复原主题预览（撤销实时预览）。 */
function onReset(): void {
  syncForm(settingsStore.settings);
  appStore.applyTheme(settingsStore.settings.general.theme);
  gitDetection.value = null;
}

// ---------------------------------------------------------------------
// 账号与安全 Tab
// ---------------------------------------------------------------------

/** 逐个账号查询 keyring 中凭据是否存在，更新 credStatus。 */
async function refreshCredStatus(): Promise<void> {
  await Promise.all(
    accountStore.accounts.map(async (acc) => {
      credStatus[acc.id] = 'checking';
      try {
        const exists = await accountApi.checkCredentialExists(acc.id);
        credStatus[acc.id] = exists ? 'stored' : 'missing';
      } catch {
        // 查询失败按「缺失」保守处理，提示用户重新验证总比误显示「已存储」安全
        credStatus[acc.id] = 'missing';
      }
    }),
  );
}

/** Tab 切换：首次进入「账号与安全」时懒加载账号列表与凭据状态。 */
async function onTabChange(name: string | number): Promise<void> {
  if (name !== 'security' || securityLoaded) return;
  securityLoaded = true;
  try {
    // 账号列表可能尚未加载（用户直接进设置页），先确保有数据
    if (accountStore.accounts.length === 0) {
      await accountStore.loadAccounts();
    }
    await refreshCredStatus();
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
}

/** 打开「重新验证」对话框。 */
function openRevalidate(account: Account): void {
  revalidateAccount.value = account;
  revalidateToken.value = '';
  revalidateVisible.value = true;
}

/**
 * 重新验证确认：用新 Token 测试连接，通过后写入 keyring。
 *
 * 先 test 后 save：仅在连接测试通过（Token 有效）时才落库，避免把无效 Token
 * 存进密钥库造成「显示已存储但其实用不了」的假象。
 */
async function onRevalidateConfirm(): Promise<void> {
  const account = revalidateAccount.value; // 当前账号
  const token = revalidateToken.value.trim(); // 去除首尾空白后的 Token
  if (!account || !token) return; // 缺账号或空 Token 直接忽略
  revalidating.value = true; // 进入加载态，禁用按钮
  try {
    // 复用账号已有的平台与地址构造测试连接 payload
    await accountApi.test({
      platform: account.platform, // 沿用账号平台
      webBaseUrl: account.webBaseUrl, // 沿用账号 Web 地址
      apiBaseUrl: account.apiBaseUrl, // 沿用账号 API 地址
      token, // 待验证的新 Token
    });
    await accountApi.saveCredential(account.id, token); // 测试通过才落库
    credStatus[account.id] = 'stored'; // 本地状态立即置「已存储」
    ElMessage.success(t('settings.security.revalidateSuccess'));
    revalidateVisible.value = false; // 成功后关闭对话框
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e)); // 测试/保存失败提示
  } finally {
    revalidating.value = false; // 无论成败都退出加载态
  }
}

/** 打开删除凭据的危险确认。 */
function openDeleteCredential(account: Account): void {
  deleteCredAccount.value = account;
  deleteCredVisible.value = true;
}

/** 删除凭据确认：只清 keyring 里的 Token，保留账号元数据。 */
async function onDeleteCredentialConfirm(): Promise<void> {
  const account = deleteCredAccount.value;
  if (!account) return;
  deletingCred.value = true;
  try {
    await accountApi.deleteCredential(account.id);
    credStatus[account.id] = 'missing';
    ElMessage.success(t('settings.security.deleteSuccess'));
    deleteCredVisible.value = false;
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    deletingCred.value = false;
  }
}

// 进入页面：从后端拉取最新设置并同步到本地副本（store.load 同时应用主题/语言）。
onMounted(async () => {
  try {
    await settingsStore.load();
    syncForm(settingsStore.settings);
  } catch (e) {
    // 加载失败：保留首屏默认表单，给出提示，用户可切走再回重试
    ElMessage.error(`${t('settings.loadFailed')}：${e instanceof Error ? e.message : String(e)}`);
  }
});
</script>

<style scoped>
/* 页面容器：统一内边距，左右略宽于上下，贴合设置类页面的阅读习惯 */
.page-settings {
  padding: 16px 24px; /* 上下 16、左右 24 */
}

/* 顶部标题栏：标题靠左、操作按钮靠右的两端对齐布局 */
.settings-header {
  display: flex; /* 横向排布标题与按钮 */
  align-items: center; /* 垂直居中对齐 */
  justify-content: space-between; /* 两端对齐：标题左、按钮右 */
  margin-bottom: 8px; /* 与下方 Tab 区留出间距 */
}

/* 页面主标题字号 */
.settings-title {
  margin: 0; /* 去掉 h1 默认外边距，避免顶部留白过大 */
  font-size: 20px; /* 主标题字号 */
}

/* 表单整体限宽，避免在宽屏上输入框拉得过长不易阅读 */
.settings-form {
  max-width: 720px; /* 限制最大宽度，保证表单紧凑可读 */
  margin-top: 12px; /* 与 Tab 头留出间距 */
}

/* 字段右侧的灰色辅助提示（如「重启生效」「秒」） */
.field-hint {
  margin-left: 12px; /* 与左侧控件留出间距 */
  color: var(--el-text-color-secondary); /* 次要文字色，弱化为辅助说明 */
  font-size: 12px; /* 比正文略小 */
}

/* 下拉宽度：策略类文案较长用宽，主题/语言选项短用窄 */
.wide-select {
  width: 360px; /* 较长选项（目录策略、pull/push）用宽下拉 */
}
.narrow-select {
  width: 200px; /* 短选项（主题、语言）用窄下拉 */
}

/* 账号与安全：安全声明文案用次色弱化 */
.security-desc {
  color: var(--el-text-color-secondary); /* 次要文字色 */
  font-size: 13px; /* 略小字号 */
  margin: 4px 0 16px; /* 与表格留出间距 */
}

/* 凭据状态表占满容器宽度 */
.security-table {
  width: 100%; /* 撑满主区宽度 */
}

/* 账号名加粗突出、显示名用次色弱化为附注 */
.account-name {
  font-weight: 600; /* 加粗主标识 */
}
.account-display {
  color: var(--el-text-color-secondary); /* 次色 */
  font-size: 12px; /* 小字附注 */
}

/* 重新验证对话框中的账号信息提示 */
.revalidate-account {
  margin: 0 0 12px; /* 与下方表单留间距 */
  color: var(--el-text-color-secondary); /* 次色 */
  font-size: 13px; /* 略小字号 */
}
</style>
