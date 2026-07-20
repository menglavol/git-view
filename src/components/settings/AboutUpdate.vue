<!--
  关于与更新子组件（003 / T013·T016·T017·T019）。

  职责：在设置「通用」Tab 底部承载「检查更新 → 下载安装 → 重启」的完整闭环，
  从 Settings.vue 抽出独立组件，避免 Settings.vue 触碰单文件 ≤500 行门禁。

  设计要点：
    - 状态机（本地 ref，不入 store，与 Settings.vue「本地副本」哲学一致）：
      idle → checking →（available | upToDate）；available → downloading → installed →（relaunch）；
      任一步失败落到 failed（中文提示、可重试、不阻断页面）。
    - 所有插件调用统一走 update.api.ts，组件不直接 import 插件（对齐既有「组件不直调底层」风格）。
    - 检查失败仅提示不抛，不影响设置页其余功能（对齐 loadLogStats 策略）。
    - 未做 OS 代码签名：安装后系统仍可能弹「无法验证开发者」警告，属已知现状，不在本组件消除范围。
-->
<template>
  <div class="about-update">
    <!-- 当前版本：始终展示，从 @tauri-apps/api/app 的 getVersion() 读取 -->
    <el-form-item :label="t('settings.update.currentVersion')">
      <span class="version-text">{{ currentVersion || '—' }}</span>
      <!-- 检查更新按钮：checking 期间 loading 并禁用，防重复触发 -->
      <el-button
        class="check-btn"
        :loading="phase === 'checking'"
        :disabled="phase === 'downloading'"
        @click="onCheck"
      >
        {{
          phase === 'checking' ? t('settings.update.checking') : t('settings.update.checkUpdate')
        }}
      </el-button>
    </el-form-item>

    <!-- 已是最新：info 提示 -->
    <el-form-item v-if="phase === 'upToDate'" label=" ">
      <el-alert type="info" :closable="false" show-icon :title="t('settings.update.upToDate')" />
    </el-form-item>

    <!-- 检查/下载失败：warning 提示 + 脱敏错误信息，可重新检查 -->
    <el-form-item v-if="phase === 'failed'" label=" ">
      <el-alert
        type="warning"
        :closable="false"
        show-icon
        :title="`${t('settings.update.checkFailed')}：${errorMessage}`"
      />
    </el-form-item>

    <!-- 发现新版本 / 下载中：success 区块，含版本号、发布说明、操作 -->
    <el-form-item v-if="phase === 'available' || phase === 'downloading'" label=" ">
      <div class="update-panel">
        <!-- 新版本号提示 -->
        <el-alert
          type="success"
          :closable="false"
          show-icon
          :title="t('settings.update.hasUpdate', { version: latestVersion })"
        />

        <!-- 发布说明（US3）：可折叠，内容超长时内部滚动，纯 CSS 不引虚拟滚动 -->
        <el-collapse v-if="releaseNotes !== null" class="notes-collapse">
          <el-collapse-item :title="t('settings.update.releaseNotes')" name="notes">
            <!-- 无说明时占位；有则原样展示，容器 max-height + overflow 滚动 -->
            <pre v-if="releaseNotes" class="notes-body">{{ releaseNotes }}</pre>
            <span v-else class="notes-empty">{{ t('settings.update.noReleaseNotes') }}</span>
          </el-collapse-item>
        </el-collapse>

        <!-- 下载进度（US2）：downloading 期间展示，百分比不倒退 -->
        <div v-if="phase === 'downloading'" class="progress-row">
          <!-- total 已知走确定百分比；未知则用不确定态（percentage 置 0 + 文案提示） -->
          <el-progress
            :percentage="downloadPercent"
            :indeterminate="downloadTotal === null"
            :duration="3"
          />
          <span class="progress-text">{{ progressText }}</span>
        </div>

        <!-- 操作按钮：available 时可下载/稍后；downloading 时禁用 -->
        <div v-else class="action-row">
          <el-button type="primary" @click="onDownloadAndInstall">
            {{ t('settings.update.downloadAndInstall') }}
          </el-button>
          <el-button @click="onLater">{{ t('settings.update.later') }}</el-button>
        </div>
      </div>
    </el-form-item>
  </div>
