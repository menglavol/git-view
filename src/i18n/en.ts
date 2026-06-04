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
