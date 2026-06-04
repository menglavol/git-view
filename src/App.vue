<!--
  GitView 应用根组件。
  职责：仅承载 <router-view>，作为 Vue 组件树的最顶层容器。
  所有布局结构（顶栏、侧边栏、主内容区）下沉到 layouts/AppLayout.vue，
  便于后续在不同路由下切换布局（如登录页使用空白布局）。
  设计决策：根组件保持极简，避免在此处引入全局状态或副作用。
-->
<template>
  <!-- 路由出口：根据 URL 渲染对应的布局组件 -->
  <router-view />
</template>

<script setup lang="ts">
// 根组件在启动时加载一次持久化设置，并由 settings store 应用主题/语言副作用。
// 为何放在这里：主题与语言是全局外观，必须在任意页面渲染前就从持久化设置恢复；
// 否则应用每次都以默认（auto 主题 / 中文）启动，用户选过的深色主题要等进入设置页
// 才生效，违背 US7「设置重启后保持」的验收标准。这是根组件唯一允许的全局副作用。
import { onMounted } from 'vue';

import { useSettingsStore } from '@/stores/settings';

const settingsStore = useSettingsStore();

onMounted(() => {
  // 加载失败（如后端尚未就绪 / keyring 异常）时静默回退默认外观，不阻断应用渲染；
  // 用户进入设置页可手动重试。这里刻意吞掉错误，避免首屏因设置加载失败而白屏。
  void settingsStore.load().catch(() => {
    // 忽略：首屏沿用默认主题/语言
  });
});
</script>

<style>
/* 全局基础样式重置 */
/* 清除浏览器默认外边距，确保应用占满整个 Tauri 窗口 */
html,
body,
#app {
  margin: 0;
  padding: 0;
  height: 100%;
  /* 字体栈：优先使用系统原生字体，中文优先 PingFang / 微软雅黑 */
  font-family:
    -apple-system, BlinkMacSystemFont, 'PingFang SC', 'Microsoft YaHei', 'Segoe UI', Roboto,
    sans-serif;
}
</style>
