//! Git 远程 URL 协议互转工具。
//!
//! 用于本地仓库的「切换协议」功能：在 HTTPS 与 SSH 两种远程地址形式之间转换。
//! 当本地仓库未关联平台远程仓库（无现成 ssh_url/clone_url）时，由这里基于
//! 当前 origin URL 做协议改写。
//!
//! 支持的形式：
//!   - HTTPS：`https://host[:port]/owner/.../repo[.git]`（http 同理）
//!   - SSH scp-like：`git@host:owner/.../repo[.git]`
//!   - SSH 显式：`ssh://[git@]host[:port]/owner/.../repo[.git]`
//!
//! 设计约束：纯字符串处理、不依赖网络与文件系统，便于单元测试覆盖。

/// 探测远程 URL 的协议：HTTPS（含 http）/ SSH / 无法识别。
#[must_use]
pub fn detect_protocol(url: &str) -> Option<&'static str> {
    let u = url.trim();
    // http/https 归为 HTTPS 类（自建 GitLab 内网可能用 http）
    if u.starts_with("https://") || u.starts_with("http://") {
        Some("https")
    // scp-like（git@）与显式 ssh:// 归为 SSH 类
    } else if u.starts_with("git@") || u.starts_with("ssh://") {
        Some("ssh")
    } else {
        // 本地路径等无法识别的形式
        None
    }
}

/// 把任意支持的远程 URL 转换为 SSH scp-like 形式（`git@host:owner/repo.git`）。
///
/// 已经是 SSH 形式时原样返回；无法解析时返回 `None`。
#[must_use]
pub fn to_ssh(url: &str) -> Option<String> {
    let u = url.trim();
    // 已是 SSH（scp-like 或 ssh://）：原样返回，避免重复改写
    if u.starts_with("git@") || u.starts_with("ssh://") {
        return Some(u.to_string());
    }
    // 仅处理 https/http；其余形式无法识别为远程地址
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))?;
    // rest 形如 host[:port]/owner/.../repo[.git]，按首个 '/' 切出 host 段与路径段
    let (host_port, path) = rest.split_once('/')?;
    // SSH scp-like 语法不带端口，去掉 host 上可能存在的 ":port"
    let host = host_port.split(':').next().unwrap_or(host_port);
    // 归一化路径（去尾斜杠、补 .git）
    let path = normalize_repo_path(path)?;
    if host.is_empty() {
        return None;
    }
    Some(format!("git@{host}:{path}"))
}

/// 把任意支持的远程 URL 转换为 HTTPS 形式（`https://host/owner/repo.git`）。
///
/// 已经是 HTTPS 形式时原样返回；无法解析时返回 `None`。
#[must_use]
pub fn to_https(url: &str) -> Option<String> {
    let u = url.trim();
    // 已是 HTTPS：原样返回
    if u.starts_with("https://") {
        return Some(u.to_string());
    }
    // 分支一：显式 ssh://[git@]host[:port]/owner/.../repo[.git]
    if let Some(rest) = u.strip_prefix("ssh://") {
        // 去掉可选的 "git@" 用户名前缀
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        // 按首个 '/' 切出 host 段与路径段
        let (host_port, path) = rest.split_once('/')?;
        // HTTPS 同样不在地址里保留 SSH 端口
        let host = host_port.split(':').next().unwrap_or(host_port);
        let path = normalize_repo_path(path)?;
        if host.is_empty() {
            return None;
        }
        return Some(format!("https://{host}/{path}"));
    }
    // 分支二：scp-like git@host:owner/.../repo[.git]，host 与路径以 ':' 分隔
    let rest = u.strip_prefix("git@")?;
    let (host, path) = rest.split_once(':')?;
    let path = normalize_repo_path(path)?;
    if host.is_empty() {
        return None;
    }
    Some(format!("https://{host}/{path}"))
}

/// 归一化仓库路径：去尾部 `/`、补 `.git` 后缀；路径为空时返回 `None`。
///
/// 这里对 `.git` 用大小写敏感比较是预期行为：Git 远程地址的 `.git` 后缀按惯例
/// 为小写，且这是 URL 路径段而非文件系统扩展名，故显式豁免 clippy 的
/// case_sensitive_file_extension_comparisons 建议（其建议改用 `Path::extension`，
/// 对 URL 字符串并不适用）。
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn normalize_repo_path(path: &str) -> Option<String> {
    // 去掉尾部多余的 '/'，避免生成 "owner/repo/.git" 之类的畸形地址
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    // 已带 .git 后缀则原样保留，否则补上，统一远程地址形态
    if trimmed.ends_with(".git") {
        Some(trimmed.to_string())
    } else {
        Some(format!("{trimmed}.git"))
    }
}

// =====================================================================
// 单元测试
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// 三类协议前缀分别被正确识别，无法识别的本地路径返回 None
    #[test]
    fn detect_protocol_variants() {
        assert_eq!(detect_protocol("https://github.com/a/b.git"), Some("https"));
        assert_eq!(
            detect_protocol("http://gitlab.local/a/b.git"),
            Some("https")
        );
        assert_eq!(detect_protocol("git@github.com:a/b.git"), Some("ssh"));
        assert_eq!(detect_protocol("ssh://git@host/a/b.git"), Some("ssh"));
        assert_eq!(detect_protocol("/local/path"), None);
    }

    /// HTTPS → SSH：基本形态（已带 .git）
    #[test]
    fn https_to_ssh_basic() {
        assert_eq!(
            to_ssh("https://github.com/owner/repo.git").as_deref(),
            Some("git@github.com:owner/repo.git")
        );
    }

    /// HTTPS → SSH：缺少 .git 后缀时应自动补全
    #[test]
    fn https_to_ssh_adds_git_suffix() {
        assert_eq!(
            to_ssh("https://github.com/owner/repo").as_deref(),
            Some("git@github.com:owner/repo.git")
        );
    }

    /// HTTPS → SSH：GitLab 嵌套 group + 端口（端口被去掉，多级路径保留）
    #[test]
    fn https_to_ssh_nested_group_and_port() {
        assert_eq!(
            to_ssh("https://gitlab.com:8443/group/sub/repo.git").as_deref(),
            Some("git@gitlab.com:group/sub/repo.git")
        );
    }

    /// SSH(scp-like) → HTTPS：基本形态
    #[test]
    fn ssh_to_https_scp_like() {
        assert_eq!(
            to_https("git@github.com:owner/repo.git").as_deref(),
            Some("https://github.com/owner/repo.git")
        );
    }

    /// SSH(显式 ssh://) → HTTPS：带端口时端口被去掉
    #[test]
    fn ssh_to_https_explicit_with_port() {
        assert_eq!(
            to_https("ssh://git@host:22/owner/repo.git").as_deref(),
            Some("https://host/owner/repo.git")
        );
    }

    /// 目标协议与当前一致时原样返回（幂等）
    #[test]
    fn already_target_protocol_is_unchanged() {
        assert_eq!(
            to_ssh("git@github.com:a/b.git").as_deref(),
            Some("git@github.com:a/b.git")
        );
        assert_eq!(
            to_https("https://github.com/a/b.git").as_deref(),
            Some("https://github.com/a/b.git")
        );
    }

    /// 无法解析的本地路径两个方向都返回 None
    #[test]
    fn unparseable_returns_none() {
        assert_eq!(to_ssh("/local/path"), None);
        assert_eq!(to_https("/local/path"), None);
    }
}
