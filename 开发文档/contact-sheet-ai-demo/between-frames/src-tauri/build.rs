fn main() {
    // 迁移变化也必须触发重新构建，使内嵌 migration 与源码一致。
    println!("cargo:rerun-if-changed=migrations");
    tauri_build::build()
}
