-- =====================================================================
-- GitView 迁移 v2：扩展账号与 GitLab 实例配置
--
-- 变更点：
--   1. accounts 表增加 remark 字段（用户备注）
--   2. gitlab_instance_configs 表重构，增加 10 个实例级配置字段：
--      account_id（外键）、allow_insecure_http、allow_invalid_certs、
--      use_system_proxy、proxy_url、request_timeout_seconds、
--      default_clone_protocol、ssh_host_alias、api_path_prefix、
--      last_connection_status、last_connection_error
--   3. 同步更新唯一约束：从 web_base_url 唯一改为 account_id 唯一
--      （一个账号最多对应一个实例配置）
--
-- 重构策略：SQLite 不支持便捷的 ALTER TABLE DROP COLUMN，
-- 故采用"创建新表 → 拷贝数据 → 删除旧表 → 改名"标准流程；
-- 整个流程在事务中执行（migrations.rs::apply_migration 已包裹 BEGIN/COMMIT）。
-- =====================================================================

-- ---------------------------------------------------------------------
-- 1. accounts 表追加 remark 字段
-- ---------------------------------------------------------------------
ALTER TABLE accounts ADD COLUMN remark TEXT;

-- ---------------------------------------------------------------------
-- 2. gitlab_instance_configs 重构
-- ---------------------------------------------------------------------

-- 2.1 创建新表（带完整字段集合）
CREATE TABLE gitlab_instance_configs_new (
    id                          TEXT PRIMARY KEY NOT NULL,
    account_id                  TEXT NOT NULL UNIQUE,
    web_base_url                TEXT NOT NULL,
    api_base_url                TEXT NOT NULL,
    allow_insecure_http         INTEGER NOT NULL DEFAULT 0,
    allow_invalid_certs         INTEGER NOT NULL DEFAULT 0,
    use_system_proxy            INTEGER NOT NULL DEFAULT 1,
    proxy_url                   TEXT,
    request_timeout_seconds     INTEGER,
    default_clone_protocol      TEXT NOT NULL DEFAULT 'https'
        CHECK (default_clone_protocol IN ('https', 'ssh')),
    ssh_host_alias              TEXT,
    api_path_prefix             TEXT,
    last_connection_status      TEXT NOT NULL DEFAULT 'unknown'
        CHECK (last_connection_status IN ('unknown', 'success', 'failed')),
    last_connection_error       TEXT,
    created_at                  TEXT NOT NULL,
    updated_at                  TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- 2.2 旧表数据迁移（旧表无 account_id 字段，因此能迁移的字段有限；
-- V1 alpha 阶段尚未发布，旧数据通常为空，但仍保留 INSERT 以保证幂等）
-- 备注：旧 allow_self_signed_cert 字段语义等同新的 allow_invalid_certs，
-- 此处做语义映射；account_id 在旧表缺失，故用 web_base_url 作为占位
-- （实际旧表为空时此 INSERT 为 no-op）。
INSERT INTO gitlab_instance_configs_new (
    id, account_id, web_base_url, api_base_url,
    allow_invalid_certs, created_at, updated_at
)
SELECT
    id,
    web_base_url, -- 占位：旧表无 account_id，使用 web_base_url 防止违反 NOT NULL
    web_base_url,
    api_base_url,
    allow_self_signed_cert,
    created_at,
    updated_at
FROM gitlab_instance_configs;

-- 2.3 删除旧表并改名
DROP TABLE gitlab_instance_configs;
ALTER TABLE gitlab_instance_configs_new RENAME TO gitlab_instance_configs;

-- 2.4 重建索引
CREATE INDEX IF NOT EXISTS idx_gitlab_configs_account
    ON gitlab_instance_configs(account_id);
