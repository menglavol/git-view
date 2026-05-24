# Git 可视化客户端产品功能设计文档

## 1. 文档概述

### 1.1 文档目的

本文档用于指导一个轻量级跨平台 Git 可视化客户端的产品设计、功能设计、技术方案设计和后续开发排期。

该产品目标是提供一个支持 macOS、Windows、Ubuntu 的桌面 Git 客户端，重点解决以下问题：

1. 多平台 Git 仓库账号统一管理。
2. GitHub、GitLab、Gitee 等平台仓库统一浏览。
3. 多账号、多平台仓库批量克隆。
4. 本地 Git 仓库集中管理。
5. 常用 Git 操作可视化。
6. 尽可能保持轻量、快速、低资源占用。

本文档应作为后续 UI 原型设计、数据库设计、Rust 后端设计、Tauri 命令设计、Vue 前端组件设计和版本规划的依据。

---

## 2. 产品定位

### 2.1 产品名称

产品名称：**GitView**

本文档中统一使用 **GitView** 作为产品名称。

### 2.2 产品定位

GitView 是一个轻量级跨平台 Git 可视化客户端，支持 GitHub、GitLab、Gitee 多平台和多账号管理，专注于远程仓库统一浏览、多项目批量 Clone、本地仓库集中管理和常用 Git 操作可视化。

### 2.3 核心价值

GitView 的核心价值不是一开始完全替代 GitKraken、Fork、SourceTree、Tower，而是先聚焦以下差异化能力：

1. **多平台统一管理**：同时支持 GitHub、GitLab、Gitee。
2. **多账号统一管理**：同一平台支持多个账号，例如个人账号、公司账号、自建 GitLab 账号。
3. **批量 Clone**：支持从远程仓库列表中多选项目，一次性克隆到本地指定目录。
4. **轻量体验**：基于 Tauri + Rust，降低安装包体积和运行时资源占用。
5. **日常 Git 操作可视化**：覆盖 status、diff、stage、commit、pull、push、fetch、branch 等常用操作。

### 2.4 目标用户

#### 2.4.1 主要用户

1. 个人开发者
2. 企业研发人员
3. 需要同时维护多个 Git 平台账号的开发者
4. 经常需要批量拉取项目的开发者
5. 不想频繁使用命令行完成基础 Git 操作的用户

#### 2.4.2 次要用户

1. 初学 Git 的开发者
2. 开源项目维护者
3. 小团队技术负责人
4. 运维、测试、实施人员

### 2.5 目标平台

必须支持：

1. macOS
2. Windows
3. Ubuntu / Linux

---

## 3. 技术路线总览

### 3.1 最终技术栈

```text
Tauri 2
Vue 3
TypeScript
Vite
Element Plus
Pinia
@tanstack/vue-query，可选
Rust
SQLite
Git CLI
git2-rs / libgit2，作为增强能力
系统 Keychain / Credential Manager / Secret Service
```

### 3.2 技术选型原则

1. **轻量优先**：桌面框架采用 Tauri，避免 Electron 带来的高体积和高内存占用。
2. **稳定优先**：Git 操作第一阶段优先调用系统 Git CLI，保证行为与命令行 Git 一致。
3. **安全优先**：Token、密码、SSH 口令等敏感信息不得明文存入 SQLite。
4. **职责分离**：平台 API Token 主要用于拉取仓库、用户、组织、PR/MR 等平台数据；Git clone、pull、push 默认交给系统 Git、SSH 配置或安全的临时凭据注入处理，避免把 Token 写入远程地址或日志。
5. **可扩展优先**：GitHub、GitLab、Gitee 通过 provider 模式拆分，后续可扩展 Bitbucket、Azure DevOps 等平台。
6. **渐进增强**：第一版不追求完整 GitKraken 级别功能，先保证核心流程稳定。
7. **可观测优先**：所有长耗时任务，例如同步远程仓库、批量 Clone、Fetch、Pull、Push，都必须有任务状态、错误信息和操作日志。

### 3.3 总体架构

```text
GitView Desktop App
│
├─ 前端层：Vue 3 + TypeScript + Element Plus
│  ├─ 页面与交互
│  ├─ 仓库列表
│  ├─ Git 状态展示
│  ├─ Diff 展示
│  ├─ Clone 任务中心
│  └─ 设置与日志
│
├─ 桌面容器：Tauri 2
│  ├─ 窗口管理
│  ├─ 前后端通信
│  ├─ 文件系统权限
│  └─ 应用打包
│
├─ 后端核心：Rust
│  ├─ Git CLI 封装
│  ├─ 多平台 API 服务
│  ├─ Clone 任务队列
│  ├─ SQLite 数据访问
│  ├─ 凭据安全存储
│  ├─ 日志记录
│  └─ 系统工具调用
│
├─ Git 执行层
│  ├─ 系统 Git CLI：主要 Git 操作
│  └─ git2-rs/libgit2：后续增强读取性能
│
└─ 本地存储
   ├─ SQLite：账号元信息、仓库信息、任务、日志、设置
   └─ 系统凭据服务：Token、敏感凭据
```

---

## 4. 产品功能总览

### 4.1 一级功能模块

GitView 包含以下一级模块：

1. 首页仪表盘
2. 多平台账号管理
3. 远程仓库中心
4. 批量 Clone 中心
5. 本地仓库管理
6. 单仓库工作区
7. Git 操作中心
8. 平台协作功能
9. 设置中心
10. 日志与问题诊断

### 4.2 主界面布局

推荐采用典型桌面工具布局：

```text
┌────────────────────────────────────────────────────────────┐
│ 顶部栏：当前账号 / 当前平台 / 全局搜索 / 同步 / 设置          │
├───────────────┬────────────────────────────────────────────┤
│ 左侧导航       │ 主内容区                                     │
│               │                                            │
│ 首页           │                                            │
│ 远程仓库       │                                            │
│ 本地仓库       │                                            │
│ Clone 中心     │                                            │
│ 账号管理       │                                            │
│ 操作日志       │                                            │
│ 设置           │                                            │
└───────────────┴────────────────────────────────────────────┘
```

### 4.3 左侧导航

左侧导航项建议：

1. 首页
2. 远程仓库
3. 本地仓库
4. Clone 中心
5. 账号管理
6. 操作日志
7. 设置

后续版本可增加：

1. Pull Request / Merge Request
2. Issue
3. CI 状态
4. 插件中心

---

## 5. 版本规划

### 5.1 V1：MVP 可用版

目标：让用户完成账号添加、远程仓库同步、批量 Clone、本地仓库管理和基础 Git 操作。

必须包含：

1. Git 环境检测
2. 账号管理
3. GitHub / GitLab / Gitee Token 登录
4. 远程仓库同步
5. 远程仓库搜索和筛选
6. 多选仓库批量 Clone
7. Clone 任务队列
8. 本地仓库列表
9. 打开单个仓库
10. Git status
11. 文件 diff
12. stage / unstage
13. commit
14. fetch / pull / push
15. 分支列表和切换
16. 设置默认项目目录
17. 操作日志

### 5.2 V2：日常开发增强版

目标：让 GitView 成为开发者日常可用的 Git GUI。

增加：

1. 新建分支
2. 删除分支
3. merge
4. stash
5. tag
6. cherry-pick
7. revert
8. commit amend
9. reset soft / mixed / hard
10. commit history
11. commit graph 初版
12. 冲突检测
13. 基础冲突解决
14. 批量 fetch
15. 批量 pull
16. 目录扫描

### 5.3 V3：平台协作版

目标：增加 GitHub、GitLab、Gitee 平台协作能力。

增加：

1. Pull Request / Merge Request 列表
2. 创建 PR / MR
3. 查看 PR / MR 状态
4. Issue 列表
5. CI 状态查看
6. 远程分支管理
7. 组织 / Group 项目管理
8. 项目收藏
9. 项目归档

### 5.4 V4：高级增强版

目标：提升专业性、自动化能力和可扩展性。

增加：

1. AI commit message
2. AI 解释 commit
3. 多仓库批量 pull
4. 多仓库批量 push
5. 多仓库批量状态检查
6. Git LFS 检测
7. submodule 管理
8. SSH Key 管理
9. 代理高级配置
10. 自动更新
11. 多语言
12. 插件机制
13. 团队配置同步

