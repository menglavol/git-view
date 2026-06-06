/**
 * Vite 构建配置。
 *
 * 核心职责：
 *   1. 注册 Vue 单文件组件支持。
 *   2. 配置 Element Plus 按需自动引入（unplugin-auto-import +
 *      unplugin-vue-components），避免全量打包造成体积膨胀。
 *   3. 适配 Tauri 桌面环境：固定 dev 端口、禁用清屏、Hot Reload 监听。
 *
 * 注意：Tauri 期望前端 dev server 在固定端口（默认 1420）；修改时需要同步
 *      调整 src-tauri/tauri.conf.json 的 build.devUrl 字段。
 */

import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import AutoImport from 'unplugin-auto-import/vite';
import Components from 'unplugin-vue-components/vite';
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers';
import path from 'node:path';

// Tauri 期望前端运行在固定端口，便于桌面壳子加载
const TAURI_DEV_PORT = 1420;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [
    // Vue 单文件组件支持
    vue(),
    // 自动引入 Vue/Pinia/VueRouter 等常用 API
    AutoImport({
      imports: ['vue', 'vue-router', 'pinia'],
      resolvers: [ElementPlusResolver()],
      dts: 'src/auto-imports.d.ts',
    }),
    // 自动按需注册 Element Plus 组件，无需手动 import
    Components({
      resolvers: [ElementPlusResolver()],
      dts: 'src/components.d.ts',
    }),
  ],
  // 路径别名：@ 指向 src/
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  // vue-i18n 生产构建特性开关。
  // 关键是 __INTLIFY_JIT_COMPILATION__：本项目 messages 以 TS 对象传入（未走
  // 预编译插件），生产构建默认不含 message compiler，会导致 t() 渲染为空
  // （设置页全是 t()，dev 正常但打包后整片空白）。开启 JIT 让运行时（含生产）
  // 也能编译 messages；其余三个 flag 用于消除 vue-i18n 的特性未定义告警。
  define: {
    __VUE_I18N_FULL_INSTALL__: true,
    __VUE_I18N_LEGACY_API__: false,
    __INTLIFY_PROD_DEVTOOLS__: false,
    __INTLIFY_JIT_COMPILATION__: true,
  },
  // 防止 Vite 屏蔽掉 Rust 编译错误，保留 Tauri 输出
  clearScreen: false,
  // 开发服务器配置
  server: {
    port: TAURI_DEV_PORT,
    strictPort: true, // 端口被占用时直接报错，避免无意切换
    host: '0.0.0.0',
    watch: {
      // 忽略 Rust 后端目录，避免无意义重启
      ignored: ['**/src-tauri/**'],
    },
  },
  // 构建产物配置
  build: {
    target: 'es2021', // 与 Tauri 内嵌 WebView 兼容
    minify: 'esbuild',
    sourcemap: false,
  },
  // 环境变量前缀
  envPrefix: ['VITE_', 'TAURI_'],
}));
