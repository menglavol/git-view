# 契约：更新流程与前端 API

**Feature**: 003-app-auto-update | **Phase 1** | 关联 [plan.md](../plan.md) · [data-model.md](../data-model.md)

本文件定义两层契约：
1. **插件契约** —— `tauri-plugin-updater` / `tauri-plugin-process` 暴露给前端的调用与事件。
2. **前端 API 契约** —— `src/api/update.api.ts` 对上述插件的收拢封装（组件只依赖本层，不直接 import 插件）。

---

## 一、插件契约（`@tauri-apps/plugin-updater` v2）

### 1. 检查更新 `check()`

```ts
import { check, type Update } from '@tauri-apps/plugin-updater';

const update: Update | null = await check();
```

- **返回**：有新版时返回 `Update` 对象；已是最新时返回 `null`。
- **Update 关键字段**：
  - `version: string` —— 远端最新版本（来自 `latest.json` 的 `version`）
  - `currentVersion: string` —— 当前应用版本（来自内置 `tauri.conf.json` 的 `version`）
  - `body?: string` —— 发布说明（`latest.json` 的 `notes`）
  - `date?: string` —— 发布时间（`latest.json` 的 `pub_date`）
- **版本比较**：由插件内部完成（客户端**不自写**语义化比较）。
- **失败**：网络不可达 / endpoint 404 / 验签公钥缺失等抛异常，前端 catch 后按「失败不阻断」处理。

### 2. 下载并安装 `update.downloadAndInstall(onEvent)`

```ts
await update.downloadAndInstall((event) => {
  switch (event.event) {
    case 'Started':   // event.data.contentLength：待下载总字节
    case 'Progress':  // event.data.chunkLength：本次增量字节
    case 'Finished':  // 下载完成，进入安装
  }
});
```

- **进度事件三态**：`Started`（拿到总大小）→ `Progress`（多次，累加得已下载量）→ `Finished`。
- **验签**：下载完成后插件用内置公钥对 `.sig` 做 minisign 校验，**校验失败则安装中止并抛错**（防篡改核心）。
- **安装副作用**：替换应用自身二进制/包，属插件受控行为；安装完成后应用需重启才生效。

### 3. 重启 `relaunch()`（`@tauri-apps/plugin-process` v2）

```ts
import { relaunch } from '@tauri-apps/plugin-process';
await relaunch(); // 进程立即重启，后续代码不会执行
```

---

## 二、前端 API 契约（`src/api/update.api.ts`）

组件通过本层调用，屏蔽插件细节；与既有 `settings.api.ts` / `account.api.ts` 的「api 层收拢底层」风格一致。

```ts
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

/** 下载进度回调：已下载字节 / 总字节（总字节未知时为 null）。 */
export type UpdateProgress = (downloaded: number, total: number | null) => void;

export const updateApi = {
  /** 检查是否有新版本；有则返回 Update 句柄，已是最新返回 null。 */
  check(): Promise<Update | null> {
    return check();
  },

  /**
   * 下载并安装给定更新，通过 onProgress 回报进度。
   * 内部把插件的 Started/Progress/Finished 事件归一为「已下载 / 总量」两个数。
   */
  async downloadAndInstall(update: Update, onProgress: UpdateProgress): Promise<void> {
    let downloaded = 0;
    let total: number | null = null;
    await update.downloadAndInstall((event: DownloadEvent) => {
      if (event.event === 'Started') {
        total = event.data.contentLength ?? null; // 起始拿到总大小
      } else if (event.event === 'Progress') {
        downloaded += event.data.chunkLength; // 累加增量
        onProgress(downloaded, total);
      } else if (event.event === 'Finished') {
        onProgress(total ?? downloaded, total); // 完成时进度置满
      }
    });
  },

  /** 安装完成后重启应用使新版本生效。 */
  relaunch(): Promise<void> {
    return relaunch();
  },
};
```

**契约要点**：
- 组件拿到的进度永远是「已下载字节 + 总字节」两个数，UI 自行换算百分比与可读大小（复用 `Settings.vue` 既有 `formatBytes`）。
- `total` 可能为 `null`（服务端未给 Content-Length），UI 需容错：无总量时显示不确定进度态。
- API 层不吞异常，异常向上抛给组件统一提示（对齐既有 api 层「只封装不吞错」风格）。

---

## 三、组件交互契约（`AboutUpdate.vue`）

状态机（本地 `ref`，不入 store）：

```
idle ──检查──► checking ──┬─ 有新版 ─► available ──下载──► downloading ──► installed ──重启──► (relaunch)
                          └─ 已最新 ─► upToDate
     任一步失败 ─► failed（中文提示，可重试，不阻断页面）
```

| 状态 | UI 表现 |
|---|---|
| `idle` | 显示当前版本 + 「检查更新」按钮 |
| `checking` | 按钮 loading「检查中…」 |
| `available` | `el-alert` success：新版本号 + 可折叠发布说明 + 「下载并安装」+「稍后」 |
| `downloading` | 进度条（百分比 + 已下载/总量），禁用重复触发 |
| `installed` | `ElMessageBox` 引导「立即重启 / 稍后」（复用 `promptRestart` 交互范式） |
| `upToDate` | `el-alert` info「已是最新版本」 |
| `failed` | `el-alert` warning + 脱敏错误信息 + 可重新检查 |

**权限前提**（`capabilities/default.json`）：`updater:default`、`process:allow-restart`。
