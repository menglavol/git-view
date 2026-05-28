// =====================================================================
// 应用全局 store 骨架
// 承载与具体业务无关的应用级状态：初始化进度、全局错误提示、侧栏折叠等。
// =====================================================================

import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useAppStore = defineStore('app', () => {
  /** 应用是否已完成启动初始化（后端 setup 完成后置 true） */
  const ready = ref(false);
  /** 侧边栏是否折叠 */
  const sidebarCollapsed = ref(false);
  /** 全局通知中心未读数量（占位，后续可对接 Tauri event） */
  const unreadNotifications = ref(0);

  return {
    ready,
    sidebarCollapsed,
    unreadNotifications,
  };
});
