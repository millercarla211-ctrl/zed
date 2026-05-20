use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct DxLaunchStatus {
    pub overall_implementation_status: u8,
    pub planning_status: u8,
    pub browser_chrome_hardening_status: u8,
    pub dx_catalog_status: u8,
    pub features: Vec<LaunchFeatureStatus>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct LaunchFeatureStatus {
    pub feature: String,
    pub status: u8,
    pub target: String,
    pub next_action: String,
}

pub fn current_launch_status() -> DxLaunchStatus {
    DxLaunchStatus {
        overall_implementation_status: 95,
        planning_status: 100,
        browser_chrome_hardening_status: 99,
        dx_catalog_status: 100,
        features: vec![
            feature(
                "Browser/Chrome functional plugin execution",
                99,
                "Live UI exercise and final runtime proof are the remaining gates.",
                "Run the final live proof when build headroom is available.",
            ),
            feature(
                "Screen Dock Carousel",
                85,
                "Full-width screen switching is visible and usable; persistence and spring polish remain.",
                "Add reduced-motion-safe spring polish after catalog/routing foundations.",
            ),
            feature(
                "dx_catalog provider/model archive",
                100,
                "Archived provider/model structs, artifact header, memmap read path, generator merge/dedupe, validation, last-good fallback, source adapters, source discovery, local GGUF model reader, provider/auth reader, models.dev/OpenRouter/LiteLLM JSON model parsers, Agent picker projection, route selection, OpenRouter input, auth-profile enrichment, optional generated-artifact Agent picker enrichment, catalog alias resolution, route IDs that resolve to executable Agent models, permissioned catalog execution plans, provider adapter registration specs, production artifact materialization from discovered G-drive sources, and an explicit Agent-approved artifact generation trigger are in place.",
                "Keep the catalog stable while provider settings approvals and metasearch are wired.",
            ),
            feature(
                "Universal provider routing",
                76,
                "One router picks local, free, premium, and remote providers from dx_catalog; approved provider settings registration can write catalog specs into native Zed language-model settings; the Agent preview validates native settings, runtime registry registration, credential/auth state, and matching model exposure; and an explicit permissioned Agent tool can queue approved native settings registration.",
                "Continue serializer/RLM execution integration and cross-panel routing.",
            ),
            feature(
                "Metasearch AI tool",
                74,
                "Agent panel can call cancellable multi-engine cited search through permissioned DX metasearch tools, inspect service/engine readiness, emit compact citations, return token-aware cited source packs, persist managed source-pack receipts, fetch bounded readable extracts, prepare compact context bundles, and create approved serializer/RLM execution-plan receipts.",
                "Connect context bundles and execution-plan receipts into panel surfaces.",
            ),
            feature(
                "Serializer/RLM context pipeline",
                44,
                "Metasearch source packs and deep extracts can be compacted into citation-preserving context bundles, discover serializer/RLM roots, and produce execution-plan approval receipts without running external reducers.",
                "Add the actual approved external reducer runner after safety review.",
            ),
            feature(
                "Forge safety and backup policy",
                10,
                "Risky file operations create reversible Forge/zstd receipts instead of permanent loss.",
                "Define the no-permanent-delete command receipt contract.",
            ),
            feature(
                "Forge panel",
                5,
                "Snapshots, remotes, jobs, restore points, and media-aware status are visible.",
                "Add a read-only host adapter and panel skeleton.",
            ),
            feature(
                "Drive/Sources rail",
                5,
                "NotebookLM-style source sets feed agents with hashes and summaries.",
                "Define source-set records and left-rail state.",
            ),
            feature(
                "Check panel",
                5,
                "Project score imports formatting, lint, structure, visual proof, and deploy readiness.",
                "Define score schema and read-only scanner handoff.",
            ),
            feature(
                "Deploy panel",
                0,
                "CI/CD, env readiness, preview URLs, production status, logs, rollback, and receipts are visible.",
                "Define deploy target registry.",
            ),
            feature(
                "DCP bridge",
                0,
                "DCP, MCP, ACP, and local tools share one permission and receipt model.",
                "Define the minimum capability schema.",
            ),
        ],
    }
}

fn feature(
    feature: impl Into<String>,
    status: u8,
    target: impl Into<String>,
    next_action: impl Into<String>,
) -> LaunchFeatureStatus {
    LaunchFeatureStatus {
        feature: feature.into(),
        status,
        target: target.into(),
        next_action: next_action.into(),
    }
}
