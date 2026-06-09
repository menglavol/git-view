-- =====================================================================
-- GitView 迁移 v4：账户默认 Clone 协议（提升为通用账户字段）
--
-- 变更点：
--   1. accounts 表新增 default_clone_protocol 列（所有平台通用）。
--      此前协议偏好仅存在于 gitlab_instance_configs（GitLab 专属），
--      现提升为账户级字段，供 clone 编排按仓库所属账户选择 SSH/HTTPS。
--   2. 回填：把已有自建 GitLab 账户在实例配置里选过的协议复制到 accounts，
--      避免提升字段后丢失这些账户原有的协议选择。
--
-- 默认值策略：列默认 'https'，存量账户保持原 HTTPS clone 行为；
-- 新建账户的默认 SSH 由前端表单初始值负责，不在 DB 层强制。
-- 整个流程在事务中执行（migrations.rs::apply_migration 已包裹 BEGIN/COMMIT）。
-- =====================================================================

-- 1. 新增列（SQLite 支持 ALTER ADD COLUMN，仅加列无需重建表）
ALTER TABLE accounts ADD COLUMN default_clone_protocol TEXT NOT NULL DEFAULT 'https'
    CHECK (default_clone_protocol IN ('https', 'ssh'));

-- 2. 回填自建 GitLab 账户已选协议（存在实例配置则覆盖账户级默认）
UPDATE accounts
SET default_clone_protocol = (
    SELECT g.default_clone_protocol
    FROM gitlab_instance_configs g
    WHERE g.account_id = accounts.id
)
WHERE id IN (SELECT account_id FROM gitlab_instance_configs);
