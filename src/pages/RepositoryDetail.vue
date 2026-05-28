<!--
  单仓库详情/工作区页面（占位）。
  V1 MVP US5 - T082 将在此实现仓库工作区三栏布局：文件变更列表 / Diff /
  Commit 面板，及 Fetch/Pull/Push/分支切换等 Git 工作流。
  当前仅渲染标题占位以支撑路由可达性测试。

  路由参数 id 对应 LocalRepository.id；通过 useRoute().params.id 获取。
  核心交互：文件树、Diff 视图、Commit 表单、分支选择器、操作按钮组
-->
<template>
  <!-- 仓库工作区页面容器 -->
  <div class="page-repository-detail">
    <!-- 页面标题 -->
    <h1>仓库工作区</h1>
    <!-- 占位提示：说明功能将在后续 Phase 实现 -->
    <p class="placeholder"> 单仓库 Git 工作流将在 T082（Phase 7 US5）实现。 </p>
    <!-- 展示当前路由参数，验证动态路由正常工作 -->
    <p class="meta"> 当前仓库 ID（路由参数）：{{ repoId }} </p>
  </div>
</template>

<script setup lang="ts">
/**
 * 仓库详情页面脚本占位：
 *   - 读取路由参数 id 作为 LocalRepository.id
 *   - 后续将根据 id 加载 git_status / git_diff / git_list_branches 等数据
 *   - 提供文件变更列表、Diff 展示、Commit 提交等核心 Git 工作流
 */
import { computed } from 'vue';
import { useRoute } from 'vue-router';

// 获取当前路由实例
const route = useRoute();
// 路由参数 id：对应 local_repositories 表中的主键 id
const repoId = computed(() => route.params.id);
</script>

<style scoped>
/* 页面容器：统一内边距 */
.page-repository-detail {
  padding: 8px;
}

/* 占位文案与元信息样式：次要文字颜色、较小字号 */
.placeholder,
.meta {
  color: var(--el-text-color-secondary);
  font-size: 14px;
  margin-top: 12px;
}
</style>
