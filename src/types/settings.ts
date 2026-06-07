// =====================================================================
// 应用设置类型
// 与 src-tauri/src/models/settings.rs 一一对应；按四组拆分以与后端
// key/value 分组存储、与设置页 Tab 结构对齐。字段名 camelCase，
// 对应后端各 struct 的 #[serde(rename_all = "camelCase")]。
// =====================================================================

/** 默认克隆协议。 */
export type CloneProtocol = 'https' | 'ssh';

/** 仓库目录组织策略。 */
export type DirectoryStrategy = 'flat' | 'by_owner' | 'by_platform_and_owner';

/** 界面主题。 */
export type Theme = 'auto' | 'light' | 'dark';

/** 界面语言。 */
export type Language = 'zh_cn' | 'en_us';

/** 默认 pull 策略（对应 git pull 行为）。 */
export type PullStrategy = 'ff_only' | 'rebase' | 'merge';

/** 默认 push 策略（对应 git push.default）。 */
export type PushStrategy = 'simple' | 'current' | 'upstream';

/** 通用设置组：目录、协议、并发、外观等高频项。 */
export interface GeneralSettings {
  /** 默认仓库根目录（批量克隆时预填） */
  defaultRepoBaseDir: string;
  /** 默认克隆协议 */
  defaultCloneProtocol: CloneProtocol;
  /** 默认并发克隆数（改动需重启生效） */
  defaultConcurrency: number;
  /** 目录组织策略 */
  directoryStrategy: DirectoryStrategy;
  /** 界面主题 */
  theme: Theme;
  /** 界面语言 */
  language: Language;
  /** 启动时是否自动打开上次的仓库 */
  openLastRepoOnStartup: boolean;
  /** 启动时是否自动检查本地仓库状态 */
  autoCheckRepoStatus: boolean;
}

/** Git 设置组：可执行路径、提交身份、默认网络策略。 */
export interface GitSettings {
  /** 自定义 git 可执行路径（缺省表示用 PATH 自动探测） */
  gitExecutablePath?: string;
  /** 提交身份 user.name（缺省表示沿用 git 全局配置） */
  userName?: string;
  /** 提交身份 user.email（缺省表示沿用 git 全局配置） */
  userEmail?: string;
  /** 默认 pull 策略 */
  defaultPullStrategy: PullStrategy;
  /** 默认 push 策略 */
  defaultPushStrategy: PushStrategy;
}

/** 网络设置组：代理与超时。 */
export interface NetworkSettings {
  /** HTTP 代理 URL（缺省表示不显式设置） */
  httpProxy?: string;
  /** HTTPS 代理 URL（缺省表示不显式设置） */
  httpsProxy?: string;
  /** 是否跟随系统代理（开启时忽略上面两项） */
  useSystemProxy: boolean;
  /** API 请求超时（秒） */
  apiTimeoutSecs: number;
  /** 克隆超时（秒） */
  cloneTimeoutSecs: number;
}

/** 外部工具设置组：「在外部工具打开」时调用的命令。 */
export interface ExternalToolsSettings {
  /** 默认编辑器命令（如 code / cursor） */
  editorCommand?: string;
  /** 默认终端命令 */
  terminalCommand?: string;
  /** 默认文件管理器命令 */
  fileManagerCommand?: string;
}

/** 应用设置聚合快照（与后端 get_settings 返回结构一致）。 */
export interface Settings {
  /** 通用组 */
  general: GeneralSettings;
  /** Git 组 */
  git: GitSettings;
  /** 网络组 */
  network: NetworkSettings;
  /** 外部工具组 */
  externalTools: ExternalToolsSettings;
}

/**
 * 日志目录占用统计（get_log_stats 返回，对应后端 LogStats）。
 *
 * 供设置页展示日志目录路径、占用大小与文件数，作为「清理历史日志」的决策依据。
 */
export interface LogStats {
  /** 日志目录绝对路径 */
  dir: string;
  /** 目录内所有文件的总字节数 */
  sizeBytes: number;
  /** 日志文件个数 */
  fileCount: number;
}

/**
 * 清理历史日志的结果（clear_old_logs 返回，对应后端 ClearLogsResult）。
 *
 * 保留当天日志、删除更早的滚动文件后，回报删除数量与释放空间。
 */
export interface ClearLogsResult {
  /** 删除的文件数 */
  removed: number;
  /** 释放的字节数 */
  freedBytes: number;
}

/**
 * 当前数据目录信息（get_data_dir 返回，对应后端 DataDirInfo）。
 *
 * current 为当前生效的数据目录；previous 为迁移后保留的旧目录（无则为 null）。
 */
export interface DataDirInfo {
  /** 当前生效的数据目录绝对路径 */
  current: string;
  /** 迁移后保留的旧目录路径（无则为 null） */
  previous: string | null;
}

/**
 * 数据目录迁移结果（migrate_data_dir 返回，对应后端 MigrateResult）。
 */
export interface MigrateResult {
  /** 迁移后的新数据目录绝对路径 */
  newDir: string;
  /** 被保留的旧数据目录绝对路径（待手动删除） */
  previousDir: string;
}

/**
 * 旧数据目录占用统计（get_old_data_dir 返回，对应后端 OldDataDir）。
 */
export interface OldDataDir {
  /** 旧目录绝对路径 */
  dir: string;
  /** 旧目录内所有文件的总字节数 */
  sizeBytes: number;
  /** 旧目录文件个数 */
  fileCount: number;
}

/**
 * Git 环境检测结果（detect_git / set_git_path 返回）。
 *
 * 以 `found` 表达「是否检测到 git」：未装 git 是可处理的正常状态，
 * 前端据此引导安装或手动指定路径，而非按异常处理。
 */
export interface GitDetectionResult {
  /** 是否检测到可用的 git */
  found: boolean;
  /** git 可执行路径 */
  path?: string;
  /** git --version 解析出的版本号 */
  version?: string;
  /** 读取到的 user.name */
  userName?: string;
  /** 读取到的 user.email */
  userEmail?: string;
}
