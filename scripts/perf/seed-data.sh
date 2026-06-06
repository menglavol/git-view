#!/usr/bin/env bash
# =====================================================================
# 性能基线种子数据脚本（spec SC-005 / SC-006 / SC-007）
#
# 用途：向 gitview 数据库批量写入测试数据，用于验证大数据量下的展示与刷新性能：
#   - 本地仓库   500 条（SC-005）
#   - 远程仓库  5000 条（SC-006）
#   - 操作日志 10000 条（SC-007）
# 使用 SQLite 递归 CTE 一次性批量生成，远快于 shell 逐条插入。
#
# 前提：数据库已存在（即应用至少正常启动过一次以建表）。
# 路径：默认取各平台标准位置，可用环境变量 GITVIEW_DB 覆盖（便于测试 / 自定义）。
#
# 调用：
#   bash scripts/perf/seed-data.sh           # 默认规模 500/5000/10000
#   bash scripts/perf/seed-data.sh --small   # 小规模 50/100/200，快速验证脚本
#   bash scripts/perf/seed-data.sh --clean   # 清除本脚本写入的全部 seed-% 数据
# =====================================================================

set -uo pipefail

have() { command -v "$1" >/dev/null 2>&1; }

# 定位数据库：优先环境变量，其次各平台标准位置
detect_db() {
    if [[ -n "${GITVIEW_DB:-}" ]]; then
        echo "$GITVIEW_DB"
        return
    fi
    case "$(uname -s)" in
        Darwin) echo "$HOME/Library/Application Support/com.gitview.app/gitview.db" ;;
        Linux) echo "${XDG_DATA_HOME:-$HOME/.local/share}/com.gitview.app/gitview.db" ;;
        MINGW* | MSYS* | CYGWIN*) echo "$LOCALAPPDATA/com.gitview.app/gitview.db" ;;
        *) echo "" ;;
    esac
}

# 规模参数（默认对齐 SC 指标）
N_LOCAL=500
N_REMOTE=5000
N_LOGS=10000
MODE="seed"

for arg in "$@"; do
    case "$arg" in
        --small) N_LOCAL=50; N_REMOTE=100; N_LOGS=200 ;;
        --clean) MODE="clean" ;;
        -h | --help) sed -n '1,30p' "$0"; exit 0 ;;
        *) echo "未知参数：$arg" >&2; exit 2 ;;
    esac
done

DB_PATH="$(detect_db)"

if ! have sqlite3; then
    echo "❌ 未安装 sqlite3，无法写入种子数据" >&2
    exit 1
fi
if [[ -z "$DB_PATH" || ! -f "$DB_PATH" ]]; then
    echo "❌ 未找到数据库：${DB_PATH:-（未知平台）}" >&2
    echo "   请先正常启动一次 GitView 应用以创建数据库，或用 GITVIEW_DB 指定路径。" >&2
    exit 1
fi

# 清理模式：仅删除本脚本写入的 seed-% 数据，绝不动用户真实数据
if [[ "$MODE" == "clean" ]]; then
    sqlite3 "$DB_PATH" <<'SQL'
DELETE FROM operation_logs    WHERE id LIKE 'seed-log-%';
DELETE FROM local_repositories WHERE id LIKE 'seed-local-%';
DELETE FROM remote_repositories WHERE id LIKE 'seed-remote-%';
DELETE FROM accounts          WHERE id = 'seed-account';
SQL
    echo "✓ 已清除全部 seed-% 种子数据"
    exit 0
fi

echo "→ 向 $DB_PATH 写入种子数据（local=${N_LOCAL} remote=${N_REMOTE} logs=${N_LOGS}）..."

# 用递归 CTE 批量生成；先建一个 seed 账号供远程仓库外键关联
sqlite3 "$DB_PATH" <<SQL
BEGIN;

-- seed 账号（token_key 指向不存在的凭据，仅用于满足列表/外键，不参与真实请求）
INSERT OR IGNORE INTO accounts
    (id, platform, web_base_url, api_base_url, username, token_key, is_default, enabled, created_at, updated_at)
VALUES
    ('seed-account', 'github', 'https://github.com', 'https://api.github.com',
     'seed-user', 'seed-account', 0, 1, datetime('now'), datetime('now'));

-- 远程仓库：每 10 个收藏 1 个，便于测收藏筛选
INSERT OR IGNORE INTO remote_repositories
    (id, account_id, platform, remote_id, full_name, name, owner, description,
     visibility, default_branch, html_url, clone_url, is_favorite, synced_at)
WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < ${N_REMOTE})
SELECT 'seed-remote-' || n, 'seed-account', 'github', n,
       'seed-user/repo-' || n, 'repo-' || n, 'seed-user', '性能基线种子仓库 ' || n,
       'public', 'main',
       'https://github.com/seed-user/repo-' || n,
       'https://github.com/seed-user/repo-' || n || '.git',
       (n % 10 = 0), datetime('now')
FROM seq;

-- 本地仓库：四种状态轮转，覆盖状态筛选
INSERT OR IGNORE INTO local_repositories
    (id, remote_repository_id, local_path, current_branch, remote_url, status, last_checked_at, created_at)
WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < ${N_LOCAL})
SELECT 'seed-local-' || n, NULL, '/tmp/gitview-seed/repo-' || n, 'main',
       'https://github.com/seed-user/repo-' || n || '.git',
       (CASE n % 4 WHEN 0 THEN 'clean' WHEN 1 THEN 'dirty' WHEN 2 THEN 'ahead' ELSE 'behind' END),
       datetime('now'), datetime('now')
FROM seq;

-- 操作日志：时间向过去递减分布，便于测时间范围筛选与分页
INSERT OR IGNORE INTO operation_logs
    (id, operation_type, target, status, error_message, duration_ms, occurred_at)
WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < ${N_LOGS})
SELECT 'seed-log-' || n,
       (CASE n % 4 WHEN 0 THEN 'pull' WHEN 1 THEN 'push' WHEN 2 THEN 'fetch' ELSE 'commit' END),
       'seed-user/repo-' || (n % ${N_REMOTE}),
       (CASE n % 5 WHEN 0 THEN 'failed' ELSE 'success' END),
       NULL, (n % 2000), datetime('now', '-' || n || ' minutes')
FROM seq;

COMMIT;
SQL

echo "✓ 种子数据写入完成。统计："
sqlite3 "$DB_PATH" "SELECT '  remote_repositories=' || count(*) FROM remote_repositories;
SELECT '  local_repositories =' || count(*) FROM local_repositories;
SELECT '  operation_logs     =' || count(*) FROM operation_logs;"