---

## 6. 模块一：首页仪表盘

### 6.1 模块目标

首页用于让用户快速了解当前账号、远程仓库、本地仓库和任务状态。

### 6.2 功能清单

#### 6.2.1 概览卡片

显示以下指标：

1. 已添加账号数量
2. 远程仓库数量
3. 本地仓库数量
4. 正在运行的 Clone 任务数量
5. 有未提交变更的仓库数量
6. 需要 push 的仓库数量
7. 需要 pull 的仓库数量
8. 最近失败任务数量

#### 6.2.2 最近打开仓库

显示最近打开的本地仓库列表。

字段：

1. 仓库名
2. 本地路径
3. 当前分支
4. 未提交变更数量
5. ahead 数量
6. behind 数量
7. 最后打开时间

快捷操作：

1. 打开仓库
2. Fetch
3. Pull
4. Push
5. 打开目录
6. 打开终端

#### 6.2.3 最近任务

显示最近执行的 Git 操作和 Clone 任务。

字段：

1. 任务名
2. 任务类型
3. 仓库名
4. 状态
5. 进度
6. 错误摘要
7. 执行时间

#### 6.2.4 快捷入口

提供以下快捷按钮：

1. 添加账号
2. 从远程 Clone
3. 添加本地仓库
4. 扫描目录
5. 打开设置

### 6.3 UI 组件建议

Element Plus 组件：

1. `el-card`
2. `el-row`
3. `el-col`
4. `el-statistic`
5. `el-table`
6. `el-tag`
7. `el-button`
8. `el-empty`

---

## 7. 模块二：多平台账号管理

> 本模块需要重点支持小型公司自建 GitLab 环境。自建 GitLab 不应被简单当作 gitlab.com 的一个地址输入框，而应作为完整的 GitLab 实例配置能力来设计，包括实例地址、API 地址、证书策略、代理策略、Token 验证、Group 同步和 Clone URL 规则。

### 7.1 模块目标

管理 GitHub、GitLab、Gitee 多平台账号，并支持同一平台多个账号。

### 7.2 支持平台

V1 必须支持：

1. GitHub
2. GitLab
3. Gitee

GitLab 必须支持：

1. `https://gitlab.com`
2. 自建 GitLab，例如 `https://git.company.com`
3. 内网 GitLab，例如 `http://gitlab.local`、`http://192.168.1.10`、`https://git.company.local`
4. 非标准端口 GitLab，例如 `https://git.company.com:8443`
5. 带子路径部署的 GitLab，例如 `https://code.company.com/gitlab`，需要在高级配置中处理 API 地址推导

后续可扩展：

1. Bitbucket
2. Azure DevOps
3. Codeberg
4. 自定义 Git 服务

### 7.3 账号模型

账号模型中需要区分 Web 地址和 API 地址。GitHub 的 Web 地址是 `https://github.com`，但 REST API 地址通常是 `https://api.github.com`；GitLab 自建实例的 Web 地址和 API 地址通常也需要通过 `/api/v4` 区分。如果只保存一个 `baseUrl`，后续会在平台 API 调用和 Web 页面跳转之间产生混淆。

```ts
type GitPlatform = 'github' | 'gitlab' | 'gitee';

interface Account {
  id: string;
  platform: GitPlatform;
  webBaseUrl: string;
  apiBaseUrl: string;
  username: string;
  displayName?: string;
  avatarUrl?: string;
  tokenKey: string;
  isDefault: boolean;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
  lastSyncAt?: string;
}
```

默认地址建议：

1. GitHub：`webBaseUrl = https://github.com`，`apiBaseUrl = https://api.github.com`
2. GitLab：`webBaseUrl = https://gitlab.com`，`apiBaseUrl = https://gitlab.com/api/v4`
3. 自建 GitLab：`webBaseUrl = https://git.company.com`，`apiBaseUrl = https://git.company.com/api/v4`
4. Gitee：`webBaseUrl = https://gitee.com`，`apiBaseUrl` 根据 Gitee OpenAPI 实际地址配置

数据库字段可以继续保留 `base_url` 作为兼容字段，但建议正式设计时拆成 `web_base_url` 和 `api_base_url`。

### 7.4 添加账号

#### 7.4.1 V1 登录方式

V1 使用 Personal Access Token 登录。

添加账号表单字段：

1. 平台：GitHub / GitLab / Gitee
2. Web 服务地址：默认根据平台填充
3. API 服务地址：根据平台自动推导，允许高级用户手动修改
4. Token
5. 账号备注
6. 是否设为默认账号

默认服务地址：

1. GitHub：Web 地址 `https://github.com`，API 地址 `https://api.github.com`
2. GitLab：Web 地址 `https://gitlab.com`，API 地址 `https://gitlab.com/api/v4`
3. 自建 GitLab：Web 地址由用户输入，API 地址默认追加 `/api/v4`
4. Gitee：Web 地址 `https://gitee.com`，API 地址根据 Gitee OpenAPI 实际地址配置

Token 权限要求需要在 UI 中明确提示。最低权限应满足读取用户信息和仓库列表；如果后续支持 PR/MR、Issue、CI 或私有仓库操作，需要提示用户增加对应权限。

#### 7.4.2 连接测试

用户填写 Token 后，点击“测试连接”。

系统需要校验：

1. Token 是否有效
2. API 地址是否正确
3. 当前用户名
4. 头像地址
5. 账号权限是否足够
6. 网络是否可达

连接成功后：

1. 保存账号元信息到 SQLite。
2. 保存 Token 到系统凭据服务。
3. 设置 lastSyncAt 为空。
4. 允许用户立即同步远程仓库。

连接失败时，需要给出明确错误：

1. Token 无效
2. API 地址错误
3. 网络连接失败
4. 权限不足
5. 平台类型和地址不匹配
6. 代理配置错误

### 7.5 账号列表

账号列表显示字段：

1. 平台
2. 头像
3. 用户名
4. 显示名称
5. 服务地址
6. 是否默认账号
7. 是否启用
8. 最近同步时间
9. 操作按钮

操作按钮：

1. 同步仓库
2. 测试连接
3. 编辑备注
4. 设为默认
5. 启用 / 禁用
6. 删除账号

### 7.6 删除账号

删除账号时需要弹窗确认。

删除行为：

1. 删除 SQLite 中账号记录。
2. 删除系统凭据服务中的 Token。
3. 保留已经 Clone 的本地仓库记录。
4. 远程仓库缓存可以选择删除或保留。

建议弹窗选项：

1. 仅删除账号
2. 同时删除该账号的远程仓库缓存

### 7.7 私有 GitLab 实例配置

#### 7.7.1 功能目标

支持小型公司、工作室或内部团队自行部署的 GitLab 实例，使用户可以像使用 gitlab.com 一样添加、验证、同步和克隆私有 GitLab 仓库。

该功能需要覆盖：

1. 私有 GitLab 实例地址配置
2. API 地址自动推导和手动修改
3. Access Token 验证
4. 内网地址支持
5. 自签名证书提示
6. HTTP / HTTPS 支持
7. 非标准端口支持
8. Group / Subgroup 项目同步
9. 私有仓库 Clone URL 处理
10. 实例级别网络和安全配置

#### 7.7.2 添加私有 GitLab 入口

账号管理页中，添加 GitLab 账号时需要提供两种模式：

1. GitLab.com
2. 私有 GitLab / 自建 GitLab

当用户选择“私有 GitLab / 自建 GitLab”时，显示更完整的实例配置表单。

#### 7.7.3 私有 GitLab 表单字段

基础字段：

1. 实例名称，例如“公司 GitLab”、“研发内网 GitLab”
2. Web 地址，例如 `https://git.company.com`
3. API 地址，默认自动推导，例如 `https://git.company.com/api/v4`
4. Personal Access Token
5. 账号备注
6. 是否设为默认 GitLab 账号

高级字段：

1. 是否使用系统代理
2. 自定义代理地址
3. 请求超时时间
4. 是否允许 HTTP 明文连接
5. 是否允许自签名证书
6. 是否跳过 TLS 证书校验，不建议默认开启
7. Clone 协议默认值：HTTPS / SSH
8. SSH 主机别名，可选，例如 `company-gitlab`
9. API 路径前缀，可选，用于带子路径部署的 GitLab