</template>

<script setup lang="ts">
/**
 * 关于与更新脚本（003）。
 *
 * 用一个 phase 状态驱动整个更新闭环；Update 句柄缓存在 currentUpdate，
 * 供「下载并安装」复用（check 返回的句柄携带下载所需的上下文）。
 */
import { onMounted, ref, shallowRef, computed } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { useI18n } from 'vue-i18n';
import { getVersion } from '@tauri-apps/api/app';
import type { Update } from '@tauri-apps/plugin-updater';

import { updateApi } from '@/api/update.api';

const { t } = useI18n();

// 更新状态机的各阶段（见组件顶部注释的状态流转）。
type Phase =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'installed'
  | 'upToDate'
  | 'failed';

// 当前阶段；初始 idle 只展示版本与检查按钮。
const phase = ref<Phase>('idle');
// 当前应用版本（getVersion 读取；失败留空由模板占位）。
const currentVersion = ref('');
// 远端最新版本号（available 后填充，用于提示文案）。
const latestVersion = ref('');
// 发布说明；null 表示尚未获得（不渲染折叠块），'' 表示有更新但无说明。
const releaseNotes = ref<string | null>(null);
// 失败时的脱敏错误信息，展示给用户。
const errorMessage = ref('');
// check 返回的 Update 句柄，缓存供下载安装复用；无更新时为 null。
// 用 shallowRef 而非 ref：Update 是带私有字段的插件 class，
// ref 的深度解包（UnwrapRef）会拍平其类型并丢失 #private 品牌，
// 导致传给 downloadAndInstall(update: Update) 时类型不匹配；
// 且该句柄是外部对象，本就不应被 Vue 深度响应式化。
const currentUpdate = shallowRef<Update | null>(null);

// 下载进度：已下载字节与总字节（总字节未知时为 null，走不确定进度态）。
const downloadedBytes = ref(0);
const downloadTotal = ref<number | null>(null);

// 下载百分比：总量已知才计算，未知时返回 0（配合 indeterminate 展示）。
const downloadPercent = computed(() => {
  if (downloadTotal.value === null || downloadTotal.value === 0) return 0;
  // 百分比取整并封顶 100，避免累加误差越界
  return Math.min(100, Math.floor((downloadedBytes.value / downloadTotal.value) * 100));
});

// 进度文案：总量已知显示「已下载/总量」，未知只显示已下载量。
const progressText = computed(() => {
  const done = formatBytes(downloadedBytes.value);
  if (downloadTotal.value === null) return done;
  return `${done} / ${formatBytes(downloadTotal.value)}`;
});

/**
 * 把字节数格式化为可读字符串（B/KB/MB/GB）。
 *
 * 与 Settings.vue 的同名局部函数逻辑一致；因本组件独立，自带一份避免跨组件耦合。
 */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`; // 不足 1KB 直接显示字节
  const units = ['KB', 'MB', 'GB', 'TB']; // 逐级进位单位
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(1)} ${units[unitIndex]}`;
}

/**
 * 检查更新（US1）。
 *
 * 有新版则进入 available 并缓存句柄、填充版本与发布说明；
 * 已是最新进入 upToDate；失败落到 failed 仅提示不抛，不阻断页面。
 */
async function onCheck(): Promise<void> {
  phase.value = 'checking';
  errorMessage.value = '';
  try {
    const update = await updateApi.check();
    if (update) {
      currentUpdate.value = update;
      latestVersion.value = update.version;
      // body 可能为空：null 不渲染折叠块，此处统一成 string（空串=有更新无说明）
      releaseNotes.value = update.body ?? '';
      phase.value = 'available';
    } else {
      // 无更新：清空句柄，进入「已是最新」
      currentUpdate.value = null;
      phase.value = 'upToDate';
    }
  } catch (e) {
    // 失败不阻断：记录脱敏信息、切到 failed 态，用户可重新检查
    errorMessage.value = e instanceof Error ? e.message : String(e);
    phase.value = 'failed';
  }
}

/**
 * 下载并安装（US2）。
 *
 * 用进度回调驱动进度条；下载完成（含验签通过）后进入 installed 并引导重启。
 * 任一步失败落到 failed 并提示，不留半成品（插件保证验签失败即中止安装）。
 */
