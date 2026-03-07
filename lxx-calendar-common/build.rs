use std::path::Path;

fn main() {
    // 从 workspace 根目录读取 .env
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap();
    let env_path = workspace_root.join(".env");

    // 尝试加载 .env 文件（如果存在）
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    }

    // 必需的环境变量列表
    let required_vars = [
        "QWEATHER_API_HOST",
        "QWEATHER_LOCATION",
        "QWEATHER_KEY_ID",
        "QWEATHER_PROJECT_ID",
        "QWEATHER_PRIVATE_KEY",
    ];

    // 检查并设置每个必需的环境变量
    for var in required_vars {
        let value = std::env::var(var).unwrap_or_else(|_| {
            panic!(
                "\n\n========== 编译错误 ==========\n\
                 缺少必需的环境变量: '{}'\n\n\
                 请按以下步骤操作:\n\
                 1. 复制 .env.example 为 .env\n\
                 2. 在 .env 中填写和风天气 API 凭据\n\
                 3. 重新编译\n\n\
                 获取凭据: https://console.qweather.com/\n\
                 ==============================\n",
                var
            )
        });
        println!("cargo:rustc-env={}={}", var, value);
    }

    // 监听 .env 文件变化，触发重新编译
    println!("cargo:rerun-if-changed={}", env_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join(".env.example").display()
    );
}