#### 7.7.4 API 地址推导规则

用户填写 Web 地址后，系统自动生成 API 地址：

普通部署：

```text
Web 地址：https://git.company.com
API 地址：https://git.company.com/api/v4
```

非标准端口：

```text
Web 地址：https://git.company.com:8443
API 地址：https://git.company.com:8443/api/v4
```

内网 HTTP：

```text
Web 地址：http://gitlab.local
API 地址：http://gitlab.local/api/v4
```

子路径部署，需允许用户手动确认：

```text
Web 地址：https://code.company.com/gitlab
API 地址：https://code.company.com/gitlab/api/v4
```

由于不同公司部署方式可能不完全一致，API 地址必须允许用户手动修改。

#### 7.7.5 地址校验规则

用户点击“测试连接”前，前端需要做基础校验：

1. Web 地址不能为空。
2. API 地址不能为空。
3. 地址必须以 `http://` 或 `https://` 开头。
4. 如果使用 `http://`，需要弹出安全提示。
5. 如果端口不是 80 或 443，需要允许保存。
6. Token 不能为空。
7. 地址末尾多余 `/` 需要自动规范化。

#### 7.7.6 测试连接逻辑

测试连接时，Rust 后端调用私有 GitLab API：

```text
GET {apiBaseUrl}/user
```

验证成功后，需要读取：

1. 用户 ID
2. username
3. name
4. avatar\_url
5. web\_url，可选
6. email，可选

然后继续测试项目访问能力：

```text
GET {apiBaseUrl}/projects?membership=true&per_page=1
```

测试结果分为：

1. 连接成功
2. Token 无效
3. API 地址错误
4. GitLab 版本过旧或接口不兼容
5. 网络不可达
6. DNS 解析失败
7. TLS 证书错误
8. 代理连接失败
9. 权限不足

#### 7.7.7 Token 权限要求

私有 GitLab 的 Token 权限需要在 UI 中明确提示。

V1 最低建议权限：

1. `read_user`
2. `read_api` 或 `api`
3. `read_repository`，用于读取仓库信息

如果后续支持创建 MR、Issue、推送相关平台操作，则需要：

1. `api`
2. `write_repository`，视功能而定

提示文案建议：

```text
为了同步私有 GitLab 项目，请为 Token 授予 read_user、read_api 或 api、read_repository 权限。Token 仅存储在系统安全凭据中，不会明文写入本地数据库。
```

#### 7.7.8 Group 和 Subgroup 同步

私有 GitLab 常见结构是公司、部门、项目组多级 Group。

同步远程仓库时，需要支持：

1. 用户参与的项目：`/projects?membership=true`
2. 用户有权限访问的 Group
3. Group 下的项目
4. Subgroup 下的项目
5. 分页同步
6. archived 项目筛选
7. visibility 筛选

V1 可以优先同步：

```text
GET {apiBaseUrl}/projects?membership=true&simple=true&per_page=100
```

V2 再扩展完整 Group / Subgroup 树。

#### 7.7.9 Clone URL 处理

私有 GitLab 返回的 Clone URL 可能包含：

1. HTTPS 地址
2. SSH 地址
3. 内网域名
4. 非标准 SSH 端口
5. 自定义 SSH 主机别名

系统需要显示平台返回的原始 Clone URL，并允许用户选择 HTTPS 或 SSH。

如果用户选择 HTTPS：

1. 默认交给系统 Git credential helper。
2. 如果使用 Token 辅助认证，Token 不得写入 remote URL。
3. Clone 成功后 remote URL 必须保持干净。

如果用户选择 SSH：

1. 默认使用用户本机 `~/.ssh/config`。
2. 如果公司 GitLab 使用非标准端口，建议用户配置 SSH Host 别名。
3. V1 不强制提供 SSH Key 管理，只提供错误提示和配置建议。

#### 7.7.10 自签名证书处理

小型公司自建 GitLab 可能使用自签名证书。

设计要求：

1. 默认不跳过 TLS 校验。
2. 遇到证书错误时，明确提示用户证书不受信任。
3. 设置中提供高级选项“允许该实例使用自签名证书”。
4. 不建议提供全局跳过 TLS 校验；如果必须提供，只能作为高级风险选项。
5. 日志中记录证书错误类型，但不记录 Token。

推荐提示：

```text
当前 GitLab 实例的 HTTPS 证书无法验证。建议联系管理员配置可信证书。也可以在高级设置中仅对此实例允许自签名证书，但这会降低连接安全性。
```

#### 7.7.11 私有 GitLab 实例列表

账号管理页需要能展示私有 GitLab 实例信息：

1. 实例名称
2. Web 地址
3. API 地址
4. 当前账号
5. 最近连接状态
6. 最近同步时间
7. 项目数量
8. 是否允许自签名证书
9. 默认 Clone 协议

操作：

1. 测试连接
2. 同步项目
3. 编辑实例配置
4. 删除账号
5. 打开 GitLab 网页

#### 7.7.12 私有 GitLab 错误提示

常见错误和提示：

1. DNS 解析失败

```text
无法解析该 GitLab 地址，请确认公司内网、VPN 或 DNS 配置是否正常。
```

2. 连接超时

```text
连接私有 GitLab 超时，请确认当前网络是否可以访问公司 GitLab，必要时连接 VPN。
```

3. TLS 证书错误

```text
该 GitLab 实例的证书不受信任，请联系管理员配置可信证书，或在高级设置中允许该实例使用自签名证书。
```

4. 404 Not Found

```text
API 地址可能不正确。请确认 API 地址是否类似 https://your-gitlab-domain/api/v4。
```

5. 401 Unauthorized

```text
Token 无效或已过期，请重新生成 Personal Access Token。
```

6. 403 Forbidden

```text
Token 权限不足，请确认已授予 read_user、read_api 或 api、read_repository 权限。
```

#### 7.7.13 私有 GitLab 数据模型补充

账号模型需要支持实例级别配置。

```ts
interface GitLabInstanceConfig {
  accountId: string;
  instanceName: string;
  webBaseUrl: string;
  apiBaseUrl: string;
  allowInsecureHttp: boolean;
  allowInvalidCerts: boolean;
  useSystemProxy: boolean;
  proxyUrl?: string;
  requestTimeoutSeconds: number;
  defaultCloneProtocol: 'https' | 'ssh';
  sshHostAlias?: string;
  apiPathPrefix?: string;
  createdAt: string;
  updatedAt: string;
  lastConnectionStatus?: 'success' | 'failed';
  lastConnectionError?: string;
}
```

如果不单独建表，也可以把这些字段合并到 accounts 表中。但为了后续扩展，建议单独增加 `gitlab_instance_configs` 表。

#### 7.7.14 私有 GitLab 验收标准

该功能完成后，需要满足：

1. 用户可以添加 gitlab.com 账号。
2. 用户可以添加自建 GitLab 账号。
3. 用户可以配置 `https://git.company.com` 类型地址。
4. 用户可以配置 `http://gitlab.local` 类型内网地址，并看到安全提示。
5. 用户可以配置非标准端口地址。
6. 用户可以手动修改 API 地址。
7. 用户可以测试私有 GitLab Token 是否有效。
8. Token 无效、权限不足、API 地址错误时有明确提示。
9. 用户可以同步私有 GitLab 项目。
10. 同步时支持分页。
11. Clone 私有 GitLab 仓库时支持 HTTPS / SSH URL。
12. HTTPS Clone 不会把 Token 写入 remote URL。
13. 自签名证书错误有明确提示。
14. 删除私有 GitLab 账号时会删除对应 Token 凭据。

### 7.8 安全要求

1. Token 不得明文存储在 SQLite。
2. Token 不得出现在日志中。
3. Token 不得出现在 Git 命令日志中。
4. 前端不应长期保存 Token。
5. Rust 后端从系统凭据中读取 Token 后，仅在请求时使用。

---

## 8. 模块三：远程仓库中心

### 8.1 模块目标

统一展示 GitHub、GitLab、Gitee 账号下的远程仓库，支持搜索、筛选、收藏、多选和批量 Clone。

