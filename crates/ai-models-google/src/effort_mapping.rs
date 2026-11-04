//! One source for Google effort capabilities and native thinking controls.

use ai_interface::{Effort, EffortSet};

/// Catalog model id for Gemini Flash.
pub(crate) const FLASH_ID: &str = "gemini-3.6-flash";
/// Catalog model id for Gemini Flash-Lite.
pub(crate) const FLASH_LITE_ID: &str = "gemini-3.5-flash-lite";
/// Catalog model id for Gemini image generation.
pub(crate) const IMAGE_ID: &str = "gemini-3.1-flash-image";

#[derive(Clone, Copy, Eq, PartialEq)]
/// A catalog model or generic wire family with explicit effort rows.
pub(crate) enum EffortProfile {
    /// Generic token-budget controls, including a real zero-budget off value.
    Budget,
    /// Generic Gemini 3 controls for caller-owned unlisted specs.
    Named,
    /// Explicit controls supported by Gemini 3.6 Flash.
    Flash,
    /// Explicit controls supported by Gemini 3.5 Flash-Lite.
    FlashLite,
    /// No explicit effort control exposed by the image adapter.
    Image,
}

#[derive(Clone, Copy)]
/// A native control that always serializes a thinking field.
pub(crate) enum ThinkingControl {
    /// A fixed reasoning-token budget, with zero meaning off.
    Budget(i32),
    /// A native named thinking level.
    Level(&'static str),
}

type EffortMapping = (EffortProfile, Effort, ThinkingControl);

const MAPPINGS: &[EffortMapping] = &[
    (
        EffortProfile::Budget,
        Effort::Disabled,
        ThinkingControl::Budget(0),
    ),
    (
        EffortProfile::Budget,
        Effort::Low,
        ThinkingControl::Budget(1024),
    ),
    (
        EffortProfile::Budget,
        Effort::Medium,
        ThinkingControl::Budget(4096),
    ),
    (
        EffortProfile::Budget,
        Effort::High,
        ThinkingControl::Budget(8192),
    ),
    (
        EffortProfile::Budget,
        Effort::ExtraHigh,
        ThinkingControl::Budget(16_384),
    ),
    (
        EffortProfile::Budget,
        Effort::Max,
        ThinkingControl::Budget(32_768),
    ),
    (
        EffortProfile::Named,
        Effort::Disabled,
        ThinkingControl::Budget(0),
    ),
    (
        EffortProfile::Named,
        Effort::Low,
        ThinkingControl::Level("low"),
    ),
    (
        EffortProfile::Named,
        Effort::Medium,
        ThinkingControl::Level("medium"),
    ),
    (
        EffortProfile::Named,
        Effort::High,
        ThinkingControl::Level("high"),
    ),
    (
        EffortProfile::Named,
        Effort::ExtraHigh,
        ThinkingControl::Level("high"),
    ),
    (
        EffortProfile::Named,
        Effort::Max,
        ThinkingControl::Level("high"),
    ),
    (
        EffortProfile::Flash,
        Effort::Low,
        ThinkingControl::Level("low"),
    ),
    (
        EffortProfile::Flash,
        Effort::Medium,
        ThinkingControl::Level("medium"),
    ),
    (
        EffortProfile::Flash,
        Effort::High,
        ThinkingControl::Level("high"),
    ),
    (
        EffortProfile::FlashLite,
        Effort::Low,
        ThinkingControl::Level("minimal"),
    ),
    (
        EffortProfile::FlashLite,
        Effort::Medium,
        ThinkingControl::Level("medium"),
    ),
    (
        EffortProfile::FlashLite,
        Effort::High,
        ThinkingControl::Level("high"),
    ),
];

/// Derives the catalog effort set from the rows that have native controls.
pub(crate) const fn supported_efforts(profile: EffortProfile) -> EffortSet {
    let mut efforts = EffortSet::empty();
    let mut index = 0;
    while index < MAPPINGS.len() {
        let (candidate, effort, _) = MAPPINGS[index];
        if candidate as u8 == profile as u8 {
            efforts = efforts.with(effort);
        }
        index += 1;
    }
    efforts
}

/// Selects a native control without interpreting absence as disabled thinking.
pub(crate) fn thinking_control(model_id: &str, effort: Effort) -> Option<ThinkingControl> {
    let generic = if model_id.starts_with("gemini-3") {
        EffortProfile::Named
    } else {
        EffortProfile::Budget
    };
    let profile = match model_id {
        FLASH_ID => EffortProfile::Flash,
        FLASH_LITE_ID => EffortProfile::FlashLite,
        IMAGE_ID => EffortProfile::Image,
        _ => generic,
    };
    let control = lookup(profile, effort);
    if control.is_some() || effort == Effort::Disabled {
        return control;
    }
    lookup(generic, effort)
}

fn lookup(profile: EffortProfile, effort: Effort) -> Option<ThinkingControl> {
    MAPPINGS.iter().find_map(|(candidate, level, control)| {
        (*candidate == profile && *level == effort).then_some(*control)
    })
}