async function onDownloadAndInstall(): Promise<void> {
  const update = currentUpdate.value;
  if (!update) return;
  phase.value = 'downloading';
  downloadedBytes.value = 0;
  downloadTotal.value = null;
  try {
    await updateApi.downloadAndInstall(update, (downloaded, total) => {
      // 进度回调：刷新已下载与总量，模板据此渲染进度条（百分比不倒退）
      downloadedBytes.value = downloaded;
      downloadTotal.value = total;
    });
    // 下载+验签+安装完成，引导重启使新版本生效
    phase.value = 'installed';
    await promptRestart();
  } catch (e) {
    // 下载/验签失败：回到 failed 态并提示，允许重新检查后重试
    errorMessage.value = e instanceof Error ? e.message : String(e);
    phase.value = 'failed';
    ElMessage.error(`${t('settings.update.installFailed')}：${errorMessage.value}`);
  }
}

/**
 * 安装完成后引导重启（US2）。
 *
 * 复用 Settings.vue promptRestart 的交互范式：禁止点遮罩关闭，强制在
 * 「立即重启 / 稍后」间显式选择；「立即」调 relaunch（进程随即重启）。
 */
async function promptRestart(): Promise<void> {
  try {
    await ElMessageBox.confirm(
      t('settings.update.restartMessage'),
      t('settings.update.restartTitle'),
      {
        type: 'success',
        confirmButtonText: t('settings.update.restartNow'),
        cancelButtonText: t('settings.update.restartLater'),
        closeOnClickModal: false, // 重启是重要决策，强制显式选择
      },
    );
    await updateApi.relaunch(); // 进程立即重启，后续代码不会执行
  } catch {
    // 用户选择稍后重启：保持 installed 提示态，下次启动生效
    ElMessage.info(t('settings.update.restartLaterHint'));
  }
}

/** 「稍后」：放弃本次更新，回到 idle（保留已检查到的版本信息由下次检查刷新）。 */
function onLater(): void {
  phase.value = 'idle';
}

// 进入组件即读取当前版本；失败静默（模板用占位符兜底，不影响检查功能）。
onMounted(async () => {
  try {
    currentVersion.value = await getVersion();
  } catch {
    currentVersion.value = '';
  }
});
</script>

<style scoped>
/* 检查更新按钮：与版本号文字拉开间距 */
.check-btn {
  margin-left: 16px;
}

/* 当前版本号：稍加粗便于一眼读出 */
.version-text {
  font-weight: 600;
}

/* 更新面板：纵向堆叠提示、说明、进度、操作，限宽与表单一致 */
.update-panel {
  display: flex; /* 弹性布局承载各子块 */
  flex-direction: column; /* 纵向堆叠：提示在上、操作在下 */
  gap: 12px; /* 子块之间的统一间距 */
  width: 100%; /* 占满表单项可用宽度 */
}

/* 发布说明折叠块：与上方提示留出间距 */
.notes-collapse {
  border: none;
}

/* 发布说明正文：保留换行、限高滚动，防超长撑破布局 */
.notes-body {
  margin: 0; /* 去掉 pre 默认外边距 */
  max-height: 240px; /* 限高，超出走内部滚动 */
  overflow: auto; /* 内容超长时容器内滚动，不撑破布局 */
  white-space: pre-wrap; /* 保留 Markdown 原文换行，同时自动折行 */
  word-break: break-word; /* 长串（如 URL）强制折断，避免横向溢出 */
  font-size: 13px; /* 略小字号，弱化为辅助信息 */
  line-height: 1.6; /* 行距略宽，提升多行说明可读性 */
  color: var(--el-text-color-regular); /* 常规文字色 */
}

/* 无发布说明时的占位文字：次色弱化 */
.notes-empty {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

/* 进度行：进度条与文字同行，文字不换行 */
.progress-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.progress-row .el-progress {
  flex: 1; /* 进度条占据剩余宽度 */
}

/* 进度文案：小字次色，展示已下载/总量 */
.progress-text {
  white-space: nowrap;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

/* 操作按钮行 */
.action-row {
  display: flex;
  gap: 8px;
}
</style>