### 8.2 远程仓库模型

```ts
interface RemoteRepository {
  id: string;
  accountId: string;
  platform: GitPlatform;
  owner: string;
  name: string;
  fullName: string;
  description?: string;
  defaultBranch?: string;
  visibility?: 'public' | 'private' | 'internal';
  webUrl: string;
  cloneUrlHttps: string;
  cloneUrlSsh?: string;
  language?: string;
  updatedAtRemote?: string;
  lastSyncAt?: string;
  isFavorite: boolean;
  localRepositoryId?: string;
}
```

### 8.3 仓库同步

#### 8.3.0 同步逻辑注意事项

远程仓库同步必须考虑分页、限流和增量更新，否则账号下仓库较多时容易同步不完整。

同步要求：

1. 所有平台 provider 必须处理分页。
2. 同步时记录每个账号的 `lastSyncAt`。
3. 同步失败时保留旧缓存，不清空已有仓库列表。
4. API 限流时提示用户稍后重试。
5. 支持手动刷新，避免每次进入页面都请求远程 API。
6. 同步过程中显示进度，例如已同步 80 / 350 个仓库。
7. 对于 GitLab Group、Subgroup、GitHub Organization、Gitee 组织仓库，需要在 provider 层分别处理。

#### 8.3.1 同步方式

支持：

1. 同步单个账号仓库
2. 同步当前平台所有账号仓库
3. 同步全部账号仓库

#### 8.3.2 同步内容

每个仓库同步以下信息：

1. 仓库名称
2. Owner / Group
3. 描述
4. 默认分支
5. 可见性
6. Web URL
7. HTTPS Clone URL
8. SSH Clone URL
9. 主要语言
10. 最近更新时间

#### 8.3.3 同步策略

1. 不要每次打开页面都请求远程 API。
2. 远程仓库信息需要缓存到 SQLite。
3. 页面默认展示缓存数据。
4. 用户点击刷新时再请求 API。
5. 支持显示最近同步时间。

### 8.4 仓库列表

列表字段：

1. 选择框
2. 平台
3. 仓库名
4. Owner / Group
5. 所属账号
6. 默认分支
7. 可见性
8. 语言
9. 是否已 Clone
10. 最近更新时间
11. 操作按钮

操作按钮：

1. Clone
2. 查看详情
3. 打开网页
4. 复制 HTTPS 地址
5. 复制 SSH 地址
6. 收藏 / 取消收藏

### 8.5 搜索与筛选

支持：

1. 仓库名称搜索
2. 描述搜索
3. 按平台筛选
4. 按账号筛选
5. 按 Owner / Group 筛选
6. 按 public / private / internal 筛选
7. 按是否已 Clone 筛选
8. 按收藏筛选
9. 按语言筛选，可选
10. 按更新时间排序
11. 按仓库名排序

### 8.6 多选操作

用户多选仓库后，支持：

1. 批量 Clone
2. 批量收藏
3. 批量取消收藏
4. 批量复制 HTTPS Clone URL
5. 批量复制 SSH Clone URL
6. 批量导出仓库列表

V1 必须支持：

1. 批量 Clone
2. 批量收藏
3. 批量复制 Clone URL

### 8.7 仓库详情抽屉

点击仓库后，右侧抽屉展示详情。

详情字段：

1. 仓库名
2. 描述
3. 平台
4. 所属账号
5. Owner / Group
6. 默认分支
7. 可见性
8. Web URL
9. HTTPS Clone URL
10. SSH Clone URL
11. 语言
12. 最近更新时间
13. 本地路径，如果已 Clone

详情操作：

1. Clone
2. 打开网页
3. 复制 HTTPS 地址
4. 复制 SSH 地址
5. 收藏
6. 打开本地仓库，如果已 Clone

---

## 9. 模块四：批量 Clone 中心

### 9.1 模块目标

支持用户从远程仓库中心多选多个项目，一次性加入 Clone 队列，并展示每个任务的实时进度、状态和错误信息。

### 9.2 批量 Clone 流程

```text
远程仓库中心选择多个仓库
        ↓
点击“批量 Clone”
        ↓
弹出 Clone 配置对话框
        ↓
选择目标目录
        ↓
选择目录组织方式
        ↓
选择 Clone 协议：HTTPS / SSH
        ↓
设置并发数量
        ↓
生成 Clone 任务
        ↓
进入 Clone 中心执行
        ↓
任务完成后自动加入本地仓库列表
```

### 9.3 Clone 配置项

V1 配置项：

1. 目标根目录
2. Clone 协议：HTTPS / SSH
3. 目录组织方式
4. 并发数量
5. 已存在目录处理方式
6. Clone 完成后自动添加到本地仓库
7. HTTPS 私有仓库认证方式

HTTPS 私有仓库认证策略：

1. 默认优先使用系统 Git credential helper。
2. 如果用户希望使用已添加账号的 Token 进行 HTTPS Clone，不应把 Token 拼进 clone URL 后持久保存。
3. 推荐后端使用临时 `GIT_ASKPASS`、临时 credential helper 或进程环境变量完成认证。
4. 操作日志和错误日志必须对 Token 做脱敏。
5. Clone 成功后，本地 remote URL 应保持干净，例如 `https://github.com/user/repo.git`，不能保存 `https://token@github.com/user/repo.git`。

V2 可增加：

1. shallow clone
2. depth 数量
3. 是否 clone submodules
4. 是否自动 fetch tags
5. 是否使用指定分支

### 9.4 目录组织方式

#### 9.4.1 扁平模式

```text
~/Projects/
  repo-a/
  repo-b/
  repo-c/
```

优点：简单。 缺点：不同平台或不同 Owner 下同名仓库容易冲突。

#### 9.4.2 按平台和账号分组，推荐默认

```text
~/Projects/
  github/
    username-a/
      repo-a/
  gitlab/
    company-a/
      repo-b/
  gitee/
    username-c/
      repo-c/
```

优点：结构清晰，避免冲突。

#### 9.4.3 按 Owner / Group 分组

```text
~/Projects/
  owner-a/
    repo-a/
  group-b/
    repo-b/
```

适合团队项目较多的用户。

### 9.5 已存在目录处理

用户选择目标目录时，系统应提前检查目标路径。

处理策略：

1. 跳过已存在目录，推荐默认。
2. 如果为空目录，允许继续 Clone。
3. 如果是 Git 仓库，标记为已存在。
4. 如果是非空非 Git 目录，提示冲突。
5. 不建议 V1 支持覆盖目录。

### 9.6 Clone 任务模型

```ts
type CloneTaskStatus =
  | 'pending'
  | 'running'
  | 'success'
  | 'failed'
  | 'cancelled'
  | 'skipped';

interface CloneTask {
  id: string;
  remoteRepositoryId?: string;
  accountId?: string;
  platform?: GitPlatform;
  repoName: string;
  remoteUrl: string;
  targetPath: string;
  status: CloneTaskStatus;
  progress: number;
  stage?: string;
  errorMessage?: string;
  createdAt: string;
  startedAt?: string;
  finishedAt?: string;
}
```

### 9.7 Clone 任务状态

状态说明：

1. pending：等待中
2. running：执行中
3. success：成功
4. failed：失败
5. cancelled：已取消
6. skipped：已跳过

### 9.8 Clone 进度解析

Rust 后端需要监听 `git clone` 的 stdout 和 stderr。

常见阶段：

1. Cloning into
2. Enumerating objects
3. Counting objects
4. Compressing objects
5. Receiving objects
6. Resolving deltas
7. Updating files

前端展示：

1. 总进度百分比
2. 当前阶段
3. 当前仓库
4. 当前任务状态
5. 错误信息

### 9.9 任务队列能力

必须支持：

1. 并发执行
2. 默认并发数 3
3. 用户可配置并发数
4. 取消单个任务
5. 重试失败任务
6. 清空已完成任务
7. 打开目标目录
8. 查看任务日志

V2 可增加：

1. 暂停队列
2. 继续队列
3. 批量取消
4. 批量重试
5. 任务优先级

### 9.10 Clone 中心页面

列表字段：

1. 仓库名
2. 平台
3. 所属账号
4. 目标路径
5. 状态
6. 进度
7. 当前阶段
8. 错误信息
9. 创建时间
10. 操作按钮

