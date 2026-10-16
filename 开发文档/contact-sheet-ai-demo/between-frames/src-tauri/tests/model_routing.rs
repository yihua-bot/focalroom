use between_frames_lib::agent::{
    capabilities::{Capability, CapabilityProfile},
    openai::{OpenAiConfig, OpenAiProvider},
    router::*,
};

fn provider(endpoint: &str, model: &str, key: &str) -> OpenAiProvider {
    OpenAiProvider::with_capability_profile(
        OpenAiConfig {
            base_url: endpoint.into(),
            model: model.into(),
            api_key: key.into(),
        },
        CapabilityProfile::declared(
            endpoint,
            model,
            &[
                Capability::Vision,
                Capability::ToolCalling,
                Capability::StructuredOutput,
                Capability::InstructionFollowing,
            ],
        ),
    )
    .unwrap()
}
#[test]
fn configured_model_identity_is_real_unmeasured_and_credential_independent() {
    let configured = provider(
        "https://example.invalid/api/",
        "configured-model",
        "dummy-key-one",
    );
    let rotated = provider(
        "https://example.invalid/api",
        "configured-model",
        "dummy-key-two",
    );
    assert_eq!(configured.identity(), rotated.identity());
    assert_ne!(
        configured.identity(),
        provider(
            "https://other.invalid/api",
            "configured-model",
            "dummy-key-one"
        )
        .identity()
    );
    assert_ne!(
        configured.identity(),
        provider(
            "https://example.invalid/api",
            "other-model",
            "dummy-key-one"
        )
        .identity()
    );
    let serialized = serde_json::to_string(&configured.identity()).unwrap();
    assert!(!serialized.contains("dummy-key") && !serialized.contains("example.invalid"));
    let region = RegionPolicy::default();
    let request = RouteRequest {
        task_type: TaskType::AgentOrchestrate,
        tier: ServiceTier::Standard,
        region: &region,
        budget_remaining_usd: 0.50,
        images_required: false,
        image_consent: false,
    };
    let decision =
        route_for_provider(&request, "openai", Some(configured.route().unwrap())).unwrap();
    assert_eq!(decision.model_id, "configured-model");
    assert_eq!(decision.quality, "unmeasured");
    assert_eq!(decision.residency, "unknown");
    assert_eq!(
        route_for_provider(&request, "openai", None)
            .unwrap_err()
            .code,
        "MODEL_CAPABILITY_MISSING"
    );
    assert_eq!(
        route_for_provider(&request, "mock", Some(configured.route().unwrap()))
            .unwrap()
            .provider_id,
        "mock"
    );
}

#[test]
fn unknown_residency_and_unimplemented_embedding_cannot_pass_routing_as_verified() {
    let region = RegionPolicy {
        restricted: vec![],
        allowed_residencies: vec!["CN".into(), "unknown".into()],
    };
    let request = RouteRequest {
        task_type: TaskType::VisionObserve,
        tier: ServiceTier::Standard,
        region: &region,
        budget_remaining_usd: 0.50,
        images_required: true,
        image_consent: true,
    };
    assert_eq!(
        route_for_provider(
            &request,
            "openai",
            Some(
                provider("https://example.invalid/api", "configured-model", "fake")
                    .route()
                    .unwrap()
            )
        )
        .unwrap_err()
        .code,
        "PRIVACY_REGION_REJECTED"
    );
    let mut configured = provider("https://example.invalid/api", "configured-model", "fake")
        .route()
        .unwrap();
    configured.residency = "CN".into();
    assert!(route_for_provider(&request, "openai", Some(configured)).is_ok());
    let unrestricted = RegionPolicy::default();
    let index = RouteRequest {
        task_type: TaskType::Index,
        region: &unrestricted,
        ..request
    };
    assert_eq!(
        route(&index, true).unwrap_err().code,
        "MODEL_CAPABILITY_MISSING"
    );
}

