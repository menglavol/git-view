# GitView 路线图

本文件明确 **V1 已交付什么、不做什么，以及 V2/V3/V4 各版本计划**。能力划分来源于产品功能设计文档 §5，范围边界与 [spec.md](../specs/001-gitview-mvp/spec.md) 的 Assumptions 章节保持一致。

> 状态图例：✅ 已交付 ｜ 🚧 部分交付(V1 已含简化版) ｜ ⬜ 计划中

---

## V1 — MVP 可用版(当前)

**目标**：账号添加稳定、远程仓库同步稳定、批量 Clone 稳定、基础 Git 操作稳定。

| 能力 | 状态 |
| --- | --- |
| Git 环境检测与首次启动引导 | ✅ |
| 多平台多账号管理(GitHub / GitLab / 自建 GitLab / Gitee，PAT 登录) | ✅ |
| 远程仓库同步 / 搜索 / 筛选 / 收藏 | ✅ |
| 多选仓库批量 Clone + 任务队列(并发 / 进度 / 取消 / 重试) | ✅ |
| 本地仓库列表 / 目录扫描 / 状态总览 / 批量 Fetch | ✅ |
| 单仓库工作流：status / diff / stage / unstage / commit | ✅ |
| fetch / pull / push | ✅ |
| 分支列表与切换、新建分支 | ✅ |
| 提交历史(简版列表) | ✅ |
| 操作日志与中文错误提示、敏感信息脱敏 | ✅ |
| 设置中心(通用 / Git / 网络 / 外部工具 / 账号与安全) | ✅ |
| 首页仪表盘 | ✅ |

### V1 明确不做(归入后续版本)

与 spec Assumptions 一致：

- **认证**：仅支持 PAT；OAuth / SSO / 双因素 → V2/V3。
- **SSH Key 管理 UI**：由用户自行在 `~/.ssh/config` 维护 → V4。
- **冲突解决**：仅检测与提示，需外部工具解决 → V2。
- **平台协作**(PR/MR、Issue、CI)：仅提供「打开网页」跳转 → V3。
- **进阶 Git**：stash、tag、merge、rebase、cherry-pick、revert、reset、commit amend、commit graph → V2/V4。
- **多设备同步、团队配置同步、插件机制** → V4。

---

## V2 — 日常开发增强版

**目标**：让 GitView 成为开发者日常可用的 Git GUI。

| 能力 | 状态 |
| --- | --- |
| 新建分支 | 🚧(V1 已含) |
| 删除分支 | ⬜ |
| merge | ⬜ |
| stash | ⬜ |
| tag | ⬜ |
| cherry-pick | ⬜ |
| revert | ⬜ |
| commit amend | ⬜ |
| reset(soft / mixed / hard) | ⬜ |
| commit history(完整) | 🚧(V1 已含简版) |
| commit graph 初版 | ⬜ |
| 冲突检测 | ⬜ |
| 基础冲突解决 | ⬜ |
| 批量 fetch | 🚧(V1 已含) |
| 批量 pull | ⬜ |
| 目录扫描 | 🚧(V1 已含) |

---

## V3 — 平台协作版

**目标**：接入 GitHub / GitLab / Gitee 的协作能力。

| 能力 | 状态 |
| --- | --- |
| Pull Request / Merge Request 列表 | ⬜ |
| 创建 PR / MR | ⬜ |
| 查看 PR / MR 状态 | ⬜ |
| Issue 列表 | ⬜ |
| CI 状态查看 | ⬜ |
| 远程分支管理 | ⬜ |
| 组织 / Group 项目管理 | ⬜ |
| 项目收藏 | 🚧(V1 已含远程仓库收藏) |
| 项目归档 | ⬜ |

---

## V4 — 高级增强版

**目标**：提升专业性、自动化与可扩展性。

| 能力 | 状态 |
| --- | --- |
| AI 生成 commit message | ⬜ |
| AI 解释 commit | ⬜ |
| 多仓库批量 pull / push / 状态检查 | ⬜ |
| Git LFS 检测 | ⬜ |
| submodule 管理 | ⬜ |
| SSH Key 管理 | ⬜ |
| 代理高级配置 | ⬜ |
| 自动更新 | ⬜ |
| 多语言(完整中英) | 🚧(V1 已含 i18n 骨架) |
| 插件机制 | ⬜ |
| 团队配置同步 | ⬜ |
