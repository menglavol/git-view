#!/usr/bin/env bash
# =====================================================================
# 凭据残留检查脚本（宪法 Principle III / spec SC-011）
#
# 目标：验证「数据库中的账号」与「系统密钥库中 gitview 服务下的凭据」一致：
#   - 残留（违规）：密钥库中存在某 account 的凭据，但数据库已无该账号
#                   —— 说明删除账号时凭据未被清除，违反 SC-011，退出码 1。
#   - 缺失（仅告警）：数据库有账号但密钥库无对应凭据 —— 提示用户「重新验证」，不阻断。
#
# 跨平台说明：密钥库枚举依赖各平台工具，尽力而为：
#   - macOS：security dump-keychain
#   - Linux：secret-tool（libsecret-tools）
#   - 其他平台 / 工具缺失 / 无数据库：打印提示并以 0 退出（CI、全新环境视为通过）。
#
# 调用：bash scripts/check-credential-cleanup.sh
# =====================================================================

set -uo pipefail

# keyring crate 使用的服务名（见 credential_service::SERVICE_NAME）
SERVICE_NAME="gitview"

# 命令存在性探测
have() { command -v "$1" >/dev/null 2>&1; }

# 跨平台定位 gitview 数据库（与 check-no-token-leak.sh 保持一致）
detect_db() {
    case "$(uname -s)" in
        Darwin) echo "$HOME/Library/Application Support/com.gitview.app/gitview.db" ;;
        Linux) echo "${XDG_DATA_HOME:-$HOME/.local/share}/com.gitview.app/gitview.db" ;;
        MINGW* | MSYS* | CYGWIN*) echo "$LOCALAPPDATA/com.gitview.app/gitview.db" ;;
        *) echo "" ;;
    esac
}

DB_PATH="$(detect_db)"

# 无数据库（全新环境 / CI）或无 sqlite3：无可比对内容，直接通过
if [[ -z "$DB_PATH" || ! -f "$DB_PATH" ]]; then
    echo "✓ 未找到 gitview 数据库（全新环境 / CI），凭据残留检查跳过"
    exit 0
fi
if ! have sqlite3; then
    echo "⚠ 未安装 sqlite3，无法读取账号清单，凭据残留检查跳过"
    exit 0
fi

# 读取数据库中的账号 id 集合
db_accounts="$(sqlite3 "$DB_PATH" "SELECT id FROM accounts;" 2>/dev/null || true)"

# 枚举密钥库中 gitview 服务下的 account（按平台），输出每行一个 account id
keyring_accounts() {
    case "$(uname -s)" in
        Darwin)
            have security || return 0
            # dump-keychain 仅列出条目属性（不含密码明文）；提取 svce=gitview 条目的 acct。
            # 注：依赖 acct 属性在同条目块内先于 svce 出现，属尽力解析。
            security dump-keychain 2>/dev/null | awk '
                /"acct"<blob>=/ { a=$0; sub(/.*"acct"<blob>="/,"",a); sub(/".*/,"",a) }
                /"svce"<blob>="'"$SERVICE_NAME"'"/ { if (a!="") { print a; a="" } }
            '
            ;;
        Linux)
            have secret-tool || return 0
            # libsecret 按 service 属性检索，输出含 attribute.account = <id> 行
            secret-tool search --all service "$SERVICE_NAME" 2>/dev/null \
                | awk -F'= ' '/account/ {print $2}'
            ;;
        *) return 0 ;;
    esac
}

# 工具不可用时跳过反向检查（避免误判为「无残留」）
case "$(uname -s)" in
    Darwin) have security || { echo "⚠ security 不可用，残留检查跳过"; exit 0; } ;;
    Linux) have secret-tool || { echo "⚠ secret-tool 不可用（需 libsecret-tools），残留检查跳过"; exit 0; } ;;
    *) echo "⚠ 当前平台暂不支持自动枚举密钥库，残留检查跳过"; exit 0 ;;
esac

kr_accounts="$(keyring_accounts || true)"

violations=0

# 残留检查：密钥库有、数据库无 → 违规（SC-011）
while IFS= read -r acct; do
    [[ -z "$acct" ]] && continue
    if ! grep -qxF "$acct" <<<"$db_accounts"; then
        echo "❌ 残留凭据：account「$acct」存在于密钥库，但数据库已无该账号" >&2
        violations=$((violations + 1))
    fi
done <<<"$kr_accounts"

# 缺失检查：数据库有、密钥库无 → 仅告警（提示重新验证，不阻断）
while IFS= read -r acct; do
    [[ -z "$acct" ]] && continue
    if ! grep -qxF "$acct" <<<"$kr_accounts"; then
        echo "⚠ 凭据缺失：account「$acct」在数据库中存在，但密钥库无对应凭据（建议重新验证）"
    fi
done <<<"$db_accounts"

echo ""
if [[ "$violations" -gt 0 ]]; then
    echo "❌ 凭据残留检查失败：发现 $violations 处残留（违反 SC-011）" >&2
    exit 1
fi
echo "✓ 凭据残留检查通过：密钥库中无与数据库脱节的残留凭据"
exit 0
