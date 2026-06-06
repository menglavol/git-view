<!--
  首次启动引导 / 环境检查页（T109）。

  应用启动时若未检测到可用 git，路由守卫（router/index.ts）会把用户导到这里。
  职责：说明问题 + 给出当前平台的安装指引 + 提供「重新检测」与「去设置手动指定
  路径」两条出路。设计为独立全屏页（不套 AppLayout 侧边栏）——在 git 不可用时，
  不应让用户还能误入克隆 / 工作流等处处报错的依赖 git 的功能页。
-->
<template>
  <!-- 全屏居中容器：引导页不依赖布局组件，自身撑满窗口并垂直水平居中 -->
  <div class="onboarding">
    <!-- el-result 提供「警告图标 + 主副标题 + extra 插槽」的标准结果版式 -->
    <el-result
      icon="warning"
      title="未检测到可用的 Git"
      sub-title="GitView 的克隆、提交、分支等核心功能都依赖系统 Git，请先安装或指定其路径。"
    >
      <!-- extra 插槽承载正文：平台提示 + 安装指引 + 操作按钮 -->
      <template #extra>
        <!-- 正文容器：限宽左对齐，承载平台提示、命令指引与操作按钮 -->
        <div class="onboarding__body">
          <!-- 当前平台提示：仅用于挑选下方安装指引，无需精确识别发行版 -->
          <el-alert
            :title="`检测到当前系统：${platformLabel}`"
            type="info"
            :closable="false"
            show-icon
          />

          <!-- 分平台安装步骤区 -->
          <div class="onboarding__guide">
            <!-- 小标题：引导用户阅读下方命令 -->
            <p class="onboarding__guide-title">推荐安装方式</p>
            <!-- 安装步骤逐条渲染，命令让用户复制到终端执行 -->
            <ul>
              <li v-for="(line, i) in installSteps" :key="i">{{ line }}</li>
            </ul>
            <!-- 仅 Windows 给官网下载链接（其余平台用包管理器即可，无需外链） -->
            <p v-if="downloadUrl" class="onboarding__hint">
              官方下载：
              <el-link type="primary" :href="downloadUrl" target="_blank">{{
                downloadUrl
              }}</el-link>
            </p>
            <!-- Linux 额外提示：凭据存储依赖 keyring 后端，缺失会导致账号功能不可用 -->
            <p v-if="platform === 'linux'" class="onboarding__hint">
              若添加账号时提示「凭据存储不可用」，请安装 gnome-keyring 或 libsecret。
            </p>
          </div>

          <!-- 两条出路：装好后重新检测，或转去设置手动指定可执行文件路径 -->
          <div class="onboarding__actions">
            <!-- 主操作：强制重新探测，成功即进入应用 -->
            <el-button type="primary" :loading="rechecking" @click="onRecheck">
              已安装，重新检测
            </el-button>
            <!-- 次操作：跳设置页（在守卫白名单内）手动指定 git 路径 -->
            <el-button @click="goSettings">去设置手动指定路径</el-button>
          </div>
          <!-- 补充说明：覆盖「装在非标准位置」的兜底路径 -->
          <p class="onboarding__path-tip">
            安装完成后点「重新检测」；若 git 装在非标准位置，可在设置里手动指定可执行文件路径。
          </p>
        </div>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';

import { useAppStore } from '@/stores/app';

// ---- 依赖装配 ----
// router 用于检测成功后跳首页 / 跳设置；appStore 持有全局 git 检测状态。
const router = useRouter();
const appStore = useAppStore();

// ---- 平台识别与安装指引（仅决定展示文案，不影响检测逻辑）----
// 平台识别：用 UserAgent 粗判即可，仅用于挑选安装指引文案，无需精确到发行版。
type OsKind = 'macos' | 'windows' | 'linux';
const platform = computed<OsKind>(() => {
  const ua = navigator.userAgent;
  // 注意顺序：先匹配 Mac / Win，剩余一律按 Linux 处理。
  if (ua.includes('Mac')) return 'macos';
  if (ua.includes('Win')) return 'windows';
  return 'linux';
});

/** 平台中文展示名（用于顶部 alert 文案）。 */
const platformLabel = computed(() => {
  switch (platform.value) {
    case 'macos':
      return 'macOS';
    case 'windows':
      return 'Windows';
    default:
      return 'Linux';
  }
});