操作按钮：

1. 取消
2. 重试
3. 打开目录
4. 查看日志
5. 从列表移除

---

## 10. 模块五：本地仓库管理

### 10.1 模块目标

集中管理本地已经存在或通过 GitView Clone 的 Git 仓库。

### 10.2 添加本地仓库

支持三种方式：

1. 手动选择单个 Git 仓库目录
2. 选择一个父目录，扫描其中所有 Git 仓库
3. Clone 成功后自动加入

### 10.3 仓库扫描

扫描逻辑：

1. 检查目录下是否存在 `.git`。
2. 检查是否是 Git worktree。
3. 检查是否是 submodule。
4. 检查路径是否可访问。
5. 检查是否已经加入本地仓库列表。

V1 可仅支持普通 `.git` 仓库识别。

### 10.4 本地仓库模型

```ts
interface LocalRepository {
  id: string;
  name: string;
  path: string;
  remoteUrl?: string;
  platform?: GitPlatform;
  accountId?: string;
  owner?: string;
  currentBranch?: string;
  defaultBranch?: string;
  status?: RepositoryStatus;
  ahead: number;
  behind: number;
  changedFiles: number;
  lastOpenedAt?: string;
  lastCheckedAt?: string;
  createdAt: string;
  updatedAt: string;
}
```

### 10.5 仓库状态

状态枚举：

```ts
type RepositoryStatus =
  | 'clean'
  | 'modified'
  | 'need_push'
  | 'need_pull'
  | 'conflict'
  | 'detached_head'
  | 'no_remote'
  | 'path_missing';
```

状态含义：

1. clean：工作区干净
2. modified：有未提交变更
3. need\_push：本地领先远程
4. need\_pull：本地落后远程
5. conflict：存在冲突
6. detached\_head：处于 detached HEAD
7. no\_remote：没有远程地址
8. path\_missing：路径不存在

### 10.6 本地仓库列表

列表字段：

1. 选择框
2. 仓库名
3. 本地路径
4. 当前分支
5. 远程地址
6. 平台
7. 所属账号
8. 工作区状态
9. 未提交文件数
10. ahead 数量
11. behind 数量
12. 最后打开时间
13. 操作按钮

操作按钮：

1. 打开仓库
2. Fetch
3. Pull
4. Push
5. 打开目录
6. 打开终端
7. 从列表移除

### 10.7 多仓库批量操作

V1 支持：

1. 批量 Fetch
2. 批量检查状态
3. 批量打开目录，可选
4. 批量从列表移除，仅删除记录，不删除文件

V2 支持：

1. 批量 Pull
2. 批量 Push
3. 批量清理已失效路径
4. 批量导出仓库信息

批量 Pull / Push 风险较高，不建议 V1 默认开放。执行前必须检查：

1. 仓库是否存在未提交变更。
2. 仓库是否存在冲突。
3. 当前分支是否有 upstream。
4. 本地是否 ahead / behind。
5. 是否处于 detached HEAD。

对于 Pull，建议先执行 fetch，再计算 ahead / behind，并在 UI 中展示将要执行的动作。

---

## 11. 模块六：单仓库工作区

### 11.1 模块目标

提供单个 Git 仓库的完整可视化操作界面，覆盖日常开发最常用的 Git 操作。

### 11.2 页面布局

推荐布局：

```text
┌────────────────────────────────────────────────────────────┐
│ 仓库名 / 当前分支 / Fetch / Pull / Push / 更多操作           │
├──────────────────────┬─────────────────────────────────────┤
│ 左侧：文件变更列表     │ 右侧：Diff Viewer                    │
│                      │                                     │
├──────────────────────┴─────────────────────────────────────┤
│ 底部：提交信息 / Stage 操作 / Commit 按钮                   │
└────────────────────────────────────────────────────────────┘
```

也可以采用三栏：

```text
左：变更文件
中：Diff
右：提交面板
```

### 11.3 顶部仓库信息

显示：

1. 仓库名
2. 本地路径
3. 当前分支
4. 远程分支
5. ahead / behind
6. 工作区状态
7. 最近 commit

快捷操作：

1. Fetch
2. Pull
3. Push
4. Branch
5. Open Terminal
6. Open Folder
7. Open Remote URL

### 11.4 文件变更列表

支持文件状态：

1. M：Modified
2. A：Added
3. D：Deleted
4. R：Renamed
5. U：Untracked
6. C：Conflict

功能：

1. 查看 modified / added / deleted / renamed / untracked 文件
2. stage 单个文件
3. unstage 单个文件
4. stage all
5. unstage all
6. discard changes，危险操作，需要确认
7. 打开文件
8. 在文件管理器中显示

### 11.5 Diff 查看

V1 支持：

1. 文本 diff
2. 新增行和删除行高亮
3. unified diff 展示
4. 大文件提示
5. 二进制文件提示
6. 图片文件提示

V2 支持：

1. side-by-side diff
2. Monaco Editor Diff
3. hunk stage
4. line stage
5. 忽略空白变更

### 11.6 Stage / Unstage

必须支持：

1. stage 单个文件
2. unstage 单个文件
3. stage all
4. unstage all

对应 Git 命令：

```bash
git add <file>
git restore --staged <file>
git add -A
git restore --staged .
```

### 11.7 Commit 面板

字段：

1. commit message
2. commit description
3. amend 选项，V2
4. commit 按钮
5. commit and push，V2 可选

提交前校验：

1. 是否存在 staged 文件
2. commit message 是否为空
3. 是否处于冲突状态
4. 是否处于 detached HEAD
5. 是否配置 Git user.name
6. 是否配置 Git user.email

对应 Git 命令：

```bash
git commit -m "message"
```

如果有 description，必须优先使用临时文件方式提交，避免命令行转义、多行文本和特殊字符问题。

```bash
git commit -F <temp_file>
```

临时文件提交完成后需要删除。

### 11.8 分支管理

V1 支持：

1. 查看本地分支
2. 查看远程分支
3. 切换分支
4. 从远程分支 checkout

V2 支持：

1. 新建分支
2. 删除分支
3. 重命名分支
4. merge 分支
5. rebase 分支
6. 设置 upstream

### 11.9 Commit 历史

V1 显示简单提交列表：

1. commit hash
2. message
3. author
4. date
5. branch / tag 标签

V2 增加：

1. commit graph
2. 查看 commit 详情
3. 查看 commit diff
4. 复制 hash
5. revert commit
6. cherry-pick commit

---

## 12. 模块七：Git 操作中心

### 12.1 模块目标

将常见 Git 命令进行可视化封装，并为复杂或危险操作提供解释和确认机制。

### 12.2 V1 基础操作

V1 必须支持：

1. `git status`
2. `git diff`
3. `git add`
4. `git restore --staged`
5. `git commit`
6. `git fetch`
7. `git pull`
8. `git push`
9. `git branch`
10. `git checkout`
11. `git clone`

### 12.3 V2 进阶操作

V2 支持：

1. merge
2. rebase
3. stash
4. tag
5. reset
6. revert
7. cherry-pick
8. clean
9. remote
10. submodule
11. worktree

### 12.4 Stash 管理

功能：

1. 创建 stash
2. 查看 stash 列表
3. apply stash
4. pop stash
5. drop stash
6. 查看 stash diff

### 12.5 Tag 管理

功能：

1. 查看 tags
2. 创建 lightweight tag
3. 创建 annotated tag
4. 删除本地 tag
5. push tag
6. 删除远程 tag

### 12.6 Reset 管理

支持：

1. soft reset
2. mixed reset
3. hard reset

UI 必须解释：

1. soft：保留工作区和暂存区
2. mixed：保留工作区，清空暂存区
3. hard：丢弃所有改动

hard reset 必须二次确认。

---

## 13. 模块八：平台协作功能

### 13.1 模块目标

提供 GitHub、GitLab、Gitee 上 PR / MR、Issue、CI 的轻量集成。

该模块不建议 V1 完整实现，但需要在架构上预留扩展能力。

### 13.2 PR / MR 功能

支持平台：

1. GitHub Pull Request
2. GitLab Merge Request
3. Gitee Pull Request

功能：

