use proptest::prelude::*;
use voice_type::config::{AppConfig, AudioConfig, OpenAiProviderConfig, ProviderConfig, Secret};
use voice_type::postprocess::config::{PostProcessorConfig, ProcessorName};
use voice_type::types::{LanguageCode, RmsLevel, SampleRate};

fn arb_secret() -> impl Strategy<Value = Secret> {
    prop_oneof![
        "[a-z]{10,20}".prop_map(|s| Secret::String(s.into())),
        "[A-Z_]{5,15}".prop_map(Secret::FromEnv),
        "echo [a-z]{5}".prop_map(Secret::FromCommand),
    ]
}

fn arb_language_code() -> impl Strategy<Value = Option<LanguageCode>> {
    prop_oneof![
        Just(None),
        "[a-z]{2}".prop_map(|s| Some(LanguageCode::new(&s).unwrap())),
    ]
}

fn arb_app_config() -> impl Strategy<Value = AppConfig> {
    (arb_secret(), arb_language_code(), 8000u32..=48000u32).prop_map(
        |(secret, language, sample_rate)| AppConfig {
            provider: ProviderConfig::OpenAi(OpenAiProviderConfig {
                api_key: secret,
                model: "whisper-1".to_owned(),
                language,
                prompt: None,
                timeout: std::time::Duration::from_secs(30),
            }),
            audio: AudioConfig {
                device: None,
                sample_rate: SampleRate::new(sample_rate).unwrap(),
                silence_threshold: RmsLevel::new(0.01),
                max_duration: std::time::Duration::from_secs(300),
            },
            post_processing: Vec::new(),
        },
    )
}

proptest! {
    #[test]
    fn config_serde_roundtrip(config in arb_app_config()) {
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: AppConfig = serde_yaml::from_str(&yaml).unwrap();

        // Compare key fields (Secret can't be compared directly)
        match (&parsed.provider, &config.provider) {
            (
                voice_type::config::ProviderConfig::OpenAi(p),
                voice_type::config::ProviderConfig::OpenAi(c),
            ) => {
                prop_assert_eq!(&p.model, &c.model);
            }
            _ => prop_assert!(false, "Provider variant mismatch"),
        }
        prop_assert_eq!(parsed.audio.sample_rate.hz(), config.audio.sample_rate.hz());
    }

    #[test]
    fn secret_yaml_roundtrip_string(s in "[a-zA-Z0-9]{1,30}") {
        let secret = Secret::String(s.clone().into());
        let yaml = serde_yaml::to_string(&secret).unwrap();
        let parsed: Secret = serde_yaml::from_str(&yaml).unwrap();
        // Verify tag is preserved
        assert!(yaml.contains("!String") || yaml.contains("String"));
        // Can't compare secrets directly, but verify it's the String variant
        if let Secret::String(ref inner) = parsed {
            assert_eq!(inner.unsecure(), s);
        } else {
            panic!("Expected Secret::String variant");
        }
    }

    #[test]
    fn secret_yaml_roundtrip_from_env(var in "[A-Z][A-Z_]{2,15}") {
        let secret = Secret::FromEnv(var.clone());
        let yaml = serde_yaml::to_string(&secret).unwrap();
        let parsed: Secret = serde_yaml::from_str(&yaml).unwrap();
        if let Secret::FromEnv(ref v) = parsed {
            assert_eq!(v, &var);
        } else {
            panic!("Expected Secret::FromEnv variant");
        }
    }

    #[test]
    fn language_code_valid(code in "[a-z]{2}") {
        let lang = LanguageCode::new(&code);
        prop_assert!(lang.is_ok());
        let lang = lang.unwrap();
        prop_assert_eq!(lang.as_str(), code.as_str());
    }

    #[test]
    fn language_code_invalid_length(code in "[a-z]{1}|[a-z]{3,10}") {
        let lang = LanguageCode::new(&code);
        prop_assert!(lang.is_err());
    }

    #[test]
    fn post_processor_config_yaml_roundtrip(
        name in "[A-Za-z][A-Za-z0-9 ]{0,19}",
        prompt in "[A-Za-z ]{5,50}",
        secret in "[a-z]{10,20}",
        model in prop_oneof![Just("gpt-4o-mini"), Just("gpt-4o"), Just("gpt-3.5-turbo")],
        temperature in prop_oneof![Just(None), (0.0f32..=2.0f32).prop_map(Some)],
        max_tokens in prop_oneof![Just(None), (1u32..=4096u32).prop_map(Some)],
        max_retries in 0u32..=10u32,
    ) {
        let config = PostProcessorConfig {
            name: ProcessorName::new(&name).unwrap(),
            system_prompt: prompt.clone(),
            api_key: Secret::from_string(&secret),
            model: model.to_owned(),
            endpoint: "https://api.openai.com".to_owned(),
            timeout: std::time::Duration::from_secs(15),
            temperature,
            max_tokens,
            max_retries,
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: PostProcessorConfig = serde_yaml::from_str(&yaml).unwrap();

        prop_assert_eq!(parsed.name.as_str(), config.name.as_str());
        prop_assert_eq!(&parsed.system_prompt, &config.system_prompt);
        prop_assert_eq!(&parsed.model, &config.model);
        prop_assert_eq!(&parsed.endpoint, &config.endpoint);
        prop_assert_eq!(parsed.temperature, config.temperature);
        prop_assert_eq!(parsed.max_tokens, config.max_tokens);
        prop_assert_eq!(parsed.max_retries, config.max_retries);
    }
}
