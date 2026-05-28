<!--
==============================================================================
Sync Impact Report (宪法同步影响报告)
==============================================================================
Version change: TEMPLATE (0.0.0) → 1.0.0
Bump rationale: Initial ratification — first formal codification of project
principles from template placeholders to concrete content. MAJOR bump from
template (0.0.0) to ratified (1.0.0) marks the constitution becoming binding.

Modified principles (placeholder → concrete name):
- PRINCIPLE_1 → I. 代码质量优先 (Code Quality First)
- PRINCIPLE_2 → II. 中文注释规范 (Chinese Documentation Standard)
- PRINCIPLE_3 → III. 文件操作安全 (File Operation Safety) — NON-NEGOTIABLE
- PRINCIPLE_4 → IV. 方案确认优先 (Plan-First Approval) — NON-NEGOTIABLE
- PRINCIPLE_5 (template slot) → REMOVED (user specified 4 principles)

Added sections:
- SECTION_2_NAME → 工程约束 (Engineering Constraints)
- SECTION_3_NAME → 开发工作流 (Development Workflow)

Removed sections: None (5th principle slot intentionally removed)

Templates requiring updates:
- ✅ .specify/templates/plan-template.md (Constitution Check gates updated to
  reflect the 4 ratified principles)
- ✅ .specify/templates/spec-template.md (reviewed — no changes required;
  principles do not add new mandatory spec sections)
- ✅ .specify/templates/tasks-template.md (reviewed — no changes required;
  principles do not introduce new task categories beyond existing structure)
- ✅ CLAUDE.md (reviewed — currently a stub pointing to plan; no conflicts)

Follow-up TODOs: None
==============================================================================
-->

# GitView Constitution

GitView 是一个轻量级跨平台 Git 可视化客户端项目，本宪法是项目所有开发工作的最高
指导文件，规定了不可妥协的代码质量、注释、文件操作与方案审批规则。

## Core Principles

### I. 代码质量优先 (Code Quality First)

所有代码 MUST 遵循通用代码格式与质量规范，确保整个项目代码风格统一、可读性强、
可维护性高。具体要求：

- 所有源代码 MUST 通过项目所选语言生态的标准格式化工具（如 Rust `rustfmt`、
  JavaScript/TypeScript `prettier`、Python `black`）格式化后方可提交。
- 所有源代码 MUST 通过对应的静态分析工具（如 Rust `clippy`、
  JavaScript/TypeScript `eslint`、Python `ruff`/`pylint`）检查，禁止存在
  ERROR 级别问题；WARNING 级别问题需在 PR 描述中提供豁免理由。
- 函数、类、变量、文件命名 MUST 使用所选语言生态推荐的命名规范，禁止在同一
  语言/模块内混用风格。
- 单个函数长度 SHOULD 控制在 50 行以内，单个文件长度 SHOULD 控制在 500 行
  以内；超出阈值时必须在 PR 描述中说明原因，并接受评审者的拆分建议。
- 禁止提交注释掉的废弃代码、临时调试输出（如 `println!`、`console.log`、
  `dbg!`）以及无关联跟踪 issue 的 TODO/FIXME。

**Rationale**: 代码质量是项目长期可维护性的基石。统一的格式与规范让团队成员可以
快速理解彼此的代码，降低 Bug 引入与回归风险；强制的工具化检查避免依赖个人自觉。

### II. 中文注释规范 (Chinese Documentation Standard)

所有源代码 MUST 包含详尽的中文注释，**中文注释行数与非空代码行数的比值 MUST
按单个源文件粒度独立达到 30% 及以上**。具体要求：

- 比例计算口径：`(单文件中文注释行数) / (单文件非空代码行数) >= 0.3`。
  注释行包括独立注释行与行尾注释；非空代码行排除纯空行与纯注释行。
- 每个模块/文件头部 MUST 包含中文文件级注释，说明：文件用途、负责的业务领域、
  关键依赖关系。
- 每个公共函数/方法/类 MUST 包含中文文档注释（如 Rust `///`、
  TypeScript JSDoc、Python docstring），覆盖：用途说明、参数含义、返回值
  含义、异常/错误情形、典型使用示例（若适用）。