1. 查看我创建的 PR / MR
2. 查看分配给我的 PR / MR
3. 查看当前仓库的 PR / MR
4. 创建 PR / MR
5. 查看状态
6. 打开浏览器查看详情

V1 可做快捷入口：

1. 本地分支 push 后，提供“创建 Pull Request / Merge Request”按钮。
2. 点击后打开浏览器进入对应平台网页。

### 13.3 Issue 功能

后续支持：

1. 查看 issue 列表
2. 按仓库筛选
3. 按 assigned to me 筛选
4. 打开 issue
5. 从 issue 创建分支

### 13.4 CI 状态

后续支持：

1. 最近一次 pipeline / workflow 状态
2. commit 对应 CI 状态
3. CI 失败时打开网页查看详情

---

## 14. 模块九：设置中心

### 14.1 通用设置

字段：

1. 启动时打开上次仓库
2. 启动时自动检查仓库状态
3. 默认项目目录
4. 默认 Clone 协议：HTTPS / SSH
5. 默认 Clone 并发数量
6. 默认目录组织方式
7. 语言：中文 / 英文
8. 主题：浅色 / 深色 / 跟随系统

### 14.2 Git 设置

字段：

1. Git 可执行文件路径
2. 自动检测 Git
3. Git 用户名
4. Git 邮箱
5. 默认 pull 策略
6. 默认 push 策略
7. 是否启用 Git LFS 检测

需要支持检测命令：

```bash
git --version
git config --global user.name
git config --global user.email
```

### 14.3 账号与安全

功能：

1. 管理保存的 Token
2. 重新验证账号
3. 删除账号凭据
4. 查看凭据存储状态
5. 数据库加密，可选

### 14.4 网络设置

字段：

1. HTTP Proxy
2. HTTPS Proxy
3. 是否使用系统代理
4. API 超时时间
5. Clone 超时时间
6. 是否跳过 SSL 校验，不建议默认提供，必要时高级设置中提供

### 14.5 外部工具

支持配置：

1. 默认编辑器
2. 默认终端
3. 默认文件管理器
4. 使用 VS Code 打开仓库
5. 使用 Cursor 打开仓库
6. 使用 JetBrains IDE 打开仓库

---

## 15. 模块十：日志与问题诊断

### 15.1 模块目标

记录用户执行的关键操作、Git 命令结果和错误信息，帮助用户定位问题。

### 15.2 操作日志

记录操作：

1. clone
2. fetch
3. pull
4. push
5. commit
6. checkout
7. branch
8. merge
9. rebase
10. 账号验证
11. API 请求错误
12. 仓库扫描

### 15.3 日志字段

字段：

1. 时间
2. 操作类型
3. 仓库名
4. 命令，敏感信息脱敏
5. 状态
6. 耗时
7. 输出摘要
8. 错误摘要
9. 查看详情

### 15.4 敏感信息脱敏

日志中不得出现：

1. Token
2. 密码
3. OAuth refresh token
4. SSH 私钥内容
5. 带 token 的 clone URL

示例：

错误示例：

```bash
git clone https://token123@github.com/user/repo.git
```

正确示例：

```bash
git clone https://github.com/user/repo.git
```

### 15.5 错误诊断映射

常见错误：

#### Authentication failed

提示：

```text
认证失败，请检查 Token、SSH Key 或账号权限。
```

#### Repository not found

提示：

```text
仓库不存在，或当前账号没有访问权限。
```

#### Permission denied (publickey)

提示：

```text
SSH Key 未配置或无权限，请检查本机 SSH 配置。
```

#### Could not resolve host

提示：

```text
无法解析主机，请检查网络连接、DNS 或代理设置。
```

#### path already exists and is not an empty directory

提示：

```text
目标目录已存在且不为空，请更换目录或选择跳过该仓库。
```

---

## 16. 数据库设计

### 16.1 accounts

```sql
CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  platform TEXT NOT NULL,
  web_base_url TEXT NOT NULL,
  api_base_url TEXT NOT NULL,
  username TEXT NOT NULL,
  display_name TEXT,
  avatar_url TEXT,
  token_key TEXT NOT NULL,
  is_default INTEGER DEFAULT 0,
  enabled INTEGER DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_sync_at TEXT
);
```

### 16.2 remote\_repositories

```sql
CREATE TABLE remote_repositories (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  platform TEXT NOT NULL,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  full_name TEXT NOT NULL,
  description TEXT,
  default_branch TEXT,
  visibility TEXT,
  web_url TEXT,
  clone_url_https TEXT,
  clone_url_ssh TEXT,
  language TEXT,
  updated_at_remote TEXT,
  last_sync_at TEXT,
  is_favorite INTEGER DEFAULT 0,
  local_repository_id TEXT
);
```

### 16.3 local\_repositories

```sql
CREATE TABLE local_repositories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  remote_url TEXT,
  platform TEXT,
  account_id TEXT,
  owner TEXT,
  current_branch TEXT,
  default_branch TEXT,
  status TEXT,
  ahead INTEGER DEFAULT 0,
  behind INTEGER DEFAULT 0,
  changed_files INTEGER DEFAULT 0,
  last_opened_at TEXT,
  last_checked_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### 16.4 clone\_tasks

```sql
CREATE TABLE clone_tasks (
  id TEXT PRIMARY KEY,
  remote_repository_id TEXT,
  account_id TEXT,
  platform TEXT,
  repo_name TEXT NOT NULL,
  remote_url TEXT NOT NULL,
  target_path TEXT NOT NULL,
  status TEXT NOT NULL,
  progress INTEGER DEFAULT 0,
  stage TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT
);
```

### 16.5 operation\_logs

```sql
CREATE TABLE operation_logs (
  id TEXT PRIMARY KEY,
  operation_type TEXT NOT NULL,
  repository_id TEXT,
  command TEXT,
  status TEXT NOT NULL,
  output TEXT,
  error TEXT,
  duration_ms INTEGER,
  created_at TEXT NOT NULL
);
```

### 16.6 settings

```sql
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

---

## 17. Rust 后端设计

### 17.1 目录结构

```text
src-tauri/src/
├─ main.rs
├─ commands/
│  ├─ accounts.rs
│  ├─ remote_repositories.rs
│  ├─ local_repositories.rs
│  ├─ clone_tasks.rs
│  ├─ git.rs
│  ├─ settings.rs
│  └─ logs.rs
│
├─ services/
│  ├─ credential_service.rs
│  ├─ account_service.rs
│  ├─ github_service.rs
│  ├─ gitlab_service.rs
│  ├─ gitee_service.rs
│  ├─ git_cli_service.rs
│  ├─ git_reader_service.rs
│  ├─ clone_task_service.rs
│  ├─ repository_service.rs
│  ├─ settings_service.rs
│  └─ log_service.rs
│
├─ db/
│  ├─ mod.rs
│  ├─ migrations.rs
│  └─ pool.rs
│
├─ models/
│  ├─ account.rs
│  ├─ repository.rs
│  ├─ clone_task.rs
│  ├─ git.rs
│  └─ settings.rs
│
├─ errors.rs
└─ utils/
   ├─ path.rs
   ├─ process.rs
   ├─ redact.rs
   └─ time.rs
```

### 17.2 Provider 设计

为 GitHub、GitLab、Gitee 设计统一接口。

```rust
trait GitHostingProvider {
    async fn get_current_user(&self) -> Result<UserProfile>;
    async fn list_repositories(&self) -> Result<Vec<RemoteRepository>>;
    async fn search_repositories(&self, keyword: String) -> Result<Vec<RemoteRepository>>;
    async fn get_repository_detail(&self, repo_id: String) -> Result<RemoteRepository>;
}
```

每个平台独立实现：

1. `GithubService`
2. `GitlabService`
3. `GiteeService`

不要过度抽象平台差异，统一接口只覆盖公共能力。

### 17.3 Git CLI Service

封装所有 Git 命令。

建议方法：

```rust
GitCliService
  detect_git()
  clone_repository()
  status()
  diff()
  stage_file()
  unstage_file()
  stage_all()
  unstage_all()
  commit()
  fetch()
  pull()
  push()
  list_branches()
  checkout_branch()
  create_branch()
  delete_branch()
  log()
```

### 17.4 Clone Task Service

