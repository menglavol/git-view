-- =====================================================================
-- GitView 初始化迁移 (version = 1)
-- 创建 V1 MVP 所需的全部业务表与索引。
-- 表清单（产品设计文档 §16）：
--   1. accounts                  账号元信息
--   2. gitlab_instance_configs   自建 GitLab 实例配置
--   3. remote_repositories       远程仓库缓存
--   4. local_repositories        本地仓库管理记录
--   5. clone_tasks               克隆任务持久化
--   6. operation_logs            操作日志
--   7. settings                  偏好键值对
-- 约定：
--   - 所有主键使用 UUID v4 字符串（TEXT PRIMARY KEY）
--   - 时间字段使用 ISO 8601 字符串（TEXT）
--   - 枚举字段以 snake_case 字符串存储，与 serde 序列化一致
--   - 外键已通过 PRAGMA foreign_keys = ON 在连接级别启用
-- =====================================================================

-- ---------------------------------------------------------------------
-- 1. 账号表
-- 记录平台账号元信息，token 明文不入库（仅 token_key 引用 keyring）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS accounts (
    id              TEXT PRIMARY KEY NOT NULL,
    platform        TEXT NOT NULL CHECK (platform IN ('github', 'gitlab', 'gitee')),
    web_base_url    TEXT NOT NULL,
    api_base_url    TEXT NOT NULL,
    username        TEXT NOT NULL,
    display_name    TEXT,
    avatar_url      TEXT,
    token_key       TEXT NOT NULL,
    is_default      INTEGER NOT NULL DEFAULT 0,
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    last_sync_at    TEXT
);

-- 索引：按平台筛选、唯一约束保证 (platform, web_base_url, username) 不重复
CREATE INDEX IF NOT EXISTS idx_accounts_platform ON accounts(platform);
CREATE UNIQUE INDEX IF NOT EXISTS uq_accounts_identity
    ON accounts(platform, web_base_url, username);

-- ---------------------------------------------------------------------
-- 2. GitLab 自建实例配置
-- 与 accounts 表通过 web_base_url 关联，记录实例级 TLS / API 配置
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gitlab_instance_configs (
    id                      TEXT PRIMARY KEY NOT NULL,
    web_base_url            TEXT NOT NULL UNIQUE,
    api_base_url            TEXT NOT NULL,
    allow_self_signed_cert  INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL
);

-- ---------------------------------------------------------------------
-- 3. 远程仓库缓存
-- 缓存从平台 API 拉取的仓库元数据，支撑离线浏览与搜索
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS remote_repositories (
    id              TEXT PRIMARY KEY NOT NULL,
    account_id      TEXT NOT NULL,
    platform        TEXT NOT NULL CHECK (platform IN ('github', 'gitlab', 'gitee')),
    remote_id       TEXT NOT NULL,
    full_name       TEXT NOT NULL,
    name            TEXT NOT NULL,
    owner           TEXT NOT NULL,
    description     TEXT,
    visibility      TEXT NOT NULL CHECK (visibility IN ('public', 'private', 'internal')),
    default_branch  TEXT NOT NULL,
    html_url        TEXT NOT NULL,
    ssh_url         TEXT,
    clone_url       TEXT NOT NULL,
    is_favorite     INTEGER NOT NULL DEFAULT 0,
    last_pushed_at  TEXT,
    synced_at       TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- 索引：按账号筛选 / 模糊搜索时按 full_name 命中
CREATE INDEX IF NOT EXISTS idx_remote_repos_account ON remote_repositories(account_id);
CREATE INDEX IF NOT EXISTS idx_remote_repos_full_name ON remote_repositories(full_name);
CREATE UNIQUE INDEX IF NOT EXISTS uq_remote_repos_account_remote
    ON remote_repositories(account_id, remote_id);

-- ---------------------------------------------------------------------
-- 4. 本地仓库管理记录
-- 记录用户克隆 / 添加到 GitView 管理的本地 Git 仓库
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS local_repositories (
    id                      TEXT PRIMARY KEY NOT NULL,
    remote_repository_id    TEXT,
    local_path              TEXT NOT NULL UNIQUE,
    current_branch          TEXT,
    remote_url              TEXT,
    status                  TEXT NOT NULL
        CHECK (status IN ('clean', 'dirty', 'ahead', 'behind', 'diverged', 'unknown')),
    last_checked_at         TEXT NOT NULL,
    created_at              TEXT NOT NULL,
    FOREIGN KEY (remote_repository_id) REFERENCES remote_repositories(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_local_repos_status ON local_repositories(status);

-- ---------------------------------------------------------------------
-- 5. 克隆任务表
-- 持久化克隆任务，应用重启后可恢复 / 标记 interrupted
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS clone_tasks (
    id                      TEXT PRIMARY KEY NOT NULL,
    remote_repository_id    TEXT NOT NULL,
    repository_name         TEXT NOT NULL,
    remote_url              TEXT NOT NULL,
    target_path             TEXT NOT NULL,
    status                  TEXT NOT NULL
        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    progress                INTEGER NOT NULL DEFAULT 0
        CHECK (progress >= 0 AND progress <= 100),
    error_message           TEXT,
    created_at              TEXT NOT NULL,
    started_at              TEXT,
    finished_at             TEXT,
    FOREIGN KEY (remote_repository_id) REFERENCES remote_repositories(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_clone_tasks_status ON clone_tasks(status);

-- ---------------------------------------------------------------------
-- 6. 操作日志表
-- 记录用户在 GitView 中的关键操作（已脱敏），便于故障复盘
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS operation_logs (
    id              TEXT PRIMARY KEY NOT NULL,
    operation_type  TEXT NOT NULL,
    target          TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('success', 'failed', 'cancelled')),
    error_message   TEXT,
    duration_ms     INTEGER NOT NULL DEFAULT 0,
    occurred_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_op_logs_occurred_at ON operation_logs(occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_op_logs_type ON operation_logs(operation_type);

-- ---------------------------------------------------------------------
-- 7. 设置表（key/value）
-- 应用偏好以键值对形式持久化，新增设置无需迁移
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY NOT NULL,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
