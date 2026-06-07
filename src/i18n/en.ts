// =====================================================================
// 英文文案（settings.* / common.*）
// V1 占位翻译：结构与 zh.ts 严格一致，缺漏项由 fallbackLocale 兜底为中文。
// 注释保持中文以说明分组用途，便于后续补全 / 校对。
// =====================================================================

export const en = {
  // 通用文案：跨多个表单复用的按钮 / 状态词
  common: {
    save: 'Save', // 设置页右上角主按钮
    saving: 'Saving…', // 保存请求进行中的按钮态
    cancel: 'Cancel', // 对话框取消按钮
    reset: 'Reset', // 丢弃未保存改动
    confirm: 'Confirm', // 通用确认
    seconds: 's', // 超时输入框后缀单位
    browse: 'Browse', // 目录/文件选择按钮
  },
  settings: {
    // 页面级文案：标题与保存 / 加载结果提示
    title: 'Settings', // 侧边栏菜单项 + 页面主标题
    saveSuccess: 'Settings saved', // 保存成功 toast
    saveFailed: 'Failed to save', // 保存失败 toast 前缀
    loadFailed: 'Failed to load settings', // 加载失败 toast 前缀
    // 五个 Tab 的标签
    tabs: {
      general: 'General',
      git: 'Git',
      network: 'Network',
      externalTools: 'External Tools',
      accountSecurity: 'Accounts & Security',
    },
    // 通用 Tab 各字段标签
    general: {
      repoBaseDir: 'Default repository root',
      repoBaseDirPlaceholder: 'Pre-filled root for batch clone',
      cloneProtocol: 'Default clone protocol',
      concurrency: 'Default clone concurrency',
      concurrencyHint: 'Takes effect after restart',
      directoryStrategy: 'Directory layout',
      theme: 'Theme',
      language: 'Language',
      openLastRepo: 'Open last repository on startup',
      autoCheckStatus: 'Auto-check repository status on startup',
    },
    // 克隆协议选项
    protocol: {
      https: 'HTTPS',
      ssh: 'SSH',
    },
    // 目录组织方式选项
    strategy: {
      flat: 'Flat (root/name)',
      byOwner: 'By owner (root/owner/name)',
      byPlatformAndOwner: 'By platform & owner (root/platform/owner/name)',
    },
    // 主题选项
    theme: {
      auto: 'Follow system',
      light: 'Light',
      dark: 'Dark',
    },
    // 语言选项
    language: {
      zhCn: '简体中文',
      enUs: 'English',
    },
    // Git Tab 各字段标签
    git: {
      executablePath: 'Git executable path',
      executablePathPlaceholder: 'Leave empty to auto-detect from PATH',
      detect: 'Detect Git',
      detecting: 'Detecting…',
      detected: 'Detected',
      notDetected: 'No usable Git detected',
      version: 'Version',
      userName: 'Commit identity user.name',
      userEmail: 'Commit identity user.email',
      pullStrategy: 'Default pull strategy',
      pushStrategy: 'Default push strategy',
    },
    // Pull 策略选项
    pull: {
      ffOnly: 'Fast-forward only (--ff-only)',
      rebase: 'Rebase (--rebase)',
      merge: 'Merge (allow merge commit)',
    },
    // Push 策略选项
    push: {
      simple: 'simple (current branch to same-name upstream)',
      current: 'current (current branch to same-name remote)',
      upstream: 'upstream (push to configured upstream)',
    },
    // 网络 Tab 各字段标签
    network: {
      httpProxy: 'HTTP proxy',
      httpsProxy: 'HTTPS proxy',
      proxyPlaceholder: 'e.g. http://127.0.0.1:7890',
      useSystemProxy: 'Follow system proxy',
      useSystemProxyHint: 'Ignores the manual proxy fields above when enabled',
      apiTimeout: 'API request timeout',
      cloneTimeout: 'Clone timeout',
    },
    // 外部工具 Tab 各字段标签
    externalTools: {
      editor: 'Default editor command',
      editorPlaceholder: 'e.g. code, cursor',
      terminal: 'Default terminal command',
      fileManager: 'Default file manager command',
    },
    // 日志与存储：日志目录占用展示与清理（通用 Tab 底部）
    storage: {
      sectionTitle: 'Logs & Storage', // 分区小标题
      logDir: 'Log directory', // 只读路径标签
      logUsage: 'Log usage', // 占用大小标签
      logFileCount: 'Log files', // 文件数标签
      fileCountUnit: 'files', // 文件数单位后缀
      refresh: 'Refresh', // 重新统计占用
      clearLogs: 'Clear old logs', // 清理按钮
      clearing: 'Clearing…', // 清理进行中按钮态
      clearConfirmTitle: 'Clear old logs', // 二次确认标题
      clearConfirmMessage:
        "Deletes all log files before today, keeping only today's. This cannot be undone.", // 二次确认正文
      clearSuccess: 'Cleared {count} log files, freed {size}', // 成功 toast
      noOldLogs: 'No old logs to clear', // 无可删文件时提示
      clearFailed: 'Failed to clear logs', // 失败 toast 前缀
      loadStatsFailed: 'Failed to read log usage', // 统计失败 toast 前缀
      // 应用数据目录（迁移 / 删除旧目录）
      dataDirTitle: 'Application data directory', // 分区小标题
      dataDirCurrent: 'Current data directory', // 只读路径标签
      dataDirChange: 'Change directory', // 选新目录按钮
      migrateConfirmTitle: 'Migrate data directory', // 迁移二次确认标题
      migrateConfirmMessage:
        'The current database and logs will be copied to "{dir}". A restart is required to take effect, and changes made before restarting will not be included in the new directory. Continue?', // 迁移二次确认正文
      migrateSuccessTitle: 'Migration complete', // 成功弹窗标题
      migrateSuccessMessage:
        'Data copied to "{dir}". Restart the app to use the new directory. The old directory is kept and can be deleted later.', // 成功弹窗正文
      migrateFailed: 'Migration failed', // 失败 toast 前缀
      restartNow: 'Restart now', // 成功弹窗确认按钮
      restartLater: 'Later', // 成功弹窗取消按钮
      loadDataDirFailed: 'Failed to read data directory', // 加载失败 toast 前缀
      oldDir: 'Old data directory', // 旧目录行标签
      deleteOldDir: 'Delete old directory', // 删除旧目录按钮
      deleteOldConfirmTitle: 'Delete old data directory', // 删除二次确认标题
      deleteOldConfirmMessage:
        'This will permanently delete the old data directory "{dir}". This cannot be undone. Confirm?', // 删除二次确认正文
      deleteOldSuccess: 'Old data directory deleted', // 删除成功 toast
      deleteOldFailed: 'Failed to delete old directory', // 删除失败 toast 前缀
      dataDirRestore: 'Restore default directory', // 恢复默认按钮
      dataDirRestoreHint: 'Copy data back to the default directory', // 恢复按钮旁说明
      restoreConfirmTitle: 'Restore default data directory', // 恢复二次确认标题
      restoreConfirmMessage:
        'The current data will be copied back to the default directory. Existing data there will be overwritten. A restart is required to take effect, and changes made before restarting will not be included. Continue?', // 恢复二次确认正文
      restoreFailed: 'Restore failed', // 恢复失败 toast 前缀
    },
    // 账号与安全 Tab：凭据状态与重新验证 / 删除凭据流程
    security: {
      description: 'Only credential existence is shown here; never the plaintext token.', // Tab 顶部安全声明
      noAccounts: 'No accounts yet. Add one in Account Management first.', // 无账号空态
      columnAccount: 'Account',
      columnPlatform: 'Platform',
      columnCredential: 'Credential',
      columnActions: 'Actions',
      credStored: 'Stored', // 绿色 Tag：keyring 中存在凭据
      credMissing: 'Missing', // 红色 Tag：keyring 中无凭据
      credMissingHint: 'Token lost from secure storage; fix it via Re-validate.', // 缺失 Tag 的 tooltip
      checking: 'Checking…', // 凭据存在性查询进行中
      revalidate: 'Re-validate',
      deleteCredential: 'Delete credential',
      // 重新验证对话框
      revalidateTitle: 'Re-validate account credential',
      tokenLabel: 'New token',
      tokenPlaceholder: 'Paste the new access token',
      testAndSave: 'Test & save',
      revalidateSuccess: 'Credential updated',
      // 删除凭据的危险确认
      deleteConfirmTitle: 'Delete account credential',
      deleteConfirmMessage:
        'Removes the token from the system keychain but keeps account metadata.',
      deleteConfirmHint: 'The account cannot sync / clone afterwards until re-validated.',
      deleteSuccess: 'Credential deleted',
    },
  },
};