职责：

1. 生成 clone task
2. 控制并发
3. 执行 git clone
4. 解析进度
5. 发送进度事件给前端
6. 记录日志
7. 失败重试
8. 成功后创建 local repository 记录

### 17.5 Credential Service

职责：

1. 保存 Token
2. 读取 Token
3. 删除 Token
4. 检查凭据是否存在

要求：

1. Token 不进入日志。
2. Token 不返回给前端。
3. SQLite 仅保存 token\_key。

---

## 18. Tauri 命令设计

### 18.1 账号相关

```ts
add_account(payload)
test_account_connection(payload)
list_accounts()
update_account(payload)
delete_account(accountId)
set_default_account(accountId)
sync_account_repositories(accountId)
```

### 18.2 远程仓库相关

```ts
list_remote_repositories(filters)
search_remote_repositories(keyword)
refresh_remote_repositories(accountId)
get_remote_repository_detail(repoId)
toggle_favorite_remote_repository(repoId)
```

### 18.3 本地仓库相关

```ts
add_local_repository(path)
scan_local_repositories(rootPath)
list_local_repositories(filters)
remove_local_repository(repoId)
refresh_local_repository_status(repoId)
refresh_all_local_repository_status()
open_repository_folder(repoId)
open_repository_in_terminal(repoId)
```

### 18.4 Clone 相关

```ts
create_clone_tasks(payload)
list_clone_tasks()
start_clone_tasks(taskIds)
cancel_clone_task(taskId)
retry_clone_task(taskId)
clear_finished_clone_tasks()
```

### 18.5 Git 操作相关

```ts
git_status(repoId)
git_diff(repoId, filePath)
git_stage_file(repoId, filePath)
git_unstage_file(repoId, filePath)
git_stage_all(repoId)
git_unstage_all(repoId)
git_commit(repoId, message, description)
git_fetch(repoId)
git_pull(repoId)
git_push(repoId)
git_list_branches(repoId)
git_checkout_branch(repoId, branch)
git_create_branch(repoId, branch, checkout)
git_delete_branch(repoId, branch)
git_log(repoId, page, pageSize)
```

### 18.6 设置相关

```ts
get_settings()
update_settings(payload)
detect_git()
set_git_path(path)
```

---

## 19. 前端设计

### 19.1 前端目录结构

```text
src/
├─ App.vue
├─ main.ts
├─ router/
│  └─ index.ts
│
├─ layouts/
│  └─ AppLayout.vue
│
├─ pages/
│  ├─ Dashboard.vue
│  ├─ RemoteRepositories.vue
│  ├─ LocalRepositories.vue
│  ├─ RepositoryDetail.vue
│  ├─ CloneCenter.vue
│  ├─ Accounts.vue
│  ├─ Logs.vue
│  └─ Settings.vue
│
├─ components/
│  ├─ common/
│  │  ├─ PlatformBadge.vue
│  │  ├─ StatusTag.vue
│  │  ├─ EmptyState.vue
│  │  └─ ConfirmDangerDialog.vue
│  │
│  ├─ layout/
│  │  ├─ Sidebar.vue
│  │  └─ Topbar.vue
│  │
│  ├─ account/
│  │  ├─ AccountCard.vue
│  │  ├─ AccountFormDialog.vue
│  │  └─ AccountSwitcher.vue
│  │
│  ├─ repository/
│  │  ├─ RemoteRepoTable.vue
│  │  ├─ LocalRepoTable.vue
│  │  ├─ RepoDetailDrawer.vue
│  │  └─ RepoStatusOverview.vue
│  │
│  ├─ clone/
│  │  ├─ BatchCloneDialog.vue
│  │  ├─ CloneTaskTable.vue
│  │  └─ CloneProgress.vue
│  │
│  └─ git/
│     ├─ GitFileChanges.vue
│     ├─ DiffViewer.vue
│     ├─ CommitPanel.vue
│     ├─ BranchSelector.vue
│     ├─ CommitHistory.vue
│     └─ CommitGraph.vue
│
├─ stores/
│  ├─ account.ts
│  ├─ remoteRepository.ts
│  ├─ localRepository.ts
│  ├─ cloneTask.ts
│  └─ settings.ts
│
├─ api/
│  ├─ tauri.ts
│  ├─ account.api.ts
│  ├─ repository.api.ts
│  ├─ git.api.ts
│  └─ cloneTask.api.ts
│
└─ types/
   ├─ account.ts
   ├─ repository.ts
   ├─ cloneTask.ts
   ├─ git.ts
   └─ settings.ts
```

### 19.2 UI 组件库

使用 Element Plus。

主要使用组件：

1. `el-container`
2. `el-menu`
3. `el-table`
4. `el-dialog`
5. `el-drawer`
6. `el-form`
7. `el-input`
8. `el-select`
9. `el-button`
10. `el-progress`
11. `el-tag`
12. `el-card`
13. `el-tabs`
14. `el-alert`
15. `el-notification`
16. `el-message`

### 19.3 状态管理

使用 Pinia。

建议 store：

1. `accountStore`
2. `remoteRepositoryStore`
3. `localRepositoryStore`
4. `cloneTaskStore`
5. `settingsStore`
6. `appStore`

### 19.4 异步状态

如果使用 `@tanstack/vue-query`，建议用于：

1. 加载远程仓库列表
2. 加载本地仓库列表
3. 加载账号列表
4. 刷新仓库状态
5. 加载操作日志

---

## 20. 核心用户流程

### 20.1 首次使用流程

```text
打开软件
  ↓
检测 Git 是否安装
  ↓
如果未安装，提示用户安装 Git
  ↓
进入欢迎页
  ↓
添加 GitHub / GitLab / Gitee 账号
  ↓
测试连接
  ↓
保存账号
  ↓
同步远程仓库
  ↓
进入远程仓库中心
```

### 20.2 批量 Clone 流程

```text
进入远程仓库中心
  ↓
选择平台 / 账号
  ↓
搜索或筛选仓库
  ↓
勾选多个仓库
  ↓
点击“批量 Clone”
  ↓
选择目标目录
  ↓
选择目录组织方式
  ↓
选择 HTTPS / SSH
  ↓
设置并发数
  ↓
创建任务
  ↓
进入 Clone 中心
  ↓
查看进度
  ↓
完成后自动加入本地仓库
```

### 20.3 日常提交流程

```text
进入本地仓库列表
  ↓
打开仓库
  ↓
查看文件变更
  ↓
选择文件查看 diff
  ↓
stage 文件
  ↓
填写 commit message
  ↓
commit
  ↓
push
```

### 20.4 多仓库同步流程

```text
进入本地仓库列表
  ↓
选择多个仓库
  ↓
点击批量 Fetch
  ↓
系统加入任务队列
  ↓
显示执行结果
```

---

## 21. Git 命令映射

### 21.1 Git 检测

```bash
git --version
```

### 21.2 Clone

```bash
git clone <remote_url> <target_path>
```

可选参数：

```bash
git clone --depth <depth> <remote_url> <target_path>
git clone --recurse-submodules <remote_url> <target_path>
```

### 21.3 Status

```bash
git status --porcelain=v1 -b
```

### 21.4 Diff

```bash
git diff -- <file>
git diff --cached -- <file>
```

### 21.5 Stage / Unstage

```bash
git add <file>
git add -A
git restore --staged <file>
git restore --staged .
```

### 21.6 Commit

```bash
git commit -m "message"
```

推荐使用临时文件提交复杂 message：

```bash
git commit -F <temp_file>
```

### 21.7 Fetch / Pull / Push

```bash
git fetch --all --prune
git pull
git push
```

### 21.8 Branch

```bash
git branch --list
git branch -r
git checkout <branch>
git checkout -b <branch>
git branch -d <branch>
```

### 21.9 Log

```bash
git log --pretty=format:"%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s" --date=iso
```

---

## 22. 非功能需求

### 22.1 性能要求

1. 应用启动应尽可能快速。
2. 本地仓库列表应支持至少 500 个仓库。
3. 远程仓库列表应支持至少 5000 条数据分页或虚拟滚动。
4. 批量 Clone 默认并发数不超过 3。
5. 大型仓库 diff 需要限制文件大小，避免 UI 卡死。
6. 仓库状态刷新、远程同步、Clone、Fetch、Pull、Push 都不得阻塞 UI 主线程。
7. 远程仓库列表和本地仓库列表需要支持分页、搜索或虚拟滚动。

