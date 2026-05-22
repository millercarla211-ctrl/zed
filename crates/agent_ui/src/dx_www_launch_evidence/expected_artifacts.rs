#[derive(Clone, Copy)]
pub(super) struct ExpectedWwwEvidenceArtifact {
    pub(super) label: &'static str,
    pub(super) relative_path: &'static str,
    pub(super) command: &'static str,
    pub(super) format: EvidenceFormat,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EvidenceFormat {
    Json,
    Markdown,
}

pub(super) const EXPECTED_EVIDENCE_ARTIFACTS: &[ExpectedWwwEvidenceArtifact] = &[
    ExpectedWwwEvidenceArtifact {
        label: "Readiness Bundle",
        relative_path: ".dx/forge/template-readiness/launch-readiness-bundle.json",
        command: "dx forge launch-readiness-bundle --project . --json --output .dx/forge/template-readiness/launch-readiness-bundle.json",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Release Packet",
        relative_path: ".dx/forge/release/launch-evidence-packet.json",
        command: "dx forge launch-evidence-packet --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Operator Index",
        relative_path: ".dx/forge/release/launch-evidence-operator-index.json",
        command: "dx forge launch-evidence-operator-index --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Status Timeline",
        relative_path: ".dx/forge/release/launch-evidence-status-timeline.json",
        command: "dx forge launch-evidence-status-timeline --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Handoff Digest",
        relative_path: ".dx/forge/release/launch-evidence-handoff-digest.md",
        command: "dx forge launch-evidence-handoff-digest --project . --write",
        format: EvidenceFormat::Markdown,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Release Checklist",
        relative_path: ".dx/forge/release/launch-evidence-release-checklist.json",
        command: "dx forge launch-evidence-release-checklist --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Share Manifest",
        relative_path: ".dx/forge/release/launch-evidence-share-manifest.json",
        command: "dx forge launch-evidence-share-manifest --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Archive Ledger",
        relative_path: ".dx/forge/release/launch-evidence-archive-ledger.json",
        command: "dx forge launch-evidence-archive-ledger --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Retention Review",
        relative_path: ".dx/forge/release/launch-evidence-retention-review.json",
        command: "dx forge launch-evidence-retention-review --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Release Seal",
        relative_path: ".dx/forge/release/launch-evidence-release-seal.json",
        command: "dx forge launch-evidence-release-seal --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Operator Summary",
        relative_path: ".dx/forge/release/launch-evidence-operator-summary.json",
        command: "dx forge launch-evidence-operator-summary --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Completion Ledger",
        relative_path: ".dx/forge/release/launch-evidence-completion-ledger.json",
        command: "dx forge launch-evidence-completion-ledger --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Closure Memo",
        relative_path: ".dx/forge/release/launch-evidence-closure-memo.md",
        command: "dx forge launch-evidence-closure-memo --project . --write",
        format: EvidenceFormat::Markdown,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Final Brief",
        relative_path: ".dx/forge/release/launch-evidence-final-brief.json",
        command: "dx forge launch-evidence-final-brief --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Operator Runbook",
        relative_path: ".dx/forge/release/launch-evidence-operator-runbook.json",
        command: "dx forge launch-evidence-operator-runbook --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Handoff Capsule",
        relative_path: ".dx/forge/release/launch-evidence-handoff-capsule.json",
        command: "dx forge launch-evidence-handoff-capsule --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Resumption Index",
        relative_path: ".dx/forge/release/launch-evidence-resumption-index.json",
        command: "dx forge launch-evidence-resumption-index --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Recovery Brief",
        relative_path: ".dx/forge/release/launch-evidence-recovery-brief.md",
        command: "dx forge launch-evidence-recovery-brief --project . --write",
        format: EvidenceFormat::Markdown,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Continuation Packet",
        relative_path: ".dx/forge/release/launch-evidence-continuation-packet.json",
        command: "dx forge launch-evidence-continuation-packet --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Operator Resume",
        relative_path: ".dx/forge/release/launch-evidence-operator-resume-card.json",
        command: "dx forge launch-evidence-operator-resume-card --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Ledger",
        relative_path: ".dx/forge/release/launch-evidence-restart-ledger.json",
        command: "dx forge launch-evidence-restart-ledger --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Checklist",
        relative_path: ".dx/forge/release/launch-evidence-restart-checklist.json",
        command: "dx forge launch-evidence-restart-checklist --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Brief",
        relative_path: ".dx/forge/release/launch-evidence-restart-brief.md",
        command: "dx forge launch-evidence-restart-brief --project . --write",
        format: EvidenceFormat::Markdown,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Manifest",
        relative_path: ".dx/forge/release/launch-evidence-restart-manifest.json",
        command: "dx forge launch-evidence-restart-manifest --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Receipt",
        relative_path: ".dx/forge/release/launch-evidence-restart-receipt.json",
        command: "dx forge launch-evidence-restart-receipt --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Summary",
        relative_path: ".dx/forge/release/launch-evidence-restart-summary.json",
        command: "dx forge launch-evidence-restart-summary --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Snapshot",
        relative_path: ".dx/forge/release/launch-evidence-restart-snapshot.json",
        command: "dx forge launch-evidence-restart-snapshot --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Dispatch",
        relative_path: ".dx/forge/release/launch-evidence-restart-dispatch.json",
        command: "dx forge launch-evidence-restart-dispatch --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Closeout",
        relative_path: ".dx/forge/release/launch-evidence-restart-closeout.md",
        command: "dx forge launch-evidence-restart-closeout --project . --write",
        format: EvidenceFormat::Markdown,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Restart Signoff",
        relative_path: ".dx/forge/release/launch-evidence-restart-signoff.json",
        command: "dx forge launch-evidence-restart-signoff --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Acceptance Index",
        relative_path: ".dx/forge/release/launch-evidence-acceptance-index.md",
        command: "dx forge launch-evidence-acceptance-index --project . --write",
        format: EvidenceFormat::Markdown,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Acceptance Digest",
        relative_path: ".dx/forge/release/launch-evidence-acceptance-digest.json",
        command: "dx forge launch-evidence-acceptance-digest --project . --write",
        format: EvidenceFormat::Json,
    },
    ExpectedWwwEvidenceArtifact {
        label: "Friday Baton",
        relative_path: ".dx/forge/release/launch-evidence-friday-baton.md",
        command: "dx forge launch-evidence-friday-baton --project . --write",
        format: EvidenceFormat::Markdown,
    },
];