- 复杂业务逻辑、非显而易见的算法分支、性能敏感代码段 MUST 在行内或邻近位置
  使用中文解释 **WHY**（为什么这样做）而非 **WHAT**（做了什么）。
- 注释 MUST 与代码同步更新；发现代码与注释不一致时，必须立即修正注释或代码，
  禁止将"注释与代码不一致"作为可接受的中间状态合并到主分支。
- 自动生成的代码（如 protobuf、ORM derive 输出）可豁免注释比例要求，但 PR
  描述中 MUST 标注豁免原因。

**Rationale**: 项目核心团队以中文为主要工作语言。详细的中文注释能够最大程度
降低团队成员理解代码的认知负担，加快新成员上手，并在代码漂移时保留设计意图。
按文件粒度核算可防止"在某些文件超额、在另一些文件偷懒"的平均值游戏。

### III. 文件操作安全 (File Operation Safety) — NON-NEGOTIABLE

为防止误操作导致用户工作成果丢失，文件操作 MUST 遵循以下分级授权策略：

- **读取操作（Read）**：所有对项目内文件的读取操作 **无需用户额外确认**，
  代理可直接执行。
- **写入操作（Write/Edit/Create）**：所有对项目内文件的新建、修改、覆盖等
  写入操作 **无需用户额外确认**，代理可直接执行；但每次写入应附带清晰的变更
  说明，以便用户审阅。
- **删除操作（Delete）**：所有对项目内文件、目录的删除操作 MUST 在执行前
  显式向用户提出确认请求，并在用户明确批准（如"同意"、"确认"、"go ahead"）
  后方可执行。
- **删除操作的范围**：本规则覆盖一切可能导致文件不可恢复或工作成果丢失的
  操作，包括但不限于：
  - 操作系统级删除：`rm`、`rm -rf`、`Remove-Item`、IDE 删除快捷键。
  - Git 破坏性命令：`git clean -fd`、`git reset --hard`、`git checkout --`、
    `git restore --`、`git branch -D`、`git stash drop`、`git push --force`
    （对远程历史的破坏）。
  - 批量替换/覆盖：通过脚本一次性覆盖大量文件、清空目录等行为。
- **删除前置告知**：在请求删除确认时 MUST 同时告知：被删除文件/目录的完整
  路径列表、被删除内容的概要（如代码量、最近修改时间）、删除原因、可恢复性
  评估（是否在 Git 历史中可找回）。
- 该原则不可豁免，即使在自动化批处理、CI 脚本或用户批准的方案执行过程中，
  涉及上述破坏性操作时仍 MUST 单独请求确认。

**Rationale**: 文件读写是高频低风险操作，强制确认会严重影响开发效率；而文件
删除是高风险且常常不可逆的操作，一次误操作可能导致数小时甚至数天的工作成果
丢失。分级授权在效率与安全之间取得最佳平衡。Git 破坏性命令同样会导致工作丢失，
故纳入同等保护范围。

### IV. 方案确认优先 (Plan-First Approval) — NON-NEGOTIABLE

所有非平凡变更必须遵循"先方案、后实施"的原则：

- 任何代理自动生成的实现方案（包括但不限于：架构设计、数据模型、API 设计、
  技术选型、重构计划、批量修改方案、依赖引入、迁移脚本）MUST 在执行任何代码
  变更之前完整呈现给用户。
- 用户 MUST 收到明确的方案展示，并通过显式回复（如"同意"、"确认"、"批准"、
  "go ahead"）授权后，代理方可启动实施。
- 在用户未明确批准前，**禁止任何具有副作用的实施动作**，包括但不限于：
  创建新文件、修改现有文件、运行迁移脚本、调用外部 API、修改 Git 提交、
  推送到远程、安装/升级/卸载依赖。
- **例外情形（无需方案确认）**：
  - 纯查询/读取操作（如阅读代码、查询 Git 日志、搜索文件）。
  - 向用户呈现信息或方案本身的动作。
  - 根据用户已明确批准方案的直接执行步骤（且无偏离）。
- 用户批准后若代理在实施过程中发现需要偏离原方案（如发现遗漏的依赖、需要调整
  数据结构、需要引入未在方案中提及的第三方库），MUST 立即暂停实施并再次请求
  用户确认偏离后的新方案。
