// =====================================================================
// 应用全局 store
// 承载与具体业务无关的应用级状态：初始化进度、侧栏折叠、主题等。
// 主题应用逻辑集中在此（而非散落到各页面），由 settings store 在
// load/save 后调用 applyTheme 驱动。
// =====================================================================

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { settingsApi } from '@/api/settings.api';
import type { GitDetectionResult, Theme } from '@/types/settings';

export const useAppStore = defineStore('app', () => {
  /** 应用是否已完成启动初始化（后端 setup 完成后置 true） */
  const ready = ref(false);
  /** 侧边栏是否折叠 */
  const sidebarCollapsed = ref(false);
  /** 全局通知中心未读数量（占位，后续可对接 Tauri event） */
  const unreadNotifications = ref(0);

  /** 当前生效主题（与 settings.general.theme 同步） */
  const theme = ref<Theme>('auto');

  // ---- Git 环境检测（T109 首次启动引导）----
  // 应用启动时必须确认本机有可用的 git，否则克隆 / 工作流等核心功能全部不可用；
  // 这里把检测状态升到 app 级，供路由守卫拦截、引导页展示共用。
  /** 最近一次 Git 环境检测结果；null 表示尚未检测。 */
  const gitDetection = ref<GitDetectionResult | null>(null);
  /** 是否已完成至少一次 Git 检测（区分「未检测」与「检测到未安装」）。 */
  const gitChecked = ref(false);
  /** Git 是否就绪（检测到可用 git）。 */
  const gitReady = computed(() => gitDetection.value?.found ?? false);

  // 检测去重：首屏路由守卫可能在多次导航中触发，用 promise 缓存保证只真正检测一次。
  let gitCheckPromise: Promise<void> | null = null;

  // 执行一次检测并落地结果。后端探测失败已在 command 层归一为 found=false，
  // 这里再兜一层 catch：即便 IPC 整体异常，也保守视为「未就绪」交引导页处理，
  // 绝不让检测异常把整个应用卡在白屏。
  async function runGitDetection(): Promise<void> {
    try {
      gitDetection.value = await settingsApi.detectGit();
    } catch {
      gitDetection.value = { found: false };
    } finally {
      gitChecked.value = true;
    }
  }

  /**
   * 确保 Git 已被检测过（懒执行，仅首次真正调用后端）。
   * 路由守卫每次导航都会调用，靠 promise 缓存避免重复 IPC 请求。
   */
  function ensureGitChecked(): Promise<void> {
    if (!gitCheckPromise) {
      gitCheckPromise = runGitDetection();
    }
    return gitCheckPromise;
  }

  /** 强制重新检测（引导页「重新检测」、或设置里改完 git 路径后调用）。 */
  function recheckGit(): Promise<void> {
    gitCheckPromise = runGitDetection();
    return gitCheckPromise;
  }

  // 'auto' 模式下监听系统配色变化的清理函数；切换主题时调用以避免重复注册。
  let systemThemeCleanup: (() => void) | null = null;

  /**
   * 把 dark class 同步到根元素。
   *
   * Element Plus 暗色主题靠 `html.dark` 类生效（main.ts 已引入暗色 css-vars），
   * 故主题切换归结为增删根元素的 dark class。
   */
  function setDarkClass(dark: boolean): void {
    const el = document.documentElement;
    if (dark) {
      el.classList.add('dark');
    } else {
      el.classList.remove('dark');
    }
  }

  /**
   * 应用主题。
   *
   * - light / dark：直接设置 html.dark class
   * - auto：跟随系统 prefers-color-scheme，并监听其变化实时切换
   *
   * 每次调用先清理上一次的 auto 监听，避免从 auto 切走后仍残留监听、
   * 或重复进入 auto 造成多重注册导致内存泄漏。
   */
  function applyTheme(next: Theme): void {
    theme.value = next;
    if (systemThemeCleanup) {
      systemThemeCleanup();
      systemThemeCleanup = null;
    }
    if (next === 'auto') {
      const mql = window.matchMedia('(prefers-color-scheme: dark)');
      setDarkClass(mql.matches);
      const onChange = (e: MediaQueryListEvent): void => {
        setDarkClass(e.matches);
      };
      mql.addEventListener('change', onChange);
      systemThemeCleanup = (): void => {
        mql.removeEventListener('change', onChange);
      };
    } else {
      setDarkClass(next === 'dark');
    }
  }

  return {
    ready,
    sidebarCollapsed,
    unreadNotifications,
    theme,
    gitDetection,
    gitChecked,
    gitReady,
    applyTheme,
    ensureGitChecked,
    recheckGit,
  };
});
