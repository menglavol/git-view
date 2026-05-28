<!--
  设置中心页面（占位）。
  V1 MVP US7 - T102 将在此实现通用 / Git / 网络 / 外部工具 / 账号与安全
  共 5 个 Tab 的设置界面。当前仅渲染标题占位以支撑路由可达性测试。
  核心交互：Tab 切换、表单编辑、保存确认、Git 路径检测
-->
<template>
  <!-- 设置页面容器 -->
  <div class="page-settings">
    <!-- 页面标题 -->
    <h1>设置</h1>
    <!-- 占位提示：说明功能将在后续 Phase 实现 -->
    <p class="placeholder"> 设置中心将在 T102（Phase 9 US7）实现。 </p>
  </div>
</template>

<script setup lang="ts">
// 设置页面脚本占位。
// 后续将接入 settingsStore 与 get_settings / update_settings 命令，
// 提供通用设置、Git 配置、网络代理、外部工具路径、安全选项等功能。
</script>

<style scoped>
/* 页面容器：统一内边距 */
.page-settings {
  padding: 8px;
}

/* 占位文案样式：次要文字颜色、较小字号 */
.placeholder {
  color: var(--el-text-color-secondary);
  font-size: 14px;
  margin-top: 12px;
}
</style>
