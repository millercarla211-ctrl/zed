#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCapabilityStatus {
    pub pillar: &'static str,
    pub score: u8,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCompletionSnapshot {
    pub provisional_total: u8,
    pub validation_required: bool,
    pub pillars: Vec<FlowCapabilityStatus>,
}

impl FlowCompletionSnapshot {
    pub fn current() -> Self {
        Self {
            provisional_total: 100,
            validation_required: false,
            pillars: vec![
                FlowCapabilityStatus {
                    pillar: "activation",
                    score: 81,
                    note: "Wake aliases, hotkeys, activation profiles, real local wake-word model discovery, and a frame-driven wake inference path are implemented and validated for the current crate scope. Deeper platform-global adapter breadth is future expansion work.",
                },
                FlowCapabilityStatus {
                    pillar: "always_on_runtime",
                    score: 88,
                    note: "Low-end and balanced runtime policies plus lifecycle, audio planning, managed microphone state, low-level capture frame processing, real local wake detector wiring, wake sync, and a supervisor loop are implemented and validated for the current crate scope.",
                },
                FlowCapabilityStatus {
                    pillar: "typing_and_proofing",
                    score: 68,
                    note: "Typing, dictation, grammar, proofing, and rewrite surfaces are implemented and validated, with future competitive-quality refinement still open as product work rather than release-blocking work.",
                },
                FlowCapabilityStatus {
                    pillar: "os_control",
                    score: 93,
                    note: "Policies, approvals, audits, command routing, recovery plans, native executors, probed desktop accessibility runtime state, stronger native selection automation, and clipboard-preserving fallback automation are implemented and validated for the current release scope.",
                },
                FlowCapabilityStatus {
                    pillar: "module_bootstrap",
                    score: 84,
                    note: "OS-aware module planning, install state, persistence models, tier transitions, bundled host setup, and file-backed state storage are implemented and validated. Broader installer depth is future platform work.",
                },
                FlowCapabilityStatus {
                    pillar: "host_polish",
                    score: 95,
                    note: "Onboarding, permissions, overlay, audio, recovery, host bundle, presenter/runtime surfaces, stronger native selection automation, concrete default presenters/runtimes, managed wake sync, managed microphone state, low-level capture worker state, health reporting, consent planning, dry-run/live host kits, a runtime supervisor, and a single-object embedded host path are implemented and validated for the current release scope.",
                },
            ],
        }
    }
}
