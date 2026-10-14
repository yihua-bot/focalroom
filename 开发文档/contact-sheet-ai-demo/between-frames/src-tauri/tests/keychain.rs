// macOS 钥匙串集成测试：使用独立测试 service，不触碰真实条目。
#[cfg(target_os = "macos")]
mod tests {
    const TEST_SERVICE: &str = "between-frames.test-key";

    #[test]
    fn store_load_delete_roundtrip() {
        // 清理可能的残留
        let _ = between_frames_lib::keychain::delete(TEST_SERVICE);
        assert!(between_frames_lib::keychain::load(TEST_SERVICE).is_none());
        between_frames_lib::keychain::store(TEST_SERVICE, "sk-test-abcd1234").unwrap();
        assert_eq!(
            between_frames_lib::keychain::load(TEST_SERVICE).as_deref(),
            Some("sk-test-abcd1234")
        );
        // 覆盖写入（-U 更新）
        between_frames_lib::keychain::store(TEST_SERVICE, "sk-test-wxyz9876").unwrap();
        assert_eq!(
            between_frames_lib::keychain::load(TEST_SERVICE).as_deref(),
            Some("sk-test-wxyz9876")
        );
        between_frames_lib::keychain::delete(TEST_SERVICE).unwrap();
        assert!(between_frames_lib::keychain::load(TEST_SERVICE).is_none());
    }

    #[test]
    fn rejects_empty_key() {
        assert!(between_frames_lib::keychain::store(TEST_SERVICE, "  ").is_err());
    }
}
