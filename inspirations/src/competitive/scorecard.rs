use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum CompetitiveSegment {
    WisprFlowParity,
    GrammarlyParity,
    FlowNativeAdvantage,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum FeatureStatus {
    Shipped,
    Partial,
    Planned,
    Missing,
}

impl FeatureStatus {
    pub fn multiplier(self) -> f32 {
        match self {
            Self::Shipped => 1.0,
            Self::Partial => 0.6,
            Self::Planned => 0.25,
            Self::Missing => 0.0,
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct CompetitiveFeature {
    pub segment: CompetitiveSegment,
    pub feature: String,
    pub weight: u8,
    pub wispr_flow_has_it: bool,
    pub grammarly_has_it: bool,
    pub flow_status: FeatureStatus,
    pub notes: String,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct SegmentScore {
    pub segment: CompetitiveSegment,
    pub earned: f32,
    pub possible: f32,
    pub score_out_of_100: u8,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct CompetitiveScorecard {
    pub measured_on: String,
    pub overall_score_out_of_100: u8,
    pub wispr_replacement_score_out_of_100: u8,
    pub grammarly_replacement_score_out_of_100: u8,
    pub flow_native_advantage_score_out_of_100: u8,
    pub segments: Vec<SegmentScore>,
    pub features: Vec<CompetitiveFeature>,
    pub top_gaps: Vec<String>,
}

pub fn default_competitive_scorecard() -> CompetitiveScorecard {
    let features = vec![
        feature(
            CompetitiveSegment::WisprFlowParity,
            "System-wide dictation in apps and websites",
            8,
            true,
            false,
            FeatureStatus::Partial,
            "Reference CLI and voice paths exist, but production-grade system-wide integrations still need hardening.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "Push-to-talk, hotkeys, and hands-free activation",
            5,
            true,
            false,
            FeatureStatus::Partial,
            "Push-to-talk and wake-word foundations exist, but polished host integrations are still incomplete.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "Dictation cleanup, punctuation, list formatting, and spoken corrections",
            8,
            true,
            false,
            FeatureStatus::Partial,
            "The dictation engine now has reusable cleanup heuristics, but it is not yet production-complete.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "Snippets and custom dictionary",
            6,
            true,
            true,
            FeatureStatus::Partial,
            "Shared and personal snippet/dictionary primitives exist, but host persistence and UI still need work.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "App-aware styles and writing context",
            5,
            true,
            true,
            FeatureStatus::Partial,
            "The experience layer models app context and styles, but per-app learning and deep host hooks are not finished.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "IDE variable recognition and voice file tagging",
            6,
            true,
            false,
            FeatureStatus::Planned,
            "Workspace file tagging heuristics exist, but full IDE symbol extraction and live integration are not complete.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "Hub, transcription history, team snippets/dictionary, and usage dashboards",
            6,
            true,
            true,
            FeatureStatus::Partial,
            "Usage snapshots and shared assets exist at the library level, but the full hub/admin product layer is incomplete.",
        ),
        feature(
            CompetitiveSegment::WisprFlowParity,
            "Mobile parity, accessibility, and polished user surfaces",
            6,
            true,
            false,
            FeatureStatus::Planned,
            "Host blueprints exist for mobile, Tauri, Flutter, and browser targets, but those surfaces are not shipped yet.",
        ),
        feature(
            CompetitiveSegment::GrammarlyParity,
            "Grammar, spelling, and fluency correction",
            8,
            false,
            true,
            FeatureStatus::Partial,
            "Harper-backed grammar correction is integrated, but deeper proofreading polish is still needed.",
        ),
        feature(
            CompetitiveSegment::GrammarlyParity,
            "Clarity, tone, and rewrite suggestions",
            6,
            false,
            true,
            FeatureStatus::Partial,
            "Typing and text-command engines provide rewrite heuristics, but richer rewrite quality and explanation layers are still missing.",
        ),
        feature(
            CompetitiveSegment::GrammarlyParity,
            "Brand tones, style guides, and team language controls",
            5,
            false,
            true,
            FeatureStatus::Partial,
            "Style presets and shared dictionary primitives exist, but centralized team governance is incomplete.",
        ),
        feature(
            CompetitiveSegment::GrammarlyParity,
            "Translation and multilingual writing assistance",
            4,
            true,
            true,
            FeatureStatus::Planned,
            "This is planned in the architecture, but there is no finished multilingual writing surface yet.",
        ),
        feature(
            CompetitiveSegment::GrammarlyParity,
            "Plagiarism, citations, fact-checking, and academic assistance",
            8,
            false,
            true,
            FeatureStatus::Missing,
            "This is a major competitive gap versus Grammarly today.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Device-aware local runtime broker",
            8,
            false,
            false,
            FeatureStatus::Partial,
            "Runtime broker, device profile, and execution plans exist, but full backend coverage is still incomplete.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Unlimited offline local inference",
            7,
            false,
            false,
            FeatureStatus::Partial,
            "Local STT, TTS, OCR, and LLM foundations exist, but the full always-on product is not complete.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Remote provider pooling with local-first auto-switching",
            7,
            false,
            false,
            FeatureStatus::Partial,
            "Policy and catalog layers exist, but the real provider/runtime orchestration is not fully connected yet.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Serializer plus long-context preparation",
            6,
            false,
            false,
            FeatureStatus::Partial,
            "Flow now has a serializer bridge and an RLM bridge, but they still need deeper runtime integration.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Local OCR, VLM, image, and video path",
            6,
            false,
            false,
            FeatureStatus::Partial,
            "OCR and runtime planning exist, but image/video/VLM execution paths are not complete.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Community conversion, validation, and publishing",
            4,
            false,
            false,
            FeatureStatus::Planned,
            "Flowpack, publish planning, and conversion planning exist, but the end-to-end pipeline is not complete.",
        ),
        feature(
            CompetitiveSegment::FlowNativeAdvantage,
            "Embeddable Rust crate for DX, editors, mobile, and browser hosts",
            6,
            false,
            false,
            FeatureStatus::Partial,
            "Embedding blueprints and the DxFlowRuntime facade exist, but FFI and WASM delivery layers still need work.",
        ),
    ];

    let segments = vec![
        compute_segment_score(&features, CompetitiveSegment::WisprFlowParity),
        compute_segment_score(&features, CompetitiveSegment::GrammarlyParity),
        compute_segment_score(&features, CompetitiveSegment::FlowNativeAdvantage),
    ];

    let overall_score_out_of_100 = score_from_features(&features);
    let wispr_replacement_score_out_of_100 = segments[0].score_out_of_100;
    let grammarly_replacement_score_out_of_100 = segments[1].score_out_of_100;
    let flow_native_advantage_score_out_of_100 = segments[2].score_out_of_100;

    CompetitiveScorecard {
        measured_on: "2026-04-26".to_string(),
        overall_score_out_of_100,
        wispr_replacement_score_out_of_100,
        grammarly_replacement_score_out_of_100,
        flow_native_advantage_score_out_of_100,
        segments,
        features,
        top_gaps: vec![
            "Finish true system-wide product polish across desktop and mobile hosts.".to_string(),
            "Ship real IDE variable recognition and live file tagging integrations.".to_string(),
            "Add plagiarism, citation, fact-checking, and academic-assistance layers.".to_string(),
            "Connect the provider layer to real local/remote auto-switching.".to_string(),
            "Finish multimodal local execution paths and conversion/publish pipelines.".to_string(),
        ],
    }
}

pub fn score_from_features(features: &[CompetitiveFeature]) -> u8 {
    let earned = features
        .iter()
        .map(|feature| feature.weight as f32 * feature.flow_status.multiplier())
        .sum::<f32>();
    let possible = features
        .iter()
        .map(|feature| feature.weight as f32)
        .sum::<f32>();
    percentage(earned, possible)
}

fn compute_segment_score(
    features: &[CompetitiveFeature],
    segment: CompetitiveSegment,
) -> SegmentScore {
    let filtered = features
        .iter()
        .filter(|feature| feature.segment == segment)
        .cloned()
        .collect::<Vec<_>>();
    let earned = filtered
        .iter()
        .map(|feature| feature.weight as f32 * feature.flow_status.multiplier())
        .sum::<f32>();
    let possible = filtered
        .iter()
        .map(|feature| feature.weight as f32)
        .sum::<f32>();

    SegmentScore {
        segment,
        earned,
        possible,
        score_out_of_100: percentage(earned, possible),
    }
}

fn percentage(earned: f32, possible: f32) -> u8 {
    if possible <= f32::EPSILON {
        0
    } else {
        ((earned / possible) * 100.0).round().clamp(0.0, 100.0) as u8
    }
}

fn feature(
    segment: CompetitiveSegment,
    name: &str,
    weight: u8,
    wispr_flow_has_it: bool,
    grammarly_has_it: bool,
    flow_status: FeatureStatus,
    notes: &str,
) -> CompetitiveFeature {
    CompetitiveFeature {
        segment,
        feature: name.to_string(),
        weight,
        wispr_flow_has_it,
        grammarly_has_it,
        flow_status,
        notes: notes.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scorecard_has_expected_current_scores() {
        let card = default_competitive_scorecard();
        assert_eq!(card.overall_score_out_of_100, 51);
        assert_eq!(card.wispr_replacement_score_out_of_100, 52);
        assert_eq!(card.grammarly_replacement_score_out_of_100, 40);
        assert_eq!(card.flow_native_advantage_score_out_of_100, 57);
    }
}
