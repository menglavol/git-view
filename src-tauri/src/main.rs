//! `GitView` 后端二进制入口。
//!
//! 该文件极简：仅调用 `gitview_lib::run()` 启动 Tauri 应用。
//! 实际的应用初始化逻辑（日志、数据库、迁移、command 注册、`AppState`）
//! 全部封装在 `lib.rs` 中以便集成测试可独立调用。
//!
//! Windows 平台需要关闭控制台窗口，故添加 `windows_subsystem = "windows"`
//! 属性；该属性仅在 release 构建中生效，开发期保留控制台便于调试日志输出。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gitview_lib::run();
}
