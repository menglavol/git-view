<!--
  应用主布局组件。
  采用经典桌面工具三段式布局：
    - el-container 上下：顶栏 (el-header) + 主体 (el-container 左右)
    - 主体左右：侧边导航 (el-aside) + 内容区 (el-main with router-view)

  侧边栏导航列出 7 个一级菜单项，与 src/router/index.ts 中的命名路由对应。
  顶栏占位区域预留：左侧返回按钮、当前账号显示、全局搜索、同步按钮、设置入口；
  这些组件将在后续 Phase 由对应 User Story 任务填充。
-->

<template>
  <ElContainer class="app-layout">
    <!-- ============ 顶栏 ============ -->
    <ElHeader class="app-header">
      <div class="header-left">
        <span class="app-title">GitView</span>
      </div>
      <div class="header-right">
        <AccountSwitcher />
        <!-- 全局搜索 / 同步按钮 / 设置入口占位 -->
      </div>
    </ElHeader>

    <!-- ============ 主体：侧边栏 + 内容区 ============ -->
    <ElContainer class="app-body">
      <!-- 左侧导航 -->
      <ElAside width="200px" class="app-sidebar">
        <ElMenu :default-active="activeRoute" router class="sidebar-menu">
          <ElMenuItem index="/">
            <span>首页</span>
          </ElMenuItem>
          <ElMenuItem index="/accounts">
            <span>账号管理</span>
          </ElMenuItem>
          <ElMenuItem index="/remote-repositories">
            <span>远程仓库</span>
          </ElMenuItem>
          <ElMenuItem index="/clone-center">
            <span>Clone 中心</span>
          </ElMenuItem>
          <ElMenuItem index="/local-repositories">
            <span>本地仓库</span>
          </ElMenuItem>
          <ElMenuItem index="/logs">
            <span>操作日志</span>
          </ElMenuItem>
          <ElMenuItem index="/settings">
            <span>设置</span>
          </ElMenuItem>
        </ElMenu>
      </ElAside>

      <!-- 主内容区：渲染当前路由对应的页面 -->
      <ElMain class="app-main">
        <RouterView />
      </ElMain>
    </ElContainer>
  </ElContainer>
</template>

<script setup lang="ts">
/**
 * AppLayout 组件脚本：
 *   - 读取当前路由路径，用于侧边菜单的高亮（default-active）。
 *   - 顶栏挂载 AccountSwitcher 提供全局账号切换入口。
 */
import { computed } from 'vue';
import { useRoute } from 'vue-router';

import AccountSwitcher from '@/components/account/AccountSwitcher.vue';

const route = useRoute();
// 当前激活路由：使用完整 path 与菜单 index 对齐
const activeRoute = computed(() => route.path);
</script>

<style scoped>
/* 全局容器：占满视口 */
.app-layout {
  height: 100vh;
  width: 100vw;
  overflow: hidden;
}

/* 顶栏：固定高度、底部细边框分隔 */
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  padding: 0 16px;
  border-bottom: 1px solid var(--el-border-color-light);
  background-color: var(--el-bg-color);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.app-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* 主体容器：占用顶栏以下的全部高度 */
.app-body {
  height: calc(100vh - 56px);
}

/* 侧边栏：固定宽度、右侧细边框 */
.app-sidebar {
  border-right: 1px solid var(--el-border-color-light);
  background-color: var(--el-bg-color);
}

.sidebar-menu {
  border-right: none;
  height: 100%;
}

/* 主内容区：剩余宽度，滚动溢出 */
.app-main {
  background-color: var(--el-bg-color-page);
  overflow: auto;
  padding: 16px;
}
</style>
