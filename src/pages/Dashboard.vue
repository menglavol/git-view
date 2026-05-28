<!--
  首页仪表盘页面（占位）。
  V1 MVP Phase 10 - T107 将在此实现 8 个统计卡片、最近打开仓库、最近任务等
  聚合视图。当前仅渲染标题占位以支撑路由可达性测试。
  核心交互：统计卡片网格、最近仓库快捷入口、最近克隆任务状态
-->
<template>
  <!-- 仪表盘页面容器 -->
  <div class="page-dashboard">
    <!-- 页面标题 -->
    <h1>首页</h1>
    <!-- 占位提示：说明功能将在后续 Phase 实现 -->
    <p class="placeholder"> 仪表盘内容将在 T107（Phase 10）实现。 </p>
  </div>
</template>

<script setup lang="ts">
// 仪表盘脚本占位。
// 后续将接入 accountStore / localRepositoryStore /
// cloneTaskStore / operationLogStore 进行聚合统计展示。
</script>

<style scoped>
/* 页面容器：统一内边距 */
.page-dashboard {
  padding: 8px;
}

/* 占位文案样式：次要文字颜色、较小字号 */
.placeholder {
  color: var(--el-text-color-secondary);
  font-size: 14px;
  margin-top: 12px;
}
</style>
