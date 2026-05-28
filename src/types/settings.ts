// =====================================================================
// 应用设置类型
// 与 src-tauri/src/models/settings.rs 对齐。
// =====================================================================

/** 默认克隆协议。 */
export type CloneProtocol = 'https' | 'ssh';

/** 仓库目录组织策略。 */
export type DirectoryStrategy = 'flat' | 'by_owner' | 'by_platform_and_owner';

/** 界面主题。 */
export type Theme = 'auto' | 'light' | 'dark';

/** 界面语言。 */
export type Language = 'zh_cn' | 'en_us';

/** 应用设置聚合。 */
export interface Settings {
  defaultRepoBaseDir: string;
  defaultCloneProtocol: CloneProtocol;
  directoryStrategy: DirectoryStrategy;
  theme: Theme;
  language: Language;
  gitExecutablePath?: string;
  httpProxy?: string;
  httpsProxy?: string;
  autoSyncOnStartup: boolean;
}
