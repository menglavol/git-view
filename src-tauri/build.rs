// build.rs — Tauri 构建脚本
// 调用 tauri-build 完成资源打包、capabilities 校验、平台元数据生成。
fn main() {
    tauri_build::build();
}
