/**
 * Vue Router 路由表。
 *
 * V1 MVP 共 8 个路由（含仓库详情动态路由），全部包裹在 AppLayout
 * 三段式布局之下。具体的路由守卫（如未安装 Git 时引导到 Settings）
 * 由 T109（Phase 10）在后续阶段补充。
 */

import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';

import AppLayout from '@/layouts/AppLayout.vue';

const routes: RouteRecordRaw[] = [
  {
    // 默认布局：含侧边栏 + 顶栏 + 主内容区
    path: '/',
    component: AppLayout,
    children: [
      // 首页仪表盘
      {
        path: '',
        name: 'dashboard',
        component: () => import('@/pages/Dashboard.vue'),
        meta: { title: '首页' },
      },
      // 账号管理
      {
        path: 'accounts',
        name: 'accounts',
        component: () => import('@/pages/Accounts.vue'),
        meta: { title: '账号管理' },
      },
      // 远程仓库
      {
        path: 'remote-repositories',
        name: 'remote-repositories',
        component: () => import('@/pages/RemoteRepositories.vue'),
        meta: { title: '远程仓库' },
      },
      // Clone 中心
      {
        path: 'clone-center',
        name: 'clone-center',
        component: () => import('@/pages/CloneCenter.vue'),
        meta: { title: 'Clone 中心' },
      },
      // 本地仓库列表
      {
        path: 'local-repositories',
        name: 'local-repositories',
        component: () => import('@/pages/LocalRepositories.vue'),
        meta: { title: '本地仓库' },
      },
      // 单仓库详情/工作区（动态参数 id 对应 LocalRepository.id）
      {
        path: 'repositories/:id',
        name: 'repository-detail',
        component: () => import('@/pages/RepositoryDetail.vue'),
        meta: { title: '仓库工作区' },
      },
      // 操作日志
      {
        path: 'logs',
        name: 'logs',
        component: () => import('@/pages/Logs.vue'),
        meta: { title: '操作日志' },
      },
      // 设置
      {
        path: 'settings',
        name: 'settings',
        component: () => import('@/pages/Settings.vue'),
        meta: { title: '设置' },
      },
    ],
  },
];

// 桌面应用环境使用 hash 模式，避免 file:// 协议下路径解析问题
const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
