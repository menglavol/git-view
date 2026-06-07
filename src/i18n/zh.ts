// =====================================================================
// 简体中文文案（settings.* / common.*）
// 仅供 Settings.vue 子树使用。结构需与 en.ts 保持一致，
// 以便 vue-i18n 以此为 schema 校验另一语言的缺漏。
// =====================================================================

export const zh = {
  // 通用文案：跨多个表单复用的按钮 / 状态词
  common: {
    save: '保存', // 设置页右上角主按钮
    saving: '保存中…', // 保存请求进行中的按钮态
    cancel: '取消', // 对话框取消按钮
    reset: '重置', // 丢弃未保存改动
    confirm: '确认', // 通用确认
    seconds: '秒', // 超时输入框后缀单位
    browse: '浏览', // 目录/文件选择按钮
  },
  settings: {
    // 页面级文案：标题与保存 / 加载结果提示
    title: '设置', // 侧边栏菜单项 + 页面主标题
    saveSuccess: '设置已保存', // 保存成功 toast
    saveFailed: '保存失败', // 保存失败 toast 前缀
    loadFailed: '加载设置失败', // 加载失败 toast 前缀
    // 五个 Tab 的标签
    tabs: {
      general: '通用',
      git: 'Git',
      network: '网络',
      externalTools: '外部工具',
      accountSecurity: '账号与安全',
    },
    // 通用 Tab 各字段标签
    general: {
      repoBaseDir: '默认仓库根目录',
      repoBaseDirPlaceholder: '批量克隆时预填的根目录',
      cloneProtocol: '默认克隆协议',
      concurrency: '默认并发克隆数',
      concurrencyHint: '改动需重启应用后生效',
      directoryStrategy: '目录组织方式',
      theme: '主题',
      language: '语言',
      openLastRepo: '启动时打开上次的仓库',
      autoCheckStatus: '启动时自动检查仓库状态',
    },
    // 克隆协议选项
    protocol: {
      https: 'HTTPS',
      ssh: 'SSH',
    },
    // 目录组织方式选项
    strategy: {
      flat: '扁平（根目录/仓库名）',
      byOwner: '按所有者（根目录/owner/仓库名）',
      byPlatformAndOwner: '按平台与所有者（根目录/平台/owner/仓库名）',
    },
    // 主题选项
    theme: {
      auto: '跟随系统',
      light: '浅色',
      dark: '深色',
    },
    // 语言选项
    language: {
      zhCn: '简体中文',
      enUs: 'English',
    },
    // Git Tab 各字段标签
    git: {
      executablePath: 'Git 可执行文件路径',
      executablePathPlaceholder: '留空则自动从 PATH 探测',
      detect: '检测 Git',
      detecting: '检测中…',
      detected: '已检测到',
      notDetected: '未检测到可用的 Git',
      version: '版本',
      userName: '提交身份 user.name',
      userEmail: '提交身份 user.email',
      pullStrategy: '默认 Pull 策略',
      pushStrategy: '默认 Push 策略',
    },
    // Pull 策略选项
    pull: {
      ffOnly: '仅快进（--ff-only）',
      rebase: '变基（--rebase）',
      merge: '合并（允许 merge commit）',
    },
    // Push 策略选项
    push: {
      simple: 'simple（仅当前分支到同名上游）',
      current: 'current（当前分支到同名远端）',
      upstream: 'upstream（推送到已配置上游）',
    },
    // 网络 Tab 各字段标签
    network: {
      httpProxy: 'HTTP 代理',
      httpsProxy: 'HTTPS 代理',
      proxyPlaceholder: '如 http://127.0.0.1:7890',
      useSystemProxy: '跟随系统代理',
      useSystemProxyHint: '开启后忽略上面手填的代理地址',
      apiTimeout: 'API 请求超时',
      cloneTimeout: '克隆超时',
    },
    // 外部工具 Tab 各字段标签
    externalTools: {
      editor: '默认编辑器命令',
      editorPlaceholder: '如 code、cursor',
      terminal: '默认终端命令',
      fileManager: '默认文件管理器命令',
    },
    // 日志与存储：日志目录占用展示与清理（通用 Tab 底部）
    storage: {
      sectionTitle: '日志与存储', // 分区小标题
      logDir: '日志目录', // 只读路径标签
      logUsage: '日志占用', // 占用大小标签
      logFileCount: '日志文件数', // 文件数标签
      fileCountUnit: '个文件', // 文件数单位后缀
      refresh: '刷新', // 重新统计占用
      clearLogs: '清理历史日志', // 清理按钮
      clearing: '清理中…', // 清理进行中按钮态
      clearConfirmTitle: '清理历史日志', // 二次确认标题
      clearConfirmMessage: '将删除当天之前的所有日志文件，仅保留当天日志。此操作不可恢复。', // 二次确认正文
      clearSuccess: '已清理 {count} 个日志文件，释放 {size}', // 成功 toast
      noOldLogs: '没有可清理的历史日志', // 无可删文件时提示
      clearFailed: '清理日志失败', // 失败 toast 前缀
      loadStatsFailed: '读取日志占用失败', // 统计失败 toast 前缀
      // 应用数据目录（迁移 / 删除旧目录）
      dataDirTitle: '应用数据目录', // 分区小标题
      dataDirCurrent: '当前数据目录', // 只读路径标签
      dataDirChange: '更改目录', // 选新目录按钮
      migrateConfirmTitle: '迁移数据目录', // 迁移二次确认标题
      migrateConfirmMessage:
        '将把当前数据库与日志复制到「{dir}」。迁移后需重启应用才生效，且重启前的改动不会进入新目录。是否继续？', // 迁移二次确认正文
      migrateSuccessTitle: '迁移完成', // 成功弹窗标题
      migrateSuccessMessage:
        '数据已复制到「{dir}」，需重启应用使新目录生效。旧目录已保留，可稍后手动删除。', // 成功弹窗正文
      migrateFailed: '迁移失败', // 失败 toast 前缀
      restartNow: '立即重启', // 成功弹窗确认按钮
      restartLater: '稍后', // 成功弹窗取消按钮
      loadDataDirFailed: '读取数据目录失败', // 加载失败 toast 前缀
      oldDir: '旧数据目录', // 旧目录行标签
      deleteOldDir: '删除旧目录', // 删除旧目录按钮
      deleteOldConfirmTitle: '删除旧数据目录', // 删除二次确认标题
      deleteOldConfirmMessage: '将永久删除旧数据目录「{dir}」，此操作不可恢复。确认删除？', // 删除二次确认正文
      deleteOldSuccess: '旧数据目录已删除', // 删除成功 toast
      deleteOldFailed: '删除旧目录失败', // 删除失败 toast 前缀
    },
    // 账号与安全 Tab：凭据状态与重新验证 / 删除凭据流程
    security: {
      description: '此处仅显示凭据是否存在，绝不显示 Token 明文。', // Tab 顶部安全声明
      noAccounts: '暂无账号，请先在「账号管理」添加。', // 无账号空态
      columnAccount: '账号',
      columnPlatform: '平台',
      columnCredential: '凭据状态',
      columnActions: '操作',
      credStored: '已存储', // 绿色 Tag：keyring 中存在凭据
      credMissing: '凭据缺失', // 红色 Tag：keyring 中无凭据
      credMissingHint: 'Token 已从安全存储中丢失，请用「重新验证」修复。', // 缺失 Tag 的 tooltip
      checking: '检查中…', // 凭据存在性查询进行中
      revalidate: '重新验证',
      deleteCredential: '删除凭据',
      // 重新验证对话框
      revalidateTitle: '重新验证账号凭据',
      tokenLabel: '新的 Token',
      tokenPlaceholder: '粘贴新的访问令牌（Token）',
      testAndSave: '测试并保存',
      revalidateSuccess: '凭据已更新',
      // 删除凭据的危险确认
      deleteConfirmTitle: '删除账号凭据',
      deleteConfirmMessage: '将从系统密钥库删除该账号的 Token，但保留账号元数据。',
      deleteConfirmHint: '删除后该账号无法同步 / 克隆，需重新验证才能恢复。',
      deleteSuccess: '凭据已删除',
    },
  },
};
