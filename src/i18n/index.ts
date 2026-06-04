// =====================================================================
// 国际化（i18n）入口
// V1 范围严格受控：命名空间仅 settings.* 与 common.*，且只在 Settings.vue
// 子树通过 useI18n() 接入；其余页面零改动，避免大范围文案迁移。
// 组合式模式（legacy: false）以契合项目 setup-style 组件。
// =====================================================================

import { createI18n } from 'vue-i18n';

import type { Language } from '@/types/settings';
import { en } from './en';
import { zh } from './zh';

// 应用语言（后端枚举 'zh_cn' | 'en_us'）到 vue-i18n locale 标识的映射。
// 后端用 snake_case 语言码，vue-i18n 习惯用 BCP-47（zh-CN / en-US），故做转换。
// 值类型收窄为字面量联合（而非 string）：vue-i18n 的 locale 依 messages 的键
// 推断为 'zh-CN' | 'en-US'，若此处用 string 赋值会触发类型不匹配。
const LOCALE_MAP: Record<Language, 'zh-CN' | 'en-US'> = {
  zh_cn: 'zh-CN',
  en_us: 'en-US',
};

// 创建 i18n 实例：默认中文，回退中文（英文文案缺漏时不至于露出 key）。
export const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zh,
    'en-US': en,
  },
});

/**
 * 把应用语言设置同步到 i18n locale。
 *
 * 由 settings store 在加载 / 保存设置后调用，实现「设置里选语言 → 界面切换」。
 */
export function setI18nLocale(language: Language): void {
  i18n.global.locale.value = LOCALE_MAP[language];
}
