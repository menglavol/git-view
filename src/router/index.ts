/**
 * Vue Router 路由表。
 *
 * V1 MVP 业务路由全部包裹在 AppLayout 三段式布局之下；另有一个独立全屏的
 * 引导页 `/onboarding`（不套布局）。全局前置守卫实现首次启动的 Git 环境
 * 拦截（T109）：未检测到 git 时把用户导向引导页。
 */

import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';

import AppLayout from '@/layouts/AppLayout.vue';
import { useAppStore } from '@/stores/app';

const routes: RouteRecordRaw[] = [
  {
    // 首次启动引导 / 环境检查页（独立全屏，不套 AppLayout 侧边栏）
    path: '/onboarding',
    name: 'onboarding',
    component: () => import('@/pages/Onboarding.vue'),
    meta: { title: '环境检查' },
  },
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

/**
 * 全局前置守卫：首次启动的 Git 环境拦截（T109）。
 *
 * 未检测到可用 git 时，除「设置」与「引导页」外的所有路由都重定向到引导页，
 * 让用户先装好 git 或在设置里手动指定路径，避免进入依赖 git 的页面后处处报错。
 * git 已就绪却仍停在引导页时，自动放行回首页。
 */
router.beforeEach(async (to) => {
  const appStore = useAppStore();
  // 懒检测：仅首次真正调用后端，后续导航命中 promise 缓存几乎零开销。
  await appStore.ensureGitChecked();

  // 设置页必须放行：它是用户在「未检测到 git」时手动指定可执行路径的修复入口。
  const allowWithoutGit = to.name === 'onboarding' || to.name === 'settings';

  if (!appStore.gitReady && !allowWithoutGit) {
    return { name: 'onboarding' };
  }
  // git 已就绪却仍停留在引导页：无意义，放行回首页。
  if (appStore.gitReady && to.name === 'onboarding') {
    return { name: 'dashboard' };
  }
  return true;
});

export default router;
