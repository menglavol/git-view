#!/usr/bin/env bash
# =====================================================================
# 调试输出与遗留 TODO 检查脚本（宪法 Principle I — 代码质量优先）
#
# 用途：阻止以下"开发期遗留"内容进入主分支：
#   - Rust：println!、eprintln!、dbg! 调试输出
#   - TypeScript/Vue：console.log、console.debug、console.trace、debugger
#   - 无 issue 链接的 TODO / FIXME（要求形如 TODO(#123) 或 TODO: link）
#
# 调用：bash scripts/check-no-debug-prints.sh
# 退出码：0 通过；1 发现违规
# =====================================================================

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
violations=0

# 工具函数：打印违规并累加计数
report() {
    local desc="$1"
    local matches="$2"
    if [[ -n "$matches" ]]; then
        echo "❌ $desc" >&2
        echo "$matches" >&2
        echo "" >&2
        violations=$((violations + 1))
    fi
}

# ----- 1. Rust 调试输出 -----
rust_files=()
if [[ -d "$REPO_ROOT/src-tauri/src" ]]; then
    while IFS= read -r -d '' f; do
        rust_files+=("$f")
    done < <(find "$REPO_ROOT/src-tauri/src" -type f -name '*.rs' -print0)
fi
if [[ ${#rust_files[@]} -gt 0 ]]; then
    # 排除测试模块内 println!（#[cfg(test)] 范围）— 简化为仅检查行级模式，
    # 实际项目可结合 rustfmt 或 cargo-deny 做更精细的过滤。
    rust_hits=$(grep -nE '(^|[^a-zA-Z_])(println!|eprintln!|dbg!)' "${rust_files[@]}" 2>/dev/null \
        | grep -v '#\[cfg(test)\]' \
        | grep -v '// allow-debug:' || true)
    report "Rust 文件存在调试输出（println!/eprintln!/dbg!）：" "$rust_hits"
fi

# ----- 2. TypeScript / Vue 调试输出 -----
ts_files=()
while IFS= read -r -d '' f; do
    ts_files+=("$f")
done < <(find "$REPO_ROOT/src" -type f \( -name '*.ts' -o -name '*.vue' \) -print0 2>/dev/null)

if [[ ${#ts_files[@]} -gt 0 ]]; then
    ts_hits=$(grep -nE '(console\.(log|debug|trace)|debugger)' "${ts_files[@]}" 2>/dev/null \
        | grep -v '// allow-debug:' || true)
    report "TypeScript/Vue 文件存在调试输出（console.log/debug/trace/debugger）：" "$ts_hits"
fi

# ----- 3. 无 issue 链接的 TODO / FIXME -----
all_src=()
all_src+=("${rust_files[@]}")
all_src+=("${ts_files[@]}")
if [[ ${#all_src[@]} -gt 0 ]]; then
    todo_hits=$(grep -nE '(TODO|FIXME)[^(]' "${all_src[@]}" 2>/dev/null \
        | grep -vE '(TODO|FIXME)\([#A-Za-z0-9_/-]+\)' \
        | grep -vE '(TODO|FIXME):[[:space:]]*https?://' || true)
    report "存在无 issue 链接的 TODO/FIXME（请使用 TODO(#123) 或 TODO: https://… 形式）：" "$todo_hits"
fi

echo ""
if [[ "$violations" -gt 0 ]]; then
    echo "❌ 调试输出检查失败：发现 $violations 类违规（详情见上）" >&2
    exit 1
else
    echo "✓ 调试输出检查通过：无遗留 println!/console.log/裸 TODO"
    exit 0
fi
