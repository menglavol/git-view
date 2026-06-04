// =====================================================================
// 应用设置 store（US7 / T102·T103）
// 职责：作为 Settings 页面的状态中枢，封装：
//   1) 完整设置快照的加载 / 保存（委托 settingsApi）
//   2) 主题与语言的副作用联动（applyTheme / setI18nLocale）
//   3) Git 环境检测与路径设置
// 设计原则（与 operationLog store 一致）：
//   - 组件不直接调底层 API，统一经由 store action
//   - actions 在 try/finally 中切换 loading/saving，避免 UI 挂起
// =====================================================================

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { settingsApi } from '@/api/settings.api';
import { setI18nLocale } from '@/i18n';
import type { GitDetectionResult, Settings } from '@/types/settings';

import { useAppStore } from './app';

// 前端首屏占位默认值：真正默认由后端 settings_service 提供，load() 后覆盖。
// 与后端 Default 对齐（并发 3、两级目录、Auto 主题、中文、FfOnly/Simple）。
const DEFAULT_SETTINGS: Settings = {
  general: {
    defaultRepoBaseDir: '', // 真实默认（~/Projects）由后端给出，首屏先空
    defaultCloneProtocol: 'https', // 默认 HTTPS，兼容性最好
    defaultConcurrency: 3, // 默认并发 3，平衡速度与资源
    directoryStrategy: 'by_platform_and_owner', // 默认按平台+所有者两级归档
    theme: 'auto', // 默认跟随系统配色
    language: 'zh_cn', // 默认简体中文
    openLastRepoOnStartup: false, // 默认不自动打开上次仓库
    autoCheckRepoStatus: true, // 默认启动时检查仓库状态
  },
  git: {
    defaultPullStrategy: 'ff_only', // 默认仅快进，避免意外 merge commit
    defaultPushStrategy: 'simple', // 默认 simple，最安全的推送策略
  },
  network: {
    useSystemProxy: false, // 默认不跟随系统代理
    apiTimeoutSecs: 30, // API 请求默认 30 秒超时
    cloneTimeoutSecs: 300, // 克隆默认 5 分钟超时（大仓库留足余量）
  },
  externalTools: {}, // 外部工具命令默认全空，由用户按需填写
};

export const useSettingsStore = defineStore('settings', () => {
  /** 当前应用设置快照 */
  const settings = ref<Settings>(structuredClone(DEFAULT_SETTINGS));
  /** 加载中：表单 loading 遮罩 */
  const loading = ref(false);
  /** 保存中：禁用保存按钮，避免重复提交 */
  const saving = ref(false);
  /** Git 检测 / 设置路径进行中：禁用相关按钮 */
  const detecting = ref(false);
  /** 最近一次错误（已脱敏）：UI 顶部提示可订阅 */
  const error = ref<string | null>(null);

  // 分组只读视图，方便组件按 Tab 取用（编辑走本地副本 + save）
  const general = computed(() => settings.value.general);
  const git = computed(() => settings.value.git);
  const network = computed(() => settings.value.network);
  const externalTools = computed(() => settings.value.externalTools);

  /**
   * 把主题与语言联动到全局。
   *
   * 主题 → html.dark class（app store）；语言 → i18n locale。
   * 加载与保存后均调用，确保「设置即所见」。
   */
  function applySideEffects(): void {
    const appStore = useAppStore();
    appStore.applyTheme(settings.value.general.theme);
    setI18nLocale(settings.value.general.language);
  }

  /** 从后端加载完整设置并应用主题 / 语言副作用。 */
  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await settingsApi.getSettings();
      applySideEffects();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** 原子保存完整设置，成功后覆盖本地快照并重新应用副作用。 */
  async function save(next: Settings): Promise<void> {
    saving.value = true;
    error.value = null;
    try {
      await settingsApi.updateSettings(next);
      settings.value = next;
      applySideEffects();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      saving.value = false;
    }
  }

  /** 探测可用 Git（仅返回结果供 UI 展示，不写库）。 */
  async function detectGit(): Promise<GitDetectionResult> {
    detecting.value = true;
    error.value = null;
    try {
      return await settingsApi.detectGit();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      detecting.value = false;
    }
  }

  /** 校验并持久化用户指定的 Git 路径，成功后回填本地快照。 */
  async function setGitPath(path: string): Promise<GitDetectionResult> {
    detecting.value = true;
    error.value = null;
    try {
      const result = await settingsApi.setGitPath(path);
      // 持久化后回填路径，保持本地快照与库一致
      if (result.path !== undefined) {
        settings.value.git.gitExecutablePath = result.path;
      }
      return result;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      detecting.value = false;
    }
  }

  return {
    settings,
    loading,
    saving,
    detecting,
    error,
    general,
    git,
    network,
    externalTools,
    load,
    save,
    detectGit,
    setGitPath,
  };
});
