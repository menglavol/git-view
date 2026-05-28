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
// 根组件目前无业务逻辑；保留 script setup 块以便未来挂接全局事件监听。
// 后续可能在此处监听 Tauri 窗口事件（如关闭前确认、深度链接等）。
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
