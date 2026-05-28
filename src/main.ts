/**
 * GitView 前端应用入口。
 *
 * 主要职责：
 *   1. 创建 Vue 应用实例。
 *   2. 注册 Pinia 全局状态管理。
 *   3. 注册 Vue Router 完成页面路由。
 *   4. 注册 Element Plus（按需引入由 vite.config.ts 中的 unplugin 自动处理，
 *      仅需手动引入暗色主题 CSS 变量以支持后续主题切换）。
 *   5. 挂载根组件到 #app。
 *
 * 注：日志/Tauri API 在各模块内按需引入；此处不做全局副作用调用。
 */

import { createApp } from 'vue';
import { createPinia } from 'pinia';

import App from './App.vue';
import router from './router';

// Element Plus 主题样式：默认浅色 + 暗色覆盖（具体切换由 settings 模块控制）
import 'element-plus/dist/index.css';
import 'element-plus/theme-chalk/dark/css-vars.css';

// 创建应用实例
const app = createApp(App);

// 注册全局插件
app.use(createPinia()); // 状态管理
app.use(router); // 路由

// 挂载到 DOM
app.mount('#app');
