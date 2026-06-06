# GitView MVP 验收报告

记录 spec.md §SC-018 的 20 项 MVP 验收用例在 **macOS / Windows / Ubuntu** 三平台的走查结果（对应 SC-015 跨平台一致性）。本文件为**模板**，需在各平台实测后填写结果。

> 结果填写：✅ 通过 ｜ ❌ 失败(附问题链接/说明) ｜ ⬜ 未测

## 测试环境(待填)

| 平台 | 系统版本 | GitView 版本 | 测试人 | 日期 |
| --- | --- | --- | --- | --- |
| macOS | _待填_ | _待填_ | _待填_ | _待填_ |
| Windows | _待填_ | _待填_ | _待填_ | _待填_ |
| Ubuntu | _待填_ | _待填_ | _待填_ | _待填_ |

## SC-018 验收用例

| # | 用例 | 预期 | macOS | Windows | Ubuntu | 备注 |
| --- | --- | --- | :---: | :---: | :---: | --- |
| 1 | 启动应用 | 应用窗口正常打开 | ⬜ | ⬜ | ⬜ | |
| 2 | 检测系统 Git | 正确识别已装/未装并引导 | ⬜ | ⬜ | ⬜ | |
| 3 | 添加 GitHub 账号并同步仓库 | 连接成功、仓库入列表 | ⬜ | ⬜ | ⬜ | |
| 4 | 添加 GitLab(含自建)账号并同步 | API 地址推导正确、同步成功 | ⬜ | ⬜ | ⬜ | |
| 5 | 添加 Gitee 账号并同步 | 连接成功、同步成功 | ⬜ | ⬜ | ⬜ | |
| 6 | 远程仓库搜索和筛选 | 关键词/筛选结果正确 | ⬜ | ⬜ | ⬜ | |
| 7 | 多选仓库批量 Clone | 任务生成、并发受控 | ⬜ | ⬜ | ⬜ | |
| 8 | Clone 任务显示进度 | 实时百分比与阶段 | ⬜ | ⬜ | ⬜ | |
| 9 | Clone 失败后重试 | 重试生效、不影响他任务 | ⬜ | ⬜ | ⬜ | |
| 10 | Clone 成功进入本地仓库列表 | 自动入列、状态 clean | ⬜ | ⬜ | ⬜ | |
| 11 | 手动添加本地 Git 仓库 | 校验并入列 | ⬜ | ⬜ | ⬜ | |
| 12 | 查看本地仓库状态 | 分支/ahead-behind/变更数正确 | ⬜ | ⬜ | ⬜ | |
| 13 | 查看文件变更和 diff | diff 高亮正确 | ⬜ | ⬜ | ⬜ | |
| 14 | stage / unstage 文件 | 暂存区正确变化 | ⬜ | ⬜ | ⬜ | |
| 15 | 提交 commit | 提交成功、含中文 message | ⬜ | ⬜ | ⬜ | |
| 16 | fetch / pull / push | 网络操作成功、错误有提示 | ⬜ | ⬜ | ⬜ | |
| 17 | 查看分支并切换 | 列表正确、脏区阻断切换 | ⬜ | ⬜ | ⬜ | |
| 18 | 查看操作日志 | 日志可见、可筛选 | ⬜ | ⬜ | ⬜ | |
| 19 | Token 不在数据库明文字段 | 自动扫描 0 命中 | ⬜ | ⬜ | ⬜ | `check-no-token-leak.sh` |
| 20 | Token 不在日志 | 自动扫描 0 命中 | ⬜ | ⬜ | ⬜ | `check-no-token-leak.sh` |

发布门禁：**20/20 在三平台全部通过**方可发布 V1（SC-018）。任一 ❌ 须修复并重测。

---

## 附：T108 构建配置核验(自动检查结果)

应用元数据与图标已就绪，三平台可分发包只需在对应环境执行 `npm run tauri:build`（或经 `.github/workflows/release.yml` 由 tag 触发）。

| 检查项 | 状态 | 说明 |
| --- | --- | --- |
| `productName` / `version` / `identifier` | ✅ 就绪 | `GitView` / `0.1.0` / `com.gitview.app` |
| `bundle.category` / `copyright` | ✅ 就绪 | `DeveloperTool` / `Copyright © 2026 GitView Team` |
| `shortDescription` / `longDescription` | ✅ 就绪 | 已填中文产品描述 |
| 图标(icns / ico / png / 多尺寸) | ✅ 就绪 | `src-tauri/icons/` 含 macOS/Windows/通用图标 |
| 三平台可分发包(.dmg/.msi/.deb/.AppImage) | ⬜ 待构建 | 需在各平台 `tauri:build` 或 release workflow 产出 |

> V1 暂未做 macOS 公证与 Windows 代码签名，首次运行警告的绕过方法见 README「安装」一节（待 T112 三平台实测时一并验证）。
