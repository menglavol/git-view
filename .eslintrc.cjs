/**
 * ESLint 配置（CommonJS 形式，兼容 ESLint 8.x）。
 *
 * 规则目标：
 *   - 强制 TypeScript 严格风格（与 tsconfig.json 协同）
 *   - Vue 3 推荐规则
 *   - 禁止 console 调试输出（宪法 Principle I）
 *   - 与 Prettier 协作时不冲突格式化（仅做风格警告，不做格式化）
 *
 * CI 通过 `eslint . --ext .ts,.vue --max-warnings 0` 强制零警告。
 */

module.exports = {
  root: true,
  env: {
    browser: true,
    es2022: true,
    node: true,
  },
  parser: 'vue-eslint-parser',
  parserOptions: {
    parser: '@typescript-eslint/parser',
    ecmaVersion: 'latest',
    sourceType: 'module',
    extraFileExtensions: ['.vue'],
  },
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:vue/vue3-recommended',
    // 必须放在 extends 数组末尾：关闭所有与 Prettier 冲突的 ESLint 风格规则
    // 让 Prettier 独占格式化职责，ESLint 仅负责代码质量类规则
    'prettier',
  ],
  plugins: ['@typescript-eslint', 'vue'],
  rules: {
    // 宪法 Principle I — 严禁遗留调试输出
    'no-console': ['error', { allow: ['warn', 'error'] }],
    'no-debugger': 'error',

    // TypeScript 严格度
    '@typescript-eslint/no-unused-vars': [
      'error',
      { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
    ],
    '@typescript-eslint/no-explicit-any': 'warn',
    '@typescript-eslint/explicit-function-return-type': 'off',
    '@typescript-eslint/explicit-module-boundary-types': 'off',

    // Vue 风格
    'vue/multi-word-component-names': 'off', // 允许如 Accounts.vue 单词命名
    'vue/component-name-in-template-casing': ['error', 'PascalCase'],
    'vue/html-self-closing': [
      'error',
      {
        html: { void: 'always', normal: 'always', component: 'always' },
        svg: 'always',
        math: 'always',
      },
    ],
  },
  ignorePatterns: [
    'node_modules/',
    'dist/',
    'build/',
    'src-tauri/',
    '*.d.ts',
    'src/auto-imports.d.ts',
    'src/components.d.ts',
  ],
};
