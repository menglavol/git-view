# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]

**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

[Extract from feature spec: primary requirement + technical approach from research]

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: [e.g., Python 3.11, Swift 5.9, Rust 1.75 or NEEDS CLARIFICATION]

**Primary Dependencies**: [e.g., FastAPI, UIKit, LLVM or NEEDS CLARIFICATION]

**Storage**: [if applicable, e.g., PostgreSQL, CoreData, files or N/A]

**Testing**: [e.g., pytest, XCTest, cargo test or NEEDS CLARIFICATION]

**Target Platform**: [e.g., Linux server, iOS 15+, WASM or NEEDS CLARIFICATION]

**Project Type**: [e.g., library/cli/web-service/mobile-app/compiler/desktop-app or NEEDS CLARIFICATION]

**Performance Goals**: [domain-specific, e.g., 1000 req/s, 10k lines/sec, 60 fps or NEEDS CLARIFICATION]

**Constraints**: [domain-specific, e.g., <200ms p95, <100MB memory, offline-capable or NEEDS CLARIFICATION]

**Scale/Scope**: [domain-specific, e.g., 10k users, 1M LOC, 50 screens or NEEDS CLARIFICATION]

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

依据 GitView Constitution v1.0.0 的 4 条核心原则进行检查。任何标记为
**NON-NEGOTIABLE** 的门禁未通过，方案 MUST 在进入实施前修正。

### Gate I — 代码质量优先 (Code Quality First)

- [ ] 方案明确声明本特性将使用的格式化工具（如 `cargo fmt`、`prettier`）。
- [ ] 方案明确声明本特性将使用的静态分析工具（如 `cargo clippy`、`eslint`）。
- [ ] 方案中规划的函数/文件预期长度未超过宪法阈值（函数 ≤ 50 行、文件 ≤ 500 行），
      或对超阈值情况提供了拆分计划与理由。
- [ ] 方案中无遗留的临时调试输出、注释掉的废弃代码或无 issue 关联的 TODO。

### Gate II — 中文注释规范 (Chinese Documentation Standard)

- [ ] 方案承诺所有新增/修改源文件的中文注释比例 ≥ 50%
      （按 `中文注释行数 / 非空代码行数` 单文件粒度核算）。
- [ ] 方案描述了文件级注释、公共 API 文档注释、复杂逻辑解释（WHY 而非 WHAT）
      的覆盖策略。
- [ ] 涉及自动生成代码的部分已在方案中列出豁免清单与豁免原因。

### Gate III — 文件操作安全 (File Operation Safety) — **NON-NEGOTIABLE**

- [ ] 方案识别了所有可能的文件/目录删除点（含 OS 级 `rm`、Git 破坏性命令
      `git clean`/`reset --hard`/`branch -D` 等）。
- [ ] 每个删除点均规划了"用户显式确认"流程，且不依赖隐式批准。
- [ ] 方案明确声明读取与写入操作可直接执行，无需逐次确认。

### Gate IV — 方案确认优先 (Plan-First Approval) — **NON-NEGOTIABLE**

- [ ] 本方案本身已（或即将）呈现给用户并获得显式批准；批准记录可追溯。
- [ ] 方案包含变更目标、关键步骤、涉及文件清单、潜在风险、回退方案五要素。
- [ ] 方案声明了在实施过程中如需偏离原方案的处理协议（暂停 + 二次确认）。

### Gate 综合检查

- [ ] CI 已配置或将配置以强制执行 Gate I/II 的自动化检查。
- [ ] 方案已关联到本特性的 `specs/<feature>/plan.md`（或在对话中显式记录）。
- [ ] 跨平台（macOS / Windows / Ubuntu）兼容性已考虑（若涉及平台相关代码）。

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
# [REMOVE IF UNUSED] Option 1: Single project (DEFAULT)
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# [REMOVE IF UNUSED] Option 2: Web application (when "frontend" + "backend" detected)
backend/
├── src/
│   ├── models/
│   ├── services/
│   └── api/
└── tests/

frontend/
├── src/
│   ├── components/
│   ├── pages/
│   └── services/
└── tests/

# [REMOVE IF UNUSED] Option 3: Mobile + API (when "iOS/Android" detected)
api/
└── [same as backend above]

ios/ or android/
└── [platform-specific structure: feature modules, UI flows, platform tests]
```

**Structure Decision**: [Document the selected structure and reference the real
directories captured above]

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
