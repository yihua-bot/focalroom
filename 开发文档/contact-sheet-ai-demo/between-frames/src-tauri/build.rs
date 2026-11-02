fn main() {
    // 迁移变化也必须触发重新构建，使内嵌 migration 与源码一致。
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=native/sentence_embeddings.m");
    if matches!(
        std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
        Ok("macos" | "ios")
    ) {
        build_sentence_embeddings();
    }
    #[cfg(feature = "desktop")]
    tauri_build::build()
}

fn build_sentence_embeddings() {
    use std::{path::PathBuf, process::Command};
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let object = output.join("sentence_embeddings.o");
    let archive = output.join("libbf_sentence_embeddings.a");
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("不支持的 macOS 架构"),
    };
    let minimum = if arch == "arm64" {
        "-mmacosx-version-min=11.0"
    } else {
        "-mmacosx-version-min=10.13"
    };
    let mut compiler = Command::new("xcrun");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        let target = std::env::var("TARGET").expect("TARGET");
        let simulator = target.ends_with("-sim") || target.starts_with("x86_64");
        let sdk = if simulator {
            "iphonesimulator"
        } else {
            "iphoneos"
        };
        let sdk_path = Command::new("xcrun")
            .args(["--sdk", sdk, "--show-sdk-path"])
            .output()
            .expect("iOS SDK");
        assert!(sdk_path.status.success(), "无法读取 iOS SDK");
        let triple = format!(
            "{arch}-apple-ios17.0{}",
            if simulator { "-simulator" } else { "" }
        );
        compiler.args([
            "--sdk",
            sdk,
            "clang",
            "-target",
            &triple,
            "-isysroot",
            String::from_utf8(sdk_path.stdout).expect("SDK 路径").trim(),
        ]);
    } else {
        compiler.args(["clang", "-arch", arch, minimum]);
    }
    let compile = compiler
        .args([
            "-fobjc-arc",
            "-O2",
            "-c",
            "native/sentence_embeddings.m",
            "-o",
        ])
        .arg(&object)
        .status()
        .expect("编译本地句向量桥");
    assert!(compile.success(), "本地句向量桥编译失败");
    let archived = Command::new("/usr/bin/ar")
        .arg("crs")
        .arg(&archive)
        .arg(&object)
        .status()
        .expect("归档本地句向量桥");
    assert!(archived.success(), "本地句向量桥归档失败");
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=bf_sentence_embeddings");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-arg=-Wl,-weak_framework,NaturalLanguage");
}
