-- =====================================================================
-- Migration 003：扩展 operation_logs 表（US6 操作日志与诊断）
-- 目的：为操作日志增加「执行命令」与「命令输出」两列，支撑诊断详情展示。
-- 约定：
--   - 两列均可空（TEXT），兼容 001 已建表后的历史日志行（旧行为 NULL）
--   - 写入前由 log_service 统一执行 redact_token 脱敏，库内不留 token 明文
-- =====================================================================

-- 执行的命令（已脱敏，如 "git push origin main"）
ALTER TABLE operation_logs ADD COLUMN command TEXT;

-- 命令输出摘要（已脱敏，成功时为 stdout 摘要）
ALTER TABLE operation_logs ADD COLUMN output TEXT;