#[test]
fn capability_file_is_bounded_strict_and_bound_to_exact_endpoint_and_model() {
    use serde_json::json;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("profile.json");
    let profile = CapabilityProfile::declared(
        "https://example.invalid/v1/",
        "custom",
        &[Capability::ToolCalling, Capability::InstructionFollowing],
    );
    let original = serde_json::to_value(&profile).unwrap();
    std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    let loaded =
        CapabilityProfile::from_file(&path, " https://example.invalid/v1 ", "custom").unwrap();
    assert_eq!(loaded.sha256(), profile.sha256());
    for (endpoint, model) in [
        ("https://example.invalid/v1.evil", "custom"),
        ("http://example.invalid/v1", "custom"),
        ("https://example.invalid/v1", "other"),
    ] {
        assert!(CapabilityProfile::from_file(&path, endpoint, model).is_err());
    }
    for (field, value) in [
        ("version", json!(2)),
        ("version", json!("1")),
        ("capabilities", json!(["embedding_or_multimodal"])),
        ("capabilities", json!(["VISION"])),
        ("capabilities", json!(null)),
        ("quality", json!("professional")),
        ("residency", json!("CN")),
        ("apiKey", json!("private-config-value")),
        ("endpointSha256", json!("https://example.invalid/v1")),
    ] {
        let mut malformed = original.clone();
        malformed[field] = value;
        std::fs::write(&path, malformed.to_string()).unwrap();
        let error = CapabilityProfile::from_file(&path, "https://example.invalid/v1", "custom")
            .unwrap_err();
        let message = serde_json::to_string(&error).unwrap();
        assert_eq!(error.code, "MODEL_CAPABILITY_MISSING");
        for private in [
            "private-config-value",
            "example.invalid",
            temp.path().to_str().unwrap(),
        ] {
            assert!(!message.contains(private));
        }
    }
    std::fs::write(&path, " ".repeat(16 * 1024 + 1)).unwrap();
    assert!(CapabilityProfile::from_file(&path, "https://example.invalid/v1", "custom").is_err());
    assert!(
        CapabilityProfile::from_file(temp.path(), "https://example.invalid/v1", "custom").is_err()
    );
    assert!(
        CapabilityProfile::from_file(
            &temp.path().join("missing"),
            "https://example.invalid/v1",
            "custom"
        )
        .is_err()
    );
    std::fs::write(&path, "{invalid-private-config-value").unwrap();
    assert!(CapabilityProfile::from_file(&path, "https://example.invalid/v1", "custom").is_err());
}

#[test]
fn capability_sets_are_canonical_frozen_and_do_not_grant_extra_privileges() {
    use serde_json::json;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("profile.json");
    let declared = CapabilityProfile::declared(
        "https://example.invalid",
        "custom",
        &[Capability::ToolCalling, Capability::InstructionFollowing],
    );
    let mut raw = serde_json::to_value(&declared).unwrap();
    raw["capabilities"] = json!(["instruction_following", "tool_calling", "tool_calling"]);
    std::fs::write(&path, raw.to_string()).unwrap();
    let loaded = CapabilityProfile::from_file(&path, "https://example.invalid", "custom").unwrap();
    assert_eq!(loaded.sha256(), declared.sha256());
    let config = || OpenAiConfig {
        base_url: "https://example.invalid".into(),
        model: "custom".into(),
        api_key: "private-key".into(),
    };
    let frozen = OpenAiProvider::with_capability_profile(config(), loaded).unwrap();
    let before = frozen.identity();
    raw["capabilities"] = json!([
        "vision",
        "tool_calling",
        "instruction_following",
        "structured_output"
    ]);
    std::fs::write(&path, raw.to_string()).unwrap();
    let changed = OpenAiProvider::with_capability_profile(
        config(),
        CapabilityProfile::from_file(&path, "https://example.invalid", "custom").unwrap(),
    )
    .unwrap();
    assert_eq!(frozen.identity(), before);
    assert_ne!(changed.identity(), before);
    assert_eq!(frozen.route().unwrap().quality, "unmeasured");
    assert_eq!(frozen.route().unwrap().residency, "unknown");
    assert_eq!(frozen.capability_source(), Some("operator_declaration"));
    let other = CapabilityProfile::declared("https://other.invalid", "custom", &[]);
    assert!(OpenAiProvider::with_capability_profile(config(), other).is_err());
}

