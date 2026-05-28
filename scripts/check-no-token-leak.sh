#!/usr/bin/env bash
# =====================================================================
# Token 明文泄漏扫描脚本（宪法 Principle III — 文件操作安全 / 安全要求）
#
# 用途：扫描以下位置查找潜在的 Token 明文：
#   - 数据库文件（SQLite）
#   - 应用日志目录
#   - 仓库内代码与配置文件
#
# 匹配模式：
#   - GitHub PAT：ghp_*, gho_*, ghu_*, ghs_*, ghr_*
#   - GitLab PAT：glpat-*
#   - 带凭据 URL：https://<token>@host
#   - Bearer 头：Authorization: Bearer <token>
#
# 调用：bash scripts/check-no-token-leak.sh
# 退出码：0 干净；1 发现疑似明文泄漏
# =====================================================================

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
violations=0

# 检测命令是否存在
have() { command -v "$1" >/dev/null 2>&1; }

# 跨平台获取 GitView 应用数据目录与日志目录
detect_app_dirs() {
    case "$(uname -s)" in
        Darwin)
            APP_DATA="$HOME/Library/Application Support/com.gitview.app"
            APP_LOGS="$HOME/Library/Logs/com.gitview.app"
            ;;
        Linux)
            APP_DATA="${XDG_DATA_HOME:-$HOME/.local/share}/com.gitview.app"
            APP_LOGS="${XDG_STATE_HOME:-$HOME/.local/state}/com.gitview.app/logs"
            ;;
        MINGW* | MSYS* | CYGWIN*)
            APP_DATA="$LOCALAPPDATA/com.gitview.app"
            APP_LOGS="$LOCALAPPDATA/com.gitview.app/logs"
            ;;
        *)
            APP_DATA=""
            APP_LOGS=""
            ;;
    esac
}
detect_app_dirs

# 正则模式
TOKEN_PATTERNS=(
    'ghp_[A-Za-z0-9]{36,}'
    'gho_[A-Za-z0-9]{36,}'
    'ghu_[A-Za-z0-9]{36,}'
    'ghs_[A-Za-z0-9]{36,}'
    'ghr_[A-Za-z0-9]{36,}'
    'glpat-[A-Za-z0-9_-]{20,}'
    'https?://[A-Za-z0-9._~%-]+:[A-Za-z0-9._~%-]+@'
    'https?://[A-Za-z0-9_-]{20,}@'
    'Authorization:[[:space:]]*Bearer[[:space:]]+[A-Za-z0-9._-]+'
)

# 扫描单个目标（文件或目录）
scan_target() {
    local target="$1"
    local desc="$2"
    [[ -e "$target" ]] || return 0
    local combined_pattern
    combined_pattern=$(IFS='|'; echo "${TOKEN_PATTERNS[*]}")
    local hits
    if [[ -d "$target" ]]; then
        hits=$(grep -rIEn "$combined_pattern" "$target" 2>/dev/null \
            | grep -v 'allow-token-pattern' || true)
    else
        # 对二进制 SQLite 文件使用 strings 抽取后再扫描
        if file "$target" 2>/dev/null | grep -qE 'SQLite|data'; then
            hits=$(strings "$target" 2>/dev/null | grep -En "$combined_pattern" || true)
        else
            hits=$(grep -IEn "$combined_pattern" "$target" 2>/dev/null \
                | grep -v 'allow-token-pattern' || true)
        fi
    fi
    if [[ -n "$hits" ]]; then
        echo "❌ 在 $desc 发现疑似 Token 明文：" >&2
        echo "目标：$target" >&2
        echo "$hits" | head -20 >&2
        echo "" >&2
        violations=$((violations + 1))
    fi
}

# 1. 扫描代码仓库（排除 node_modules / target / specs 等无关目录）
echo "→ 扫描仓库代码..."
repo_hits=$(
    grep -rIEn "$(IFS='|'; echo "${TOKEN_PATTERNS[*]}")" "$REPO_ROOT" \
        --exclude-dir=node_modules \
        --exclude-dir=target \
        --exclude-dir=src-tauri/target \
        --exclude-dir=.git \
        --exclude-dir=dist \
        --exclude-dir=build \
        --exclude-dir=specs \
        --exclude-dir=.specify \
        --exclude-dir=design-diary \
        --exclude='*.lock' \
        2>/dev/null \
    | grep -v 'allow-token-pattern' \
    | grep -v 'scripts/check-no-token-leak.sh' \
    | grep -v 'src-tauri/src/utils/redact.rs' \
    || true
)
# 注释：`src-tauri/src/utils/redact.rs` 的存在目的就是定义与测试 token 检测模式，
# 文件正文与 #[cfg(test)] 块都必然含示例 token 字符串。整文件豁免，避免在每个
# 示例后挂 // allow-token-pattern 噪音；该文件本身的 redact_token 函数有 7 个单测
# 覆盖各种 token 模式，安全性由单测保障。
if [[ -n "$repo_hits" ]]; then
    echo "❌ 仓库代码中发现疑似 Token 明文：" >&2
    echo "$repo_hits" | head -20 >&2
    violations=$((violations + 1))
fi

# 2. 扫描应用数据目录中的 SQLite
if [[ -n "$APP_DATA" && -f "$APP_DATA/gitview.db" ]]; then
    echo "→ 扫描 SQLite 数据库：$APP_DATA/gitview.db"
    scan_target "$APP_DATA/gitview.db" "SQLite 数据库"
fi

# 3. 扫描日志目录
if [[ -n "$APP_LOGS" && -d "$APP_LOGS" ]]; then
    echo "→ 扫描日志目录：$APP_LOGS"
    scan_target "$APP_LOGS" "应用日志"
fi

echo ""
if [[ "$violations" -gt 0 ]]; then
    echo "❌ Token 泄漏检查失败：发现 $violations 处疑似明文（详情见上）" >&2
    echo "对应宪法 Principle III 与 spec SC-009/SC-010。" >&2
    exit 1
else
    echo "✓ Token 泄漏检查通过：未发现明文凭据"
    exit 0
fi
