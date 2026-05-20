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
        overall_implementation_status: 70,
        planning_status: 100,
        browser_chrome_hardening_status: 99,
        dx_catalog_status: 99,
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
                99,
                "Archived provider/model structs, artifact header, memmap read path, generator merge/dedupe, validation, last-good fallback, source adapters, source discovery, local GGUF model reader, provider/auth reader, models.dev/OpenRouter/LiteLLM JSON model parsers, Agent picker projection, route selection, OpenRouter input, auth-profile enrichment, optional generated-artifact Agent picker enrichment, catalog alias resolution, route IDs that resolve to executable Agent models, permissioned catalog execution plans, provider adapter registration specs, and production artifact materialization from discovered G-drive sources are in place.",
                "Expose a small approved command/settings trigger that writes the catalog artifact to the Agent-discoverable path.",
            ),
            feature(
                "Universal provider routing",
                50,
                "One router picks local, free, premium, and remote providers from dx_catalog.",
                "Wire registration specs into provider settings after explicit approval.",
            ),
            feature(
                "Metasearch AI tool",
                10,
                "Agent panel can call cancellable multi-engine cited search.",
                "Add the metasearch adapter and compact result card contract.",
            ),
            feature(
                "Serializer/RLM context pipeline",
                10,
                "Tool catalogs, source packs, and search results compact before model calls.",
                "Define the serializer/RLM boundary for AI panel calls.",
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