#[test]
fn routing_requires_all_protocol_capabilities_including_requested_image_input() {
    let region = RegionPolicy::default();
    let request = RouteRequest {
        task_type: TaskType::AgentOrchestrate,
        tier: ServiceTier::Standard,
        region: &region,
        budget_remaining_usd: 1.0,
        images_required: false,
        image_consent: true,
    };
    let partial = |caps| Route::responses("declared", caps);
    for tier in [ServiceTier::Standard, ServiceTier::Pro] {
        let req = RouteRequest { tier, ..request };
        assert_eq!(
            route_for_provider(&req, "openai", Some(partial(vec!["tool_calling"])))
                .unwrap_err()
                .code,
            "MODEL_CAPABILITY_MISSING"
        );
    }
    let text = partial(vec!["tool_calling", "instruction_following"]);
    assert!(route_for_provider(&request, "openai", Some(text.clone())).is_ok());
    let images = RouteRequest {
        images_required: true,
        ..request
    };
    assert_eq!(
        route_for_provider(&images, "openai", Some(text))
            .unwrap_err()
            .code,
        "MODEL_CAPABILITY_MISSING"
    );
    let vision = RouteRequest {
        task_type: TaskType::VisionObserve,
        ..images
    };
    assert_eq!(
        route_for_provider(
            &vision,
            "openai",
            Some(partial(vec!["vision", "instruction_following"]))
        )
        .unwrap_err()
        .code,
        "MODEL_CAPABILITY_MISSING"
    );
    assert_eq!(
        route_for_provider(
            &vision,
            "mock",
            Some(
                provider("https://unused.invalid", "custom", "fake")
                    .route()
                    .unwrap()
            )
        )
        .unwrap_err()
        .code,
        "MODEL_CAPABILITY_MISSING"
    );
    let no_consent = RouteRequest {
        image_consent: false,
        ..vision
    };
    assert_eq!(
        route_for_provider(
            &no_consent,
            "openai",
            Some(
                provider("https://unused.invalid", "custom", "fake")
                    .route()
                    .unwrap()
            )
        )
        .unwrap_err()
        .code,
        "PRIVACY_REGION_REJECTED"
    );
}

#[test]
fn known_profiles_and_explicit_env_override_fail_closed_without_inheriting_proxy_capabilities() {
    use between_frames_lib::agent::capabilities::PROFILE_PATH_ENV;
    // 本测试二进制其他测试均显式注入 profile，不读取这个环境变量。
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                if let Some(value) = &self.0 {
                    std::env::set_var(PROFILE_PATH_ENV, value);
                } else {
                    std::env::remove_var(PROFILE_PATH_ENV);
                }
            }
        }
    }
    let _restore = Restore(std::env::var_os(PROFILE_PATH_ENV));
    unsafe {
        std::env::remove_var(PROFILE_PATH_ENV);
    }
    let config = |endpoint: &str, model: &str| OpenAiConfig {
        base_url: endpoint.into(),
        model: model.into(),
        api_key: "dummy-private-key".into(),
    };
    for (endpoint, model, source) in [
        (
            "https://api.openai.com/v1/",
            "gpt-4o-mini",
            "official_documentation",
        ),
        (
            "https://api.openai.com/v1",
            "gpt-4o-mini-2024-07-18",
            "official_documentation",
        ),
        (
            "https://api.deepseek.com",
            "deepseek-flash",
            "local_protocol_smoke",
        ),
    ] {
        let provider = OpenAiProvider::new(config(endpoint, model));
        assert!(provider.configuration_error().is_none());
        assert_eq!(provider.capability_source(), Some(source));
        assert_eq!(provider.route().unwrap().quality, "unmeasured");
    }
    for (endpoint, model) in [
        ("https://proxy.invalid/v1", "gpt-4o-mini"),
        ("https://api.deepseek.com.evil.invalid", "deepseek-flash"),
        ("http://api.deepseek.com", "deepseek-flash"),
        ("https://api.deepseek.com", "unknown-model"),
    ] {
        let provider = OpenAiProvider::new(config(endpoint, model));
        assert_eq!(
            provider.route().unwrap_err().code,
            "MODEL_CAPABILITY_MISSING"
        );
        assert!(provider.capability_source().is_none());
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("profile.json");
    let profile = CapabilityProfile::declared(
        "https://proxy.invalid/v1",
        "custom",
        &[Capability::ToolCalling, Capability::InstructionFollowing],
    );
    std::fs::write(&path, serde_json::to_vec(&profile).unwrap()).unwrap();
    unsafe {
        std::env::set_var(PROFILE_PATH_ENV, &path);
    }
    let provider = OpenAiProvider::new(config("https://proxy.invalid/v1", "custom"));
    assert!(provider.configuration_error().is_none());
    assert_eq!(provider.capability_source(), Some("operator_declaration"));
    std::fs::write(&path, "invalid-private-body").unwrap();
    assert!(
        provider.configuration_error().is_none(),
        "活动实例不重读配置文件"
    );
    for endpoint in ["https://proxy.invalid/v1", "https://api.openai.com/v1"] {
        let rejected = OpenAiProvider::new(config(endpoint, "gpt-4o-mini"));
        assert!(
            rejected.configuration_error().is_some(),
            "无效显式配置不能回退默认配置"
        );
        assert!(
            !rejected
                .configuration_error()
                .unwrap()
                .contains("private-body")
        );
    }
}