- 方案呈现 MUST 包含：变更目标、关键步骤列表、涉及的文件清单、潜在风险与
  回退方案；对于复杂方案，应通过 `specs/<feature>/plan.md` 持久化以便追溯。

**Rationale**: 该原则保证用户对项目方向保持完全控制。代理生成的方案可能因上下文
不足、对业务理解偏差或工具误判而走偏，提前确认可以低成本地纠正方向，避免在
错误方案上投入实施成本。同时，方案先行可在日后通过 Git 历史与方案文档追溯
所有重大决策。

## 工程约束 (Engineering Constraints)

### 技术栈约束

- 桌面客户端 MUST 基于 **Tauri + Rust + Vue** 实现，以兼顾轻量与跨平台。
- 支持的目标平台 MUST 包括 **macOS、Windows、Ubuntu (Linux)**；任何核心
  功能 MUST 在三个平台上均可用，禁止单平台独占功能进入主分支。
- 跨平台兼容性 MUST 通过 CI 中的多平台构建与基本运行测试自动验证，禁止
  "单平台开发、其他平台后续适配"的工作模式。
- 涉及与远程仓库平台交互的核心能力 MUST 支持 **GitHub、GitLab、Gitee**
  三大平台；新增平台支持 SHOULD 通过抽象层扩展而非分支判断。

### 代码格式化与静态分析

- Rust 代码 MUST 通过 `cargo fmt --check` 与 `cargo clippy -- -D warnings`
  的 CI 检查。
- Vue/TypeScript/JavaScript 代码 MUST 通过 `prettier --check` 与
  `eslint --max-warnings 0` 的 CI 检查。
- 任何新增语言生态 MUST 在引入时同步配置对应的格式化与静态分析工具，并将其
  纳入 CI。

### 注释比例验证

- CI MUST 包含中文注释比例检查脚本，对每个新增/修改的源文件按 Principle II
  规定的口径计算并断言 `ratio >= 0.3`。
- 检查失败的 PR MUST 在修复后才能合并；不得通过"强制合并"绕过。
- 自动生成代码的豁免清单 MUST 显式声明在 `.specify/` 或项目根级配置中。

### 依赖与安全

- 引入第三方依赖 MUST 在 PR 描述中说明：用途、许可证类型、维护活跃度（最近
  一次提交、Stars/Downloads 量级）。
- 禁止引入与项目许可证不兼容的依赖（如未经评估的 GPL 类强传染性许可证）。
- 引入显著影响最终包体积（>10MB）的二进制依赖 MUST 触发额外的方案确认（按
  Principle IV）。
- 涉及用户凭据（如 GitHub/GitLab/Gitee 的 token、密码）的存储 MUST 使用
  操作系统原生密钥库（macOS Keychain、Windows Credential Manager、
  Linux Secret Service），**禁止明文存储**于配置文件、日志或代码中。
- 与远程仓库平台的通信 MUST 使用 HTTPS 或 SSH，禁止使用未加密的 HTTP 协议。

## 开发工作流 (Development Workflow)

### 变更评审流程

1. 代理收到任务后，MUST 先按 Principle IV 生成方案并等待用户显式确认。
2. 用户确认后，代理 MUST 按 Principle I/II 编写代码，并完成自检（格式化、
   静态分析、注释比例计算）。
3. 涉及文件删除或 Git 破坏性命令时，MUST 按 Principle III 单独请求确认。
4. 提交前 MUST 在本地运行项目定义的格式化与静态分析命令；CI MUST 强制
   执行相同检查。

### 质量门禁 (Quality Gates)

以下检查 MUST 在合并到主分支之前全部通过：

- **格式化**：所有改动文件通过对应格式化工具的 `--check` 模式。
- **静态分析**：无 ERROR 级别问题；WARNING 级别问题在 PR 描述中说明。
- **注释比例**：每个新增/修改源文件按 Principle II 规定口径 `ratio >= 0.3`。
- **删除授权追踪**：所有删除操作可在 PR 描述或对话历史中追溯到用户的显式
  批准记录。
- **方案追踪**：实施型 PR MUST 关联对应的已被用户确认的方案文档（通常是
  `specs/<feature>/plan.md` 或对话中的方案确认记录）。
