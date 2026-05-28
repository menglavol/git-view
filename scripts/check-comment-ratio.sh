#!/usr/bin/env bash
# =====================================================================
# 中文注释比例验证脚本（宪法 Principle II）
#
# 用途：扫描源文件（Rust / TypeScript / Vue），核算每个文件的
#       「含中文字符的注释行数 / 非空代码行数」比例，阈值 0.3（宪法 1.1.0）。
#       注释行同时计入「行首独立注释」与「行尾注释」（宪法定义）。
#
# 调用：
#   bash scripts/check-comment-ratio.sh                # 默认 --changed-only
#   bash scripts/check-comment-ratio.sh --all          # 扫描全量
#   bash scripts/check-comment-ratio.sh PATH [PATH...] # 扫描指定路径
#   bash scripts/check-comment-ratio.sh --changed-only # 仅 git diff 变更文件
#
# --changed-only 行为：
#   - PR 场景：用 origin/${GITHUB_BASE_REF:-main}...HEAD 取差异文件清单
#   - 本地场景：用 HEAD 当前未提交改动 + HEAD vs HEAD~1（兼容刚 commit 完）
#   - 无 git 或差异为空：直接通过（exit 0），便于 main 分支 push 触发
#
# 退出码：
#   0 — 所有目标文件达标，或 --changed-only 模式下无目标
#   1 — 至少一个文件未达标（CI 会以此阻断合并）
#
# 豁免清单：从 .specify/comment-exemptions.yml 读取 exempt_paths 列表。
# 注释行定义：含中文字符（\\u4E00-\\u9FFF）的行——任意位置即可（行首或行尾）。
# 非空代码行：去除空行 + 纯注释行后剩余的行数。
# =====================================================================

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
THRESHOLD="${COMMENT_RATIO_THRESHOLD:-0.3}"
EXEMPTIONS_FILE="$REPO_ROOT/.specify/comment-exemptions.yml"

# 读取豁免清单（简单 YAML 解析：仅支持 exempt_paths 段下 `- xxx` 形式）
declare -a EXEMPT_PATTERNS=()
if [[ -f "$EXEMPTIONS_FILE" ]]; then
    in_section=false
    while IFS= read -r line; do
        # 去除前导空白与回车
        trimmed="${line#"${line%%[![:space:]]*}"}"
        if [[ "$trimmed" == "exempt_paths:"* ]]; then
            in_section=true
            continue
        fi
        # 检测下一个 YAML 段落开始（顶层 key，且不是数组项）
        if $in_section && [[ "$trimmed" =~ ^[a-zA-Z_]+: ]]; then
            in_section=false
        fi
        if $in_section && [[ "$trimmed" =~ ^-[[:space:]]+ ]]; then
            pattern="${trimmed#- }"
            pattern="${pattern// /}"  # 去除多余空格
            EXEMPT_PATTERNS+=("$pattern")
        fi
    done <"$EXEMPTIONS_FILE"
fi

# 判断路径是否在豁免清单内（支持简单 glob，*. 与目录前缀匹配）
is_exempt() {
    local path="$1"
    local rel="${path#$REPO_ROOT/}"
    for pat in "${EXEMPT_PATTERNS[@]}"; do
        # shellcheck disable=SC2053
        if [[ "$rel" == $pat ]]; then
            return 0
        fi
    done
    return 1
}

# 计算单文件比例
check_file() {
    local file="$1"
    # 含中文的注释行：宪法定义同时计入行首独立注释与行尾注释。
    # 实现：先把行内位于注释片段（//.../*.../<!--...-->）中的中文识别出来。
    # 简化策略：任意行包含「注释开始符且其右侧含中文」即记为一行注释。
    #   - //+ 后含中文（TS/Rust/Vue script）
    #   - # + 后含中文（shell——shell 脚本本身不在扫描范围；保留兼容）
    #   - /* ... */ 块（单行或多行）内含中文
    #   - <!-- ... --> 块内含中文
    # 块注释跨多行时本脚本按「单行匹配」近似计数（一行内含 /* 或 *）。
    local comment_lines
    comment_lines=$(grep -cE '(//|/\*|\*|<!--|#)[^\n]*[一-龥]' "$file" 2>/dev/null) || true
    # 非空代码行 = 总行数 - 空行数 - 纯注释行（行内仅含注释开头）
    local total
    total=$(wc -l <"$file" | tr -d ' ')
    # 注意：grep -c 在 0 匹配时打印 "0" 并 exit 1。
    # 用 `|| true` 仅消化非零退出码；若改写为 `|| echo 0` 会在 0 匹配场景
    # 双输出 "0\n0"，导致后续 $(()) 算术上下文 syntax error，
    # code_lines 落空被错判为「全是注释」→ 文件被误判通过（历史 bug）。
    local empty
    empty=$(grep -cE '^[[:space:]]*$' "$file" 2>/dev/null) || true
    local pure_comment
    pure_comment=$(grep -cE '^[[:space:]]*(//|#|/\*|\*|<!--|-->)' "$file" 2>/dev/null) || true
    local code_lines=$((total - empty - pure_comment))
    if [[ "$code_lines" -le 0 ]]; then
        # 文件全是注释或全是空行，跳过
        return 0
    fi
    # 比例 = 注释行数 / 代码行数
    local ratio
    ratio=$(awk "BEGIN { printf \"%.2f\", $comment_lines / $code_lines }")
    local pass
    pass=$(awk "BEGIN { print ($ratio >= $THRESHOLD) ? 1 : 0 }")
    if [[ "$pass" -ne 1 ]]; then
        printf "  ✗ %s — 中文注释 %d / 代码 %d = %.2f (< %s)\n" \
            "${file#$REPO_ROOT/}" "$comment_lines" "$code_lines" "$ratio" "$THRESHOLD" >&2
        return 1
    fi
    return 0
}

