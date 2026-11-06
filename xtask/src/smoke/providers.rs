//! Registry construction smoke coverage for all provider modalities.

use ai_interface::ModelSpec;
use ai_models_anthropic::CLAUDE_SONNET_5;
use ai_models_compat::{
    deepseek::DEEPSEEK_V4_PRO, kimi::KIMI_K3, minimax::MINIMAX_M3, qwen::QWEN_3_7_PLUS,
    xai::GROK_4_5,
};
use ai_models_google::{GEMINI_3_1_FLASH_IMAGE, GEMINI_3_6_FLASH};
use ai_models_openai::{GPT_5_6_SOL, GPT_IMAGE_2};
use ai_models_registry::{BuildResult, ProviderConfig, Registry};
use ai_models_typesafe::JEV_LATEST;
use json_http::DynJsonHttpClient;

/// Builds each registry-backed smoke adapter without making requests.
pub(super) fn build(client: DynJsonHttpClient) -> BuildResult<()> {
    let registry = Registry::new(client)
        .with_provider(ProviderConfig::new(
            ai_models_anthropic::PROVIDER_ID,
            "anthropic-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_compat::deepseek::PROVIDER_ID,
            "deepseek-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_google::PROVIDER_ID,
            "google-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_compat::kimi::PROVIDER_ID,
            "kimi-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_compat::minimax::PROVIDER_ID,
            "minimax-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_openai::PROVIDER_ID,
            "openai-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_compat::qwen::PROVIDER_ID,
            "qwen-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_typesafe::PROVIDER_ID,
            "typesafe-key",
        ))
        .with_provider(ProviderConfig::new(
            ai_models_compat::xai::PROVIDER_ID,
            "xai-key",
        ));
    let _anthropic = registry.model(&CLAUDE_SONNET_5)?;
    let _deepseek = registry.model(&DEEPSEEK_V4_PRO)?;
    let _google = registry.model(&GEMINI_3_6_FLASH)?;
    let _google_image = registry.image(&GEMINI_3_1_FLASH_IMAGE)?;
    let _google_video = registry.video(&ModelSpec::custom(
        ai_models_google::PROVIDER_ID,
        "smoke-veo-model",
        &ai_models_google::CUSTOM_DEFAULTS,
    ))?;
    let _kimi = registry.model(&KIMI_K3)?;
    let _minimax = registry.model(&MINIMAX_M3)?;
    let _openai = registry.model(&GPT_5_6_SOL)?;
    let _openai_image = registry.image(&GPT_IMAGE_2)?;
    let _qwen = registry.model(&QWEN_3_7_PLUS)?;
    let _typesafe = registry.judgment(&JEV_LATEST)?;
    let _xai = registry.model(&GROK_4_5)?;
    let _transcriber = registry.transcription(&ModelSpec::custom(
        ai_models_openai::PROVIDER_ID,
        "gpt-4o-mini-transcribe",
        &ai_models_openai::CUSTOM_DEFAULTS,
    ))?;
    Ok(())
}