- **跨平台构建**：CI 在 macOS、Windows、Ubuntu 三个平台上构建均成功。

### 文件删除确认协议

当代理需要执行删除操作时 MUST 按以下结构呈现确认请求：

```
[删除确认请求]
目标列表：
  - <绝对路径 1> (大小, 最近修改时间)
  - <绝对路径 2> (...)
删除原因：<为什么需要删除>
可恢复性：<在 Git 历史中是否可找回；是否有备份>
等待用户回复"同意"/"确认"后执行。
```

用户回复 **必须显式**；沉默、模糊回复（如"好的"、"嗯"）**不得**视为授权。

### 方案呈现与确认协议

代理生成方案时 MUST 至少包含以下要素：

- **变更目标**：要达成什么效果。
- **关键步骤**：分步骤列表，按依赖顺序排列。
- **涉及文件**：将创建/修改/删除的文件路径清单。
- **潜在风险**：可能的副作用、性能影响、向后兼容性问题。
- **回退方案**：失败时如何回退到原状态。

复杂方案 MUST 持久化为 `specs/<feature>/plan.md`，简单方案可在对话中呈现。
等待用户显式批准后方可启动实施；实施过程中如需偏离方案，必须暂停并重新确认。

### 合规审查

- 所有评审者 MUST 验证 PR 是否符合本宪法的全部原则；违反任一 NON-NEGOTIABLE
  原则（Principle III、IV）的 PR MUST 被拒绝合并。
- 复杂度的引入（如新增抽象层、第三方依赖、跨模块耦合）MUST 在 PR 描述中
  说明必要性，并提供"不引入该复杂度的替代方案"的对比分析。

## Governance

### 宪法地位

本宪法是 GitView 项目的最高指导文件，**优先级高于团队内部约定、个人偏好、
AI 代理默认行为以及任何未升级到宪法的文档**。当宪法与其他文档冲突时，以本
宪法为准。任何 AI 代理（包括 Claude Code）在协作时 MUST 严格遵守本宪法的
全部原则。

### 修订程序

- 任何宪法修订 MUST 通过单独的 PR 提出，并由项目维护者审阅与批准。
- 修订 PR 的描述中 MUST 阐明：修订原因、影响范围、对依赖模板（plan、
  spec、tasks 模板）与 CI 配置的同步更新清单。
- 修订合并后 MUST 同步更新所有依赖模板与 `CLAUDE.md` 等运行时指引文档。
- 任何修订 MUST 遵循 Principle IV：先方案确认、后实施。

### 版本策略

宪法版本号遵循 SemVer (`MAJOR.MINOR.PATCH`)：

- **MAJOR**：删除或不兼容地重定义现有原则、治理条款。
- **MINOR**：新增原则或章节，或对现有指导进行实质性扩展。
- **PATCH**：文字澄清、措辞优化、错别字修复等不改变语义的修订。

### 合规审查频率

- 每次合并到主分支的 PR MUST 由评审者按本宪法核对。
- 每季度（每 3 个月）项目维护者 MUST 复盘宪法执行情况，识别需要修订的部分；
  复盘结论 SHOULD 记录在 `specs/` 下的季度审查文档中。

### 运行时指引

本宪法定义了"做什么"与"为什么"。具体的"怎么做"（开发环境搭建、命令速查、
技术细节）由 `CLAUDE.md`、`specs/<feature>/plan.md` 等运行时指引文档承载。
运行时文档 MUST 与本宪法保持一致；若发现冲突，以本宪法为准并同步更新运行时
文档。

**Version**: 1.1.0 | **Ratified**: 2026-05-24 | **Last Amended**: 2026-05-27

**修订历史**：
- 1.1.0（2026-05-27）— Principle II 阈值由 0.5 调整至 0.3，反映 schema-as-code
  类文件（types、router、stores 浅包装层）天然难以达到 0.5 的现实；同时保留
  「按文件粒度」与「行首+行尾注释」的核算口径不变（MINOR 版本：规则放宽，
  不引入新增规则）。
- 1.0.0（2026-05-24）— 初始批准 5 条原则（代码质量 / 中文注释 / 文件操作安全 /
  方案先审批 / 项目治理）。
