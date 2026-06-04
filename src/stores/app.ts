// =====================================================================
// 应用全局 store
// 承载与具体业务无关的应用级状态：初始化进度、侧栏折叠、主题等。
// 主题应用逻辑集中在此（而非散落到各页面），由 settings store 在
// load/save 后调用 applyTheme 驱动。
// =====================================================================

import { defineStore } from 'pinia';
import { ref } from 'vue';

import type { Theme } from '@/types/settings';

export const useAppStore = defineStore('app', () => {
  /** 应用是否已完成启动初始化（后端 setup 完成后置 true） */
  const ready = ref(false);
  /** 侧边栏是否折叠 */
  const sidebarCollapsed = ref(false);
  /** 全局通知中心未读数量（占位，后续可对接 Tauri event） */
  const unreadNotifications = ref(0);

  /** 当前生效主题（与 settings.general.theme 同步） */
  const theme = ref<Theme>('auto');

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
    applyTheme,
  };
});