# 主逻辑：解析参数与收集待检查文件
MODE="changed-only"   # 默认增量；老代码视为豁免技术债（用户决策）
EXPLICIT_TARGETS=()

for arg in "$@"; do
    case "$arg" in
        --all)
            MODE="all"
            ;;
        --changed-only)
            MODE="changed-only"
            ;;
        -h | --help)
            sed -n '1,30p' "$0"
            exit 0
            ;;
        *)
            # 显式传入的路径优先级最高
            EXPLICIT_TARGETS+=("$arg")
            MODE="explicit"
            ;;
    esac
done

# 收集待扫描的文件清单（统一塞进 FILES 数组）
declare -a FILES=()

case "$MODE" in
    explicit)
        for t in "${EXPLICIT_TARGETS[@]}"; do
            if [[ -d "$t" ]]; then
                while IFS= read -r -d '' f; do
                    FILES+=("$f")
                done < <(find "$t" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.vue' \) -print0)
            elif [[ -f "$t" ]]; then
                FILES+=("$t")
            fi
        done
        ;;
    all)
        # 全量：扫描 src/ 与 src-tauri/src/
        for t in "$REPO_ROOT/src" "$REPO_ROOT/src-tauri/src"; do
            [[ -d "$t" ]] || continue
            while IFS= read -r -d '' f; do
                FILES+=("$f")
            done < <(find "$t" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.vue' \) -print0)
        done
        ;;
    changed-only)
        # 增量：用 git diff 取新增/修改文件
        # 优先级：
        #   1) GITHUB_BASE_REF 存在（PR 场景）→ origin/$GITHUB_BASE_REF...HEAD
        #   2) 本地未提交改动（git status --porcelain）
        #   3) HEAD vs HEAD~1（本地刚 commit 完）
        # 若收集为空 → 直接通过（exit 0）
        if ! command -v git >/dev/null 2>&1; then
            echo "→ git 不存在，--changed-only 模式直接通过"
            exit 0
        fi
        cd "$REPO_ROOT" || exit 0

        declare -a CHANGED=()
        if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
            # PR 模式
            git fetch --depth=1 origin "$GITHUB_BASE_REF" >/dev/null 2>&1 || true
            while IFS= read -r line; do
                [[ -n "$line" ]] && CHANGED+=("$line")
            done < <(git diff --name-only "origin/${GITHUB_BASE_REF}...HEAD" -- '*.rs' '*.ts' '*.vue' 2>/dev/null || true)
        fi
        if [[ ${#CHANGED[@]} -eq 0 ]]; then
            # 本地未提交改动（用 -uall 展开未跟踪目录内的文件，否则只看到目录名）
            while IFS= read -r line; do
                [[ -n "$line" ]] && CHANGED+=("$line")
            done < <(git status --porcelain -uall 2>/dev/null \
                | awk '{print $NF}' \
                | grep -E '\.(rs|ts|vue)$' || true)
        fi
        if [[ ${#CHANGED[@]} -eq 0 ]]; then
            # HEAD vs HEAD~1（兼容刚 commit 完场景）
            while IFS= read -r line; do
                [[ -n "$line" ]] && CHANGED+=("$line")
            done < <(git diff --name-only HEAD~1 HEAD -- '*.rs' '*.ts' '*.vue' 2>/dev/null || true)
        fi

        if [[ ${#CHANGED[@]} -eq 0 ]]; then
            echo "✓ --changed-only 模式：无 .rs/.ts/.vue 变更文件，跳过检查"
            exit 0
        fi

        for rel in "${CHANGED[@]}"; do
            abs="$REPO_ROOT/$rel"
            [[ -f "$abs" ]] && FILES+=("$abs")
        done
        ;;
esac

failed=0
checked=0
for file in "${FILES[@]}"; do
    if is_exempt "$file"; then
        continue
    fi
    case "$file" in
        *.rs | *.ts | *.vue)
            checked=$((checked + 1))
            if ! check_file "$file"; then
                failed=$((failed + 1))
            fi
            ;;
    esac
done

echo ""
if [[ "$failed" -gt 0 ]]; then
    echo "❌ 中文注释比例检查失败：$failed / $checked 个文件未达标 (阈值 $THRESHOLD)" >&2
    echo "请按宪法 Principle II 补充中文注释，或将自动生成文件加入 .specify/comment-exemptions.yml" >&2
    exit 1
else
    echo "✓ 中文注释比例检查通过：$checked 个文件均达标 (阈值 $THRESHOLD)"
    exit 0
fi