### 22.2 安全要求

1. Token 不存 SQLite 明文。
2. 日志脱敏。
3. Git 命令参数中不得暴露 Token。
4. 删除账号时删除对应凭据。
5. 前端不长期保存敏感信息。

### 22.3 兼容性要求

1. macOS 支持常见 Apple Silicon 和 Intel 设备。
2. Windows 支持 Windows 10 / 11。
3. Linux 优先支持 Ubuntu。
4. Git 版本过低时提示升级。

### 22.4 稳定性要求

1. Git 操作失败不能导致应用崩溃。
2. Clone 任务失败后可重试。
3. 应用关闭后，未完成任务应能标记为 interrupted 或 failed。
4. 数据库迁移失败需要回滚或提示。

### 22.5 可维护性要求

1. 平台 API provider 独立实现。
2. Git CLI 封装统一入口。
3. 所有 Tauri command 参数需要类型定义。
4. 前后端共享接口尽量保持稳定。
5. 数据库迁移脚本版本化。

---

## 23. MVP 开发任务拆解

### 23.1 阶段一：项目基础搭建

任务：

1. 创建 Tauri + Vue + TypeScript 项目
2. 集成 Element Plus
3. 集成 Vue Router
4. 集成 Pinia
5. 创建主布局
6. 创建左侧菜单
7. 创建基础页面空壳
8. Rust 后端初始化
9. SQLite 初始化
10. 日志系统初始化

### 23.2 阶段二：Git 检测与设置

任务：

1. 实现 Git 检测命令
2. 读取 Git 版本
3. 读取 global user.name
4. 读取 global user.email
5. 设置 Git 路径
6. 设置默认项目目录

### 23.3 阶段三：账号管理

任务：

1. accounts 表
2. credential service
3. 添加账号 UI
4. GitHub 连接测试
5. GitLab 连接测试
6. Gitee 连接测试
7. 账号列表
8. 删除账号
9. 设置默认账号

### 23.4 阶段四：远程仓库同步

任务：

1. remote\_repositories 表
2. GitHub 仓库同步
3. GitLab 仓库同步
4. Gitee 仓库同步
5. 远程仓库列表 UI
6. 搜索
7. 筛选
8. 收藏
9. 仓库详情抽屉

### 23.5 阶段五：批量 Clone

任务：

1. clone\_tasks 表
2. BatchCloneDialog
3. 目录选择
4. 目录组织方式
5. 创建 clone tasks
6. Clone 队列服务
7. 进度解析
8. 前端进度展示
9. 取消任务
10. 重试任务
11. 成功后加入本地仓库

### 23.6 阶段六：本地仓库管理

任务：

1. local\_repositories 表
2. 添加本地仓库
3. 本地仓库列表
4. 刷新仓库状态
5. 批量 fetch
6. 打开目录
7. 打开终端

### 23.7 阶段七：单仓库工作区

任务：

1. RepositoryDetail 页面
2. git status
3. 文件变更列表
4. git diff
5. stage 文件
6. unstage 文件
7. stage all
8. unstage all
9. commit
10. fetch
11. pull
12. push
13. 分支列表
14. 切换分支

### 23.8 阶段八：日志与诊断

任务：

1. operation\_logs 表
2. 记录 Git 命令日志
3. 日志列表页面
4. 日志详情
5. 错误脱敏
6. 常见错误翻译

---

## 24. MVP 验收标准

MVP 完成后，应满足以下标准：

1. 用户可以成功启动应用。
2. 应用可以检测系统 Git 是否安装。
3. 用户可以添加 GitHub 账号并同步仓库。
4. 用户可以添加 GitLab 账号并同步仓库。
5. 用户可以添加 Gitee 账号并同步仓库。
6. 用户可以在远程仓库中心搜索和筛选仓库。
7. 用户可以多选多个仓库并批量 Clone。
8. Clone 任务可以显示进度。
9. Clone 失败后可以重试。
10. Clone 成功后仓库进入本地仓库列表。
11. 用户可以手动添加本地 Git 仓库。
12. 用户可以查看本地仓库状态。
13. 用户可以查看文件变更和 diff。
14. 用户可以 stage / unstage 文件。
15. 用户可以提交 commit。
16. 用户可以执行 fetch / pull / push。
17. 用户可以查看分支并切换分支。
18. 用户可以查看操作日志。
19. Token 不会出现在数据库明文字段中。
20. Token 不会出现在日志中。

---

## 25. 关键风险与应对方案

### 25.1 Git 认证复杂

风险：HTTPS Token、系统 credential helper、SSH Key、代理都可能影响 clone / pull / push。

应对：

1. V1 优先让 Git CLI 处理认证。
2. 平台 API Token 只用于获取仓库列表。
3. Clone 支持 HTTPS 和 SSH URL。
4. 认证失败时提供明确提示。

### 25.2 Linux 凭据存储兼容性

风险：Linux 上 Secret Service / libsecret 环境不一定完整。

应对：

1. 优先使用系统 keyring。
2. 检测不可用时提示用户安装依赖。
3. 后续可提供加密本地存储作为 fallback。

### 25.3 Git 输出解析不稳定

风险：不同 Git 版本或语言环境可能导致输出不同。

应对：

1. 尽量使用 porcelain 格式。
2. Git status 使用 `--porcelain`。
3. Git log 使用自定义分隔符。
4. 进度解析失败时仍保留原始日志。

### 25.4 大仓库性能问题

风险：大型仓库 status / diff / log 可能很慢。

应对：

1. 后端异步执行。
2. 前端展示 loading。
3. 大文件 diff 给出提示。
4. log 分页加载。
5. 后续用 git2-rs 优化读取性能。

### 25.5 批量 Clone 并发导致网络或磁盘压力

风险：并发过高导致失败率增加。

应对：

1. 默认并发数为 3。
2. 设置最大并发数上限，例如 5 或 8。
3. 支持失败重试。
4. 记录每个任务日志。

---

## 26. 最终产品蓝图

```text
GitView
│
├─ 首页仪表盘
│  ├─ 仓库概览
│  ├─ 最近打开
│  └─ 最近任务
│
├─ 账号管理
│  ├─ GitHub
│  ├─ GitLab
│  ├─ Gitee
│  └─ 自建 GitLab
│
├─ 远程仓库
│  ├─ 仓库同步
│  ├─ 搜索筛选
│  ├─ 多选操作
│  └─ 批量 Clone
│
├─ Clone 中心
│  ├─ 任务队列
│  ├─ 进度展示
│  ├─ 取消任务
│  └─ 失败重试
│
├─ 本地仓库
│  ├─ 添加仓库
│  ├─ 扫描目录
│  ├─ 状态检查
│  └─ 批量 Fetch
│
├─ 仓库工作区
│  ├─ 文件变更
│  ├─ Diff
│  ├─ Stage
│  ├─ Commit
│  ├─ Branch
│  ├─ Pull
│  └─ Push
│
├─ 进阶 Git
│  ├─ Stash
│  ├─ Tag
│  ├─ Merge
│  ├─ Rebase
│  ├─ Cherry-pick
│  └─ Reset
│
├─ 平台协作
│  ├─ PR / MR
│  ├─ Issue
│  └─ CI 状态
│
└─ 设置与日志
   ├─ Git 设置
   ├─ 代理设置
   ├─ 主题设置
   ├─ 外部工具
   └─ 操作日志
```

---

## 27. 结论

GitView 第一阶段应专注于以下四个核心体验：

1. **多账号添加稳定**
2. **远程仓库同步稳定**
3. **批量 Clone 稳定**
4. **基础 Git 操作稳定**

第一版不建议投入过多精力在完整 commit graph、复杂冲突编辑器、完整 PR 管理、AI 功能或插件机制上。

最小可用产品的核心路径应是：

```text
添加账号 → 同步仓库 → 多选 Clone → 打开本地仓库 → 查看变更 → Commit → Push
```

只要这条链路稳定、顺滑、轻量，GitView 就已经具备明确的产品价值。