// 各平台安装步骤（纯文案，命令让用户复制到终端执行）。
const installSteps = computed<string[]>(() => {
  switch (platform.value) {
    case 'macos':
      // macOS 优先 Homebrew；无 Homebrew 时用 Xcode 命令行工具兜底。
      return [
        '方式一（推荐）：安装 Homebrew 后执行 brew install git',
        '方式二：执行 xcode-select --install 安装命令行工具（自带 git）',
      ];
    case 'windows':
      // Windows 无系统包管理器，引导到官网安装包。
      return [
        '从 Git 官网下载安装包并按默认选项安装',
        '安装后重启 GitView，或直接点击下方「重新检测」',
      ];
    default:
      // Linux 覆盖三大主流包管理器，命中用户的发行版即可。
      return [
        'Debian / Ubuntu：sudo apt install git',
        'Fedora / RHEL：sudo dnf install git',
        'Arch：sudo pacman -S git',
      ];
  }
});

// 仅 Windows 给出官网下载链接；macOS / Linux 用包管理器命令即可，留空不渲染外链。
const downloadUrl = computed(() =>
  platform.value === 'windows' ? 'https://git-scm.com/download/win' : '',
);

// ---- 重新检测与后续导航 ----
// 「重新检测」按钮的 loading 态，避免重复点击触发并发探测。
const rechecking = ref(false);

/**
 * 「重新检测」：强制重新探测 git。
 * 成功就进首页；失败给出提示，让用户继续排查或转去设置手动指定路径。
 */
async function onRecheck(): Promise<void> {
  rechecking.value = true;
  try {
    // recheckGit 内部已吞掉异常并归一为 found=false，这里无需再 try 业务分支。
    await appStore.recheckGit();
  } finally {
    rechecking.value = false;
  }
  // 依据最新检测结果分流：就绪则进入应用，否则留在本页继续提示。
  if (appStore.gitReady) {
    // 用 replace 而非 push：引导页是临时态，不该留在历史栈里被「后退」回来。
    ElMessage.success('已检测到 Git，正在进入应用');
    await router.replace({ name: 'dashboard' });
  } else {
    ElMessage.warning('仍未检测到 Git，请确认已正确安装，或去设置手动指定路径');
  }
}

// 去设置页手动指定 git 路径（设置页在守卫白名单内，可正常进入）。
function goSettings(): void {
  void router.push({ name: 'settings' });
}
</script>

<style scoped>
/* 全屏居中容器：引导页不套布局，自己撑满 Tauri 窗口并居中内容 */
.onboarding {
  /* flex 居中：让结果版式在窗口中央，不论窗口多大都视觉聚焦 */
  display: flex;
  align-items: center;
  justify-content: center;
  /* 撑满视口高度，使结果版式垂直居中 */
  min-height: 100vh;
  padding: 24px;
  /* 含 padding 计算尺寸，避免溢出出现滚动条 */
  box-sizing: border-box;
}

/* 正文区：限定宽度并左对齐，避免大窗口下文字过宽难读 */
.onboarding__body {
  /* 520px 是命令文案的舒适阅读宽度；窗口更窄时由 max-width 收敛 */
  width: 520px;
  max-width: 100%;
  /* el-result 默认居中，正文改左对齐更符合阅读步骤的习惯 */
  text-align: left;
}

/* 安装指引区与上方 alert 留出间距 */
.onboarding__guide {
  margin-top: 16px;
}

/* 指引小标题加粗，作为命令列表的视觉锚点 */
.onboarding__guide-title {
  font-weight: 600;
  /* 与下方命令列表留出呼吸间距 */
  margin-bottom: 8px;
}

/* 命令列表：放大行高便于逐行阅读与复制 */
.onboarding__guide ul {
  margin: 0;
  /* 缩进留出项目符号空间 */
  padding-left: 20px;
  line-height: 1.9;
}

/* 次要提示文字：弱化颜色与字号，从属于主指引 */
.onboarding__hint {
  margin-top: 12px;
  /* 用 Element 次级文本色，自动适配明暗主题 */
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

/* 操作按钮区：横向排列并留间距 */
.onboarding__actions {
  margin-top: 24px;
  display: flex;
  /* 两个按钮之间的水平间距 */
  gap: 12px;
}

/* 底部兜底说明：最弱化的辅助文字 */
.onboarding__path-tip {
  margin-top: 12px;
  /* 同样跟随主题的次级文本色 */
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
