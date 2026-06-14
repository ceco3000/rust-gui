//! cargo-generate 模板集成测试
//!
//! 验证模板文件结构正确、占位符语法有效、生成的项目可构建。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 模板目录的路径（相对于 workspace 根）。
fn template_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("rgui-template")
}

/// 验证模板目录存在且结构完整。
#[test]
fn template_directory_exists() {
    let dir = template_dir();
    assert!(dir.exists(), "rgui-template/ 目录必须存在");
    assert!(dir.is_dir(), "rgui-template/ 必须是目录");
}

/// 验证 cargo-generate.toml 存在且包含 [template] 段。
#[test]
fn cargo_generate_toml_is_valid() {
    let path = template_dir().join("cargo-generate.toml");
    assert!(path.exists(), "cargo-generate.toml 必须存在");

    let content = fs::read_to_string(&path).expect("读取 cargo-generate.toml");

    // 必须有 [template] 段
    assert!(
        content.contains("[template]"),
        "cargo-generate.toml 必须有 [template] 段"
    );
}

/// 验证 Cargo.toml 模板存在且包含必需的占位符。
#[test]
fn cargo_toml_has_placeholders() {
    let path = template_dir().join("Cargo.toml");
    assert!(path.exists(), "Cargo.toml 模板必须存在");

    let content = fs::read_to_string(&path).expect("读取 Cargo.toml");

    // 必须包含 project-name 占位符
    assert!(
        content.contains("{{project-name}}"),
        "Cargo.toml 必须包含 '{{project-name}}' 占位符"
    );
    // 必须包含 rgui 依赖
    assert!(content.contains("rgui"), "Cargo.toml 必须包含 rgui 依赖");
    // 必须包含 rust-version 约束（对齐 workspace 的 1.85）
    assert!(
        content.contains("rust-version"),
        "Cargo.toml 必须包含 rust-version 约束"
    );
    // 必须包含 [lints] 安全配置
    assert!(
        content.contains("[lints"),
        "Cargo.toml 必须包含 [lints] 安全配置"
    );
    assert!(
        content.contains("unsafe_code = \"deny\""),
        "Cargo.toml 必须包含 unsafe_code deny 配置"
    );
}

/// 验证 src/main.rs 存在且包含占位符。
#[test]
fn main_rs_exists_with_placeholder() {
    let path = template_dir().join("src").join("main.rs");
    assert!(path.exists(), "src/main.rs 必须存在");

    let content = fs::read_to_string(&path).expect("读取 src/main.rs");

    // main.rs 通常也包含 project-name 引用（窗口标题等）
    assert!(
        content.contains("{{project-name}}"),
        "src/main.rs 应包含 '{{project-name}}' 占位符"
    );

    // 必须包含 fn main
    assert!(content.contains("fn main"), "src/main.rs 必须有 fn main");
}

/// 验证 .gitignore 存在。
#[test]
fn gitignore_exists() {
    let path = template_dir().join(".gitignore");
    assert!(path.exists(), ".gitignore 必须存在");
}

/// 验证模板占位符替换后可编译。
///
/// 此测试自动将模板中的 {{project-name}} 替换为测试项目名，
/// 然后将 rgui 依赖改为指向 workspace 内的本地路径，
/// 最后执行 `cargo check` 验证编译通过。
#[test]
fn generated_project_compiles() {
    let dir = template_dir();
    assert!(dir.exists(), "模板目录必须存在，跳过编译测试");

    // 创建临时项目目录
    let temp_root = std::env::temp_dir().join("rgui-template-test");
    let project_dir = temp_root.join("test-rgui-app");

    // 清理旧的测试残留
    let _ = fs::remove_dir_all(&temp_root);

    // 复制模板文件并替换占位符
    copy_template(&dir, &temp_root, "test-rgui-app");

    // 修改 Cargo.toml：rgui 依赖改为指向 workspace 本地路径
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let rgui_path = workspace_root.join("rgui");
    let rgui_path_str = rgui_path.to_str().expect("路径合法");
    let cargo_toml_path = project_dir.join("Cargo.toml");
    let cargo_content = fs::read_to_string(&cargo_toml_path).expect("读取生成的 Cargo.toml");
    let modified = cargo_content.replace(
        "rgui = \"0.1\"",
        &format!("rgui = {{ path = {rgui_path_str:?} }}"),
    );
    fs::write(&cargo_toml_path, modified).expect("写入修改后的 Cargo.toml");

    // 运行 cargo check
    let output = Command::new("cargo")
        .args(["check"])
        .current_dir(&project_dir)
        .output()
        .expect("cargo check 执行失败");

    let status = output.status;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(status.success(), "生成的项目编译失败:\n{}", stderr,);
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 递归复制模板目录，将占位符替换为实际项目名。
fn copy_template(src: &Path, dest_root: &Path, project_name: &str) {
    let dest = dest_root.join(project_name);
    fs::create_dir_all(&dest).expect("创建目标目录");

    copy_recursive(src, &dest, project_name);
}

fn copy_recursive(src: &Path, dest: &Path, project_name: &str) {
    for entry in fs::read_dir(src).expect("读取模板目录") {
        let entry = entry.expect("读取条目");
        let src_path = entry.path();
        let file_name = entry.file_name();

        // 跳过 cargo-generate 配置文件（不复制到生成项目）
        if file_name == "cargo-generate.toml" {
            continue;
        }

        let dest_path = dest.join(&file_name);

        if src_path.is_dir() {
            // 跳过 target 目录
            if file_name == "target" {
                continue;
            }
            fs::create_dir_all(&dest_path).expect("创建子目录");
            copy_recursive(&src_path, &dest_path, project_name);
        } else {
            let content = fs::read_to_string(&src_path).expect("读取模板文件");

            // 替换占位符
            let replaced = content.replace("{{project-name}}", project_name);

            fs::write(&dest_path, replaced).expect("写入生成文件");
        }
    }
}

/// 清理前一次测试的残留。
///
/// 使用 `ignore` 属性避免干扰并行测试，手动需要时运行。
#[test]
#[ignore]
fn cleanup_temp_project() {
    let temp_root = std::env::temp_dir().join("rgui-template-test");
    if temp_root.exists() {
        let _ = fs::remove_dir_all(&temp_root);
    }
}
