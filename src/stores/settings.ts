// =====================================================================
// 应用设置 store 骨架
// 后续 Phase 10 将填充：加载/保存设置、监听设置变更广播。
// =====================================================================

import { defineStore } from 'pinia';
import { ref } from 'vue';

import type { Settings } from '@/types/settings';

/**
 * 设置默认值。
 * 仅作为前端首次渲染前的占位，真正的默认值由后端 settings_service 提供。
 */
const DEFAULT_SETTINGS: Settings = {
  defaultRepoBaseDir: '',
  defaultCloneProtocol: 'https',
  directoryStrategy: 'by_owner',
  theme: 'auto',
  language: 'zh_cn',
  autoSyncOnStartup: false,
};

export const useSettingsStore = defineStore('settings', () => {
  /** 当前应用设置快照 */
  const settings = ref<Settings>({ ...DEFAULT_SETTINGS });
  /** 加载中标记 */
  const loading = ref(false);

  return {
    settings,
    loading,
  };
});
