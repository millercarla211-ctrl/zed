use ui::IconName;

use crate::dx_deploy_prompts::{deploy_launch_gate_prompt, deploy_receipt_bucket_prompt};
use crate::dx_deploy_targets::DxDeployTargetSnapshot;
use crate::dx_launch_audit::DxLaunchAuditSnapshot;
use crate::dx_launch_contracts::DxLaunchContractSnapshot;
use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;
use crate::dx_launch_receipts::DxLaunchReceiptReviewSnapshot;
use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;
use crate::dx_launch_status::DxLaunchStatusSnapshot;
use crate::dx_proof_freshness::DxProofFreshnessSnapshot;
use crate::dx_receipt_history::{DxToolHistoryReceiptSummary, DxToolHistorySnapshot};
use crate::dx_receipts::DxReceiptSnapshot;
use crate::dx_source_sets::{DxSourceItem, DxSourceKind};
use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

mod runtime_proof;

pub(crate) use runtime_proof::{
    runtime_proof_evidence_template_prompt, runtime_proof_import_prompt, runtime_proof_prompt,
};

pub(crate) fn source_action_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack | DxSourceKind::ReducedContextReceipt => {
            IconName::FileTextOutlined
        }
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
    }
}

pub(crate) fn source_action_title(source: &DxSourceItem) -> String {
    match source.kind {
        DxSourceKind::WorkspaceRoot => format!("Attach {}", source.label),
        DxSourceKind::MetasearchSourcePack => "Attach Search Pack".to_string(),
        DxSourceKind::ReducedContextReceipt => "Review Reduced Context".to_string(),
        DxSourceKind::MediaOutput => "Attach Media Output".to_string(),
        DxSourceKind::ForgeRestorePreview => "Review Restore Preview".to_string(),
    }
}

pub(crate) fn source_action_label(kind: DxSourceKind) -> &'static str {
    match kind {
        DxSourceKind::WorkspaceRoot
        | DxSourceKind::MetasearchSourcePack
        | DxSourceKind::MediaOutput => "Attach",
        DxSourceKind::ReducedContextReceipt | DxSourceKind::ForgeRestorePreview => "Review",
    }
}

pub(crate) fn source_receipt_review_prompt(source: &DxSourceItem) -> String {
    let receipts = source
        .receipt_drilldowns
        .iter()
        .map(|receipt| format!("{}: {}", receipt.label, receipt.detail))
        .collect::<Vec<_>>()
        .join("; ");
    let receipts = if receipts.is_empty() {
        "No managed receipt drilldowns are visible for this source yet.".to_string()
    } else {
        format!("Visible receipt drilldowns: {receipts}.")
    };

    format!(
        "Review the DX source receipt metadata for `{label}` at `{path}`. {receipts} Summarize the receipt type, source kind, proof rows, warning rows, freshness risk, and the next safe Agent action. Do not run builds, local servers, browser input, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions.",
        label = source.label.as_str(),
        path = source.path.as_str(),
    )
}

pub(crate) fn source_action_prompt(source: &DxSourceItem) -> String {
    match source.kind {
        DxSourceKind::WorkspaceRoot => format!(
            "Prepare a DX source attachment for workspace root `{}`. Use prepare_dx_source_attachment only, write a managed receipt if appropriate, and do not run builds, local servers, browser input, shell commands, external serializer/RLM code, deploys, or restore-to-target actions.",
            source.path
        ),
        DxSourceKind::MetasearchSourcePack => format!(
            "Prepare this metasearch source-pack receipt for the DX context flow: `{}`. Use prepare_dx_source_attachment, then prepare_dx_metasearch_context if the attachment is valid. Preserve citations and stop before any external serializer/RLM runner or model-call execution unless I explicitly approve it.",
            source.path
        ),
        DxSourceKind::ReducedContextReceipt => format!(
            "Review this reduced-context receipt for the DX launch flow: `{}`. Summarize the selected sources, token budget, reducer status, citation coverage, runner-gate readiness, model-call approval state, and missing proof steps. Draft the serializer/RLM execution guard only; do not run external serializer/RLM code or model calls.",
            source.path
        ),
        DxSourceKind::MediaOutput => {
            let proof_summary = if source.proofs.is_empty() {
                "No produced-file proof summary is visible yet.".to_string()
            } else {
                format!(
                    "Visible produced-file proofs: {}.",
                    source.proofs.join("; ")
                )
            };
            format!(
                "Prepare this produced media output as a DX source attachment: `{}`. {proof_summary} Use prepare_dx_source_attachment only, keep binary payloads path-only, and report the next safe media proof step without running ffmpeg, shell commands, local servers, or browser input.",
                source.path
            )
        }
        DxSourceKind::ForgeRestorePreview => format!(
            "Review this Forge restore preview source: `{}`. Use inspect_dx_forge_history and prepare_dx_source_attachment as needed, summarize restore warnings, target path, overwrite risk, rollback evidence, visible restore_approval entries, and required restore-to-target approvals. Draft the approval checklist only; do not mutate target paths, overwrite files, delete files, or run restore-to-target actions.",
            source.path
        ),
    }
}

pub(crate) fn forge_proof_prompt(tool_history: &DxToolHistorySnapshot) -> String {
    let forge_context = forge_history_prompt_context(tool_history);

    format!(
        "Prepare the DX Forge proof flow for this workspace. Current Forge history context: {forge_context}. First call list_dx_launch_demo_recipes with focus=\"forge\" and inspect_dx_forge_history. Then guide me through the next safe receipt step for safety policy, backup runner gate, backup execution, restore preview, restore receipt review, restore-approval capture, and restore-target dry-run planning. Do not mutate target paths, permanently delete files, run local servers, builds, shell commands, browser input, or restore-to-target actions unless I explicitly approve the governed tool request."
    )
}

pub(crate) fn restore_approval_prompt(tool_history: &DxToolHistorySnapshot) -> String {
    let forge_context = forge_history_prompt_context(tool_history);

    format!(
        "Prepare a non-mutating DX Forge restore-to-target approval review for this workspace. Current Forge history context: {forge_context}. Use inspect_dx_forge_history and visible restore-preview source rows to summarize the latest safety-policy, backup, backup-manifest, restore-preview, restore-approval, restore-target plan, blockers, target path, overwrite risk, rollback evidence, and missing confirmations. If I provide operator approval evidence, use capture_dx_forge_restore_approval to write only a managed approval receipt, then use plan_dx_forge_restore_target to write only a dry-run plan receipt when approval and rollback evidence are ready, then use inspect_dx_forge_history to confirm restore_approval and restore_target_plan entries are visible. Do not mutate target paths, overwrite files, delete files, run shell commands, run local servers, or execute restore-to-target actions."
    )
}

pub(crate) fn launch_handoff_prompt(
    contracts: &DxLaunchContractSnapshot,
    readiness: &DxLaunchReadinessSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
) -> String {
    let contract_context = launch_contract_prompt_context(contracts);
    let readiness_context = launch_readiness_prompt_context(readiness);
    let launch_context = launch_status_prompt_context(launch_status);
    let receipt_context = launch_receipt_review_prompt_context(launch_receipts);

    format!(
        "Review the DX launch handoff for this Zed workspace. Launch contract metadata: {contract_context}. Launch gate readiness: {readiness_context}. Launch aggregate: {launch_context}. Launch receipt diagnostics: {receipt_context}. Use the visible source-owned import-manifest, handoff, import-summary, release-gate, and fallback-drill metadata to summarize packet coverage, polling order, diagnostics commands, action-map safety, cached receipt fallback states, command fanout, redaction posture, and missing proof. If the operator asks for a refresh, draft the exact `dx launch import-manifest --json`, `dx launch handoff --json`, `dx launch import-summary --json`, `dx launch release-gate --json`, `dx launch fallback-drill --json`, `dx launch status --json`, or `dx launch receipts --json` step, but do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_readiness_prompt(
    readiness: &DxLaunchReadinessSnapshot,
    contracts: &DxLaunchContractSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
) -> String {
    let readiness_context = launch_readiness_prompt_context(readiness);
    let contract_context = launch_contract_prompt_context(contracts);
    let launch_context = launch_status_prompt_context(launch_status);
    let receipt_context = launch_receipt_review_prompt_context(launch_receipts);

    format!(
        "Review the DX launch import gate for this Zed workspace. Launch gate readiness: {readiness_context}. Launch contract metadata: {contract_context}. Launch aggregate: {launch_context}. Launch receipt diagnostics: {receipt_context}. Summarize whether Zed can safely render the import-summary, release-gate, and fallback-drill states, which cached receipt states are represented, what recovery commands should be shown, and what governed runtime proof is still missing. Do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_audit_prompt(
    audit: &DxLaunchAuditSnapshot,
    readiness: &DxLaunchReadinessSnapshot,
    contracts: &DxLaunchContractSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
) -> String {
    let audit_context = launch_audit_prompt_context(audit);
    let readiness_context = launch_readiness_prompt_context(readiness);
    let contract_context = launch_contract_prompt_context(contracts);
    let launch_context = launch_status_prompt_context(launch_status);

    format!(
        "Review the DX launch CLI audit for this Zed workspace. Launch audit: {audit_context}. Launch gate readiness: {readiness_context}. Launch handoff contracts: {contract_context}. Launch aggregate: {launch_context}. Summarize command schema coverage, startup polling commands, fixture render states, smoke checks, write/fanout risk, redaction posture, and the next safe operator command. Do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_www_evidence_prompt(snapshot: &DxWwwLaunchEvidenceSnapshot) -> String {
    let www_context = launch_www_evidence_prompt_context(snapshot);

    format!(
        "Review the DX-WWW launch evidence handoff for this Zed workspace. WWW evidence: {www_context}. Summarize the release packet, operator index, timeline, handoff digest, release seal, restart handoff, acceptance artifacts, missing commands, and whether the visible evidence is safe to treat as no-execution handoff metadata. If artifacts are missing, draft the exact DX-WWW operator command sequence from the visible next commands and stop. Do not run DX-WWW, Forge, CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_source_audit_prompt(snapshot: &DxLaunchSourceAuditSnapshot) -> String {
    let source_context = launch_source_audit_prompt_context(snapshot);

    format!(
        "Review the DX launch source audit for this Zed workspace. Source audit: {source_context}. Summarize the hub coordination verdict, worker-output ledger, source-clean repos, risk-review blockers, template trust scan, DX Studio WWW QA status, latest deltas, and the next safe Friday action. Do not touch G:\\Dx\\www package work, run builds, run local servers, run browser automation, execute CLI commands, deploy, mutate other repos, import secrets, call providers, or restore-to-target actions."
    )
}

pub(crate) fn receipt_review_prompt(
    receipt_snapshot: &DxReceiptSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
    launch_contracts: &DxLaunchContractSnapshot,
    launch_readiness: &DxLaunchReadinessSnapshot,
    launch_audit: &DxLaunchAuditSnapshot,
    source_audit: &DxLaunchSourceAuditSnapshot,
    www_evidence: &DxWwwLaunchEvidenceSnapshot,
    tool_history: &DxToolHistorySnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
) -> String {
    let receipt_root = if receipt_snapshot.root_exists {
        format!(
            "DX receipt root present at `{}`",
            receipt_snapshot.root.display()
        )
    } else {
        format!(
            "DX receipt root missing at `{}`",
            receipt_snapshot.root.display()
        )
    };
    let receipt_buckets = receipt_snapshot
        .buckets
        .iter()
        .map(|bucket| format!("{}={}", bucket.label, bucket.count))
        .collect::<Vec<_>>()
        .join(", ");
    let receipt_buckets = if receipt_buckets.is_empty() {
        "No DX receipt buckets are tracked yet.".to_string()
    } else {
        receipt_buckets
    };
    let latest_receipts = bounded_join(&receipt_snapshot.latest, 4, "No latest DX receipts");
    let launch_context = launch_status_prompt_context(launch_status);
    let launch_receipt_context = launch_receipt_review_prompt_context(launch_receipts);
    let launch_contract_context = launch_contract_prompt_context(launch_contracts);
    let launch_readiness_context = launch_readiness_prompt_context(launch_readiness);
    let launch_audit_context = launch_audit_prompt_context(launch_audit);
    let source_audit_context = launch_source_audit_prompt_context(source_audit);
    let www_context = launch_www_evidence_prompt_context(www_evidence);
    let tool_buckets = tool_history
        .buckets
        .iter()
        .map(|bucket| {
            format!(
                "{}={} ({})",
                bucket.label,
                bucket.count,
                if bucket.root_exists {
                    "root present"
                } else {
                    "missing root"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let tool_buckets = if tool_buckets.is_empty() {
        "No tool-history buckets are tracked yet.".to_string()
    } else {
        tool_buckets
    };
    let forge_history = forge_history_prompt_context(tool_history);
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| format!("{}={} ({})", bucket.label, bucket.count, bucket.status))
        .collect::<Vec<_>>()
        .join(", ");
    let proof_rows = if proof_rows.is_empty() {
        "No proof freshness buckets are tracked yet.".to_string()
    } else {
        proof_rows
    };
    let deploy_rows = deploy_targets
        .receipt_buckets
        .iter()
        .map(deploy_receipt_bucket_prompt)
        .collect::<Vec<_>>()
        .join(", ");
    let deploy_rows = if deploy_rows.is_empty() {
        "No deploy receipt buckets are tracked yet.".to_string()
    } else {
        deploy_rows
    };
    let deploy_launch_gate = deploy_launch_gate_prompt(deploy_targets);

    format!(
        "Inspect the current DX launch receipts for this workspace. {receipt_root}. Receipt buckets: {receipt_buckets}. Latest receipts: {latest_receipts}. Launch aggregate: {launch_context}. Launch handoff contracts: {launch_contract_context}. Launch gate readiness: {launch_readiness_context}. Launch CLI audit: {launch_audit_context}. Source audit: {source_audit_context}. DX-WWW evidence: {www_context}. Launch receipt diagnostics: {launch_receipt_context}. Tool history buckets: {tool_buckets}. Forge history context: {forge_history}. Proof freshness buckets: {proof_rows}. Deploy receipt buckets: {deploy_rows}. Deploy launch gate: {deploy_launch_gate}. Summarize the latest launch status, launch receipt freshness, malformed retained snapshots, handoff packet coverage, schemas/fixtures/smoke/status audit state, source coordination verdict, DX-WWW release/restart/acceptance evidence, import-summary/release-gate/fallback-drill parser states, metasearch, source attachment, serializer/RLM context, execution, runner-gate, reduced-context, execution-preview, external-execution, media, Forge, restore-approval, restore-target plan, runtime-proof plan/import/status, and deploy receipts. Report missing receipt roots gracefully and give the next safe action without running builds, local servers, browser input, external serializer/RLM code, restore-to-target actions, deploys, shell commands, or model calls."
    )
}

fn launch_status_prompt_context(snapshot: &DxLaunchStatusSnapshot) -> String {
    if !snapshot.root_exists {
        return format!(
            "missing launch receipt root `{}`; run dx launch status --json when the CLI lane is ready",
            snapshot.root.display()
        );
    }

    if !snapshot.latest_present {
        return format!(
            "no latest launch status receipt at `{}`; expected schema dx.launch.status.v1",
            snapshot.latest_path.display()
        );
    }

    if !snapshot.schema_valid {
        return format!(
            "invalid latest launch status receipt `{}`: {}",
            snapshot.latest_path.display(),
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "schema validation failed".to_string())
        );
    }

    format!(
        "status={}, summary={}, agents={} connected of {} configured ({}) tokens_status={} budget={} estimated={} soft={} hard={} discovery={} templates_command={} packages_command={} next_action={}",
        snapshot.status,
        snapshot.operator_summary,
        snapshot.agents.connected_accounts,
        snapshot.agents.configured_accounts,
        snapshot.agents.status,
        snapshot.tokens.status,
        snapshot.tokens.budget_state,
        snapshot.tokens.estimated_tokens,
        snapshot.tokens.soft_budget_tokens,
        snapshot.tokens.hard_budget_tokens,
        snapshot.discovery.status,
        snapshot.discovery.templates_command,
        snapshot.discovery.packages_command,
        snapshot.next_action
    )
}

fn launch_contract_prompt_context(snapshot: &DxLaunchContractSnapshot) -> String {
    if !snapshot.manifest_present {
        return format!(
            "missing import manifest `{}`; expected dx.launch.import_manifest.v1",
            snapshot.manifest_path.display()
        );
    }

    if !snapshot.handoff_present {
        return format!(
            "missing handoff packet `{}`; expected dx.launch.handoff.v1",
            snapshot.handoff_path.display()
        );
    }

    let startup = bounded_join(&snapshot.startup_commands, 5, "No startup commands");
    let diagnostics = bounded_join(&snapshot.diagnostics_commands, 5, "No diagnostics commands");
    let first_packets = bounded_join(&snapshot.first_packets, 5, "No packet commands");
    let refresh = snapshot
        .refresh_command
        .as_deref()
        .unwrap_or("dx launch status --json");
    let cached = snapshot
        .cached_receipt_path
        .as_deref()
        .unwrap_or(".dx/receipts/launch/status-latest.json");

    format!(
        "status={} summary={} packets={} fixture_families={} commands={} actions={} metadata_only={} fanout={} confirmations={} no_command_fanout={} redaction_review={} refresh={} cached={} startup=[{}] diagnostics=[{}] first_packets=[{}] next_action={}",
        snapshot.status,
        snapshot.operator_summary,
        snapshot.packet_count,
        snapshot.fixture_family_count,
        snapshot.command_count,
        snapshot.action_count,
        snapshot.metadata_only_count,
        snapshot.command_fanout_count,
        snapshot.confirmation_action_count,
        snapshot.no_command_fanout,
        snapshot.redaction_requires_review,
        refresh,
        cached,
        startup,
        diagnostics,
        first_packets,
        snapshot.next_action
    )
}

fn launch_readiness_prompt_context(snapshot: &DxLaunchReadinessSnapshot) -> String {
    if !snapshot.root_exists {
        return format!(
            "missing launch example root `{}`; expected source-owned import-summary, release-gate, and fallback-drill packets",
            snapshot.root.display()
        );
    }

    let freshness = bounded_join(&snapshot.freshness_states, 5, "No cached freshness states");
    let fallback_states = bounded_join(&snapshot.fallback_states, 5, "No fallback states");
    let recovery = bounded_join(&snapshot.recovery_commands, 5, "No recovery commands");
    let examples = snapshot
        .examples
        .iter()
        .take(4)
        .map(|example| format!("{}={} ({})", example.label, example.status, example.detail))
        .collect::<Vec<_>>();
    let examples = bounded_join(&examples, 4, "No source-owned launch readiness examples");

    format!(
        "status={} summary={} import_packets={} [{}] release_gate_packets={} [{}] fallback_packets={} [{}] gate_rows={}/{} passed warning={} failed={} fallback_state_count={} freshness=[{}] fallback_states=[{}] no_command_fanout={} fanout={} redaction_review={} recovery=[{}] examples=[{}] next_action={}",
        snapshot.status,
        snapshot.operator_summary,
        snapshot.import_summary_count,
        snapshot.import_status_counts.summary(),
        snapshot.release_gate_count,
        snapshot.release_gate_status_counts.summary(),
        snapshot.fallback_drill_count,
        snapshot.fallback_status_counts.summary(),
        snapshot.passed_count,
        snapshot.acceptance_count,
        snapshot.warning_count,
        snapshot.failed_count,
        snapshot.fallback_state_count,
        freshness,
        fallback_states,
        snapshot.no_command_fanout,
        snapshot.command_fanout_count,
        snapshot.redaction_requires_review,
        recovery,
        examples,
        snapshot.next_action
    )
}

fn launch_audit_prompt_context(snapshot: &DxLaunchAuditSnapshot) -> String {
    if !snapshot.root_exists {
        return format!(
            "missing launch example root `{}`; expected source-owned schemas, fixtures, smoke, and status packets",
            snapshot.root.display()
        );
    }

    let commands = bounded_join(&snapshot.command_rows, 5, "No command rows");
    let fixtures = bounded_join(&snapshot.fixture_rows, 3, "No fixture rows");
    let smoke = bounded_join(&snapshot.smoke_rows, 3, "No smoke rows");

    format!(
        "status={} summary={} commands={} metadata_only={} startup_poll={} user_action={} writes={} fixtures={}/{} smoke={}/{} passed warning={} failed={} example_status={} agents={} tokens={} discovery={} fanout={} redaction_review={} commands=[{}] fixtures=[{}] smoke_rows=[{}] next_action={}",
        snapshot.status,
        snapshot.operator_summary,
        snapshot.command_count,
        snapshot.metadata_only_count,
        snapshot.startup_poll_count,
        snapshot.user_action_count,
        snapshot.write_path_count,
        snapshot.fixture_match_count,
        snapshot.fixture_count,
        snapshot.smoke_passed_count,
        snapshot.smoke_check_count,
        snapshot.smoke_warning_count,
        snapshot.smoke_failed_count,
        snapshot.example_status,
        snapshot.example_agents,
        snapshot.example_tokens,
        snapshot.example_discovery,
        snapshot.command_fanout_count,
        snapshot.redaction_requires_review,
        commands,
        fixtures,
        smoke,
        snapshot.next_action
    )
}

fn launch_www_evidence_prompt_context(snapshot: &DxWwwLaunchEvidenceSnapshot) -> String {
    if !snapshot.project_root_exists {
        return format!(
            "missing DX-WWW project root `{}`; expected a generated DX WWW workspace or `{}` fallback",
            snapshot.project_root.display(),
            "G:\\WWW\\www"
        );
    }

    let latest = bounded_join(
        &snapshot.latest_rows,
        4,
        "No generated launch evidence artifacts",
    );
    let missing = bounded_join(
        &snapshot.missing_rows,
        5,
        "No missing launch evidence artifacts",
    );
    let next_commands = bounded_join(&snapshot.next_commands, 5, "No next DX-WWW command");

    format!(
        "status={} summary={} project={} release_root={} release_root_present={} artifacts={}/{} json={} markdown={} ready={} warning={} blocked={} no_execution={} latest=[{}] missing=[{}] next_commands=[{}]",
        snapshot.status,
        snapshot.operator_summary,
        snapshot.project_root.display(),
        snapshot.release_root.display(),
        snapshot.release_root_exists,
        snapshot.present_count,
        snapshot.expected_count,
        snapshot.json_count,
        snapshot.markdown_count,
        snapshot.passed_count,
        snapshot.warning_count,
        snapshot.blocked_count,
        snapshot.no_execution_count,
        latest,
        missing,
        next_commands,
    )
}

fn launch_source_audit_prompt_context(snapshot: &DxLaunchSourceAuditSnapshot) -> String {
    if !snapshot.root_exists {
        return format!(
            "missing source audit root `{}`; expected G:\\Dx\\.dx\\audit\\launch-source\\latest.json",
            snapshot.root.display()
        );
    }

    if !snapshot.latest_present {
        return format!(
            "missing source audit latest receipt `{}`; rerun the G:\\Dx launch source audit helper when the hub lane is ready",
            snapshot.latest_path.display()
        );
    }

    if !snapshot.schema_valid {
        return format!(
            "invalid source audit receipt `{}`: {}",
            snapshot.latest_path.display(),
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "schema validation failed".to_string())
        );
    }

    let repos = bounded_join(&snapshot.repo_rows, 4, "No repository readiness rows");
    let blockers = bounded_join(&snapshot.blocker_rows, 4, "No source audit blockers");
    let deltas = bounded_join(&snapshot.delta_rows, 3, "No worker-output deltas");

    format!(
        "status={} score={}/100 passed={} generated={} mode={} ready_for_commit_coordination={} repos={} active_output={} source_clean={} risk_review={} owner_review={} diff_failures={} dx_studio={}/100 passed={} checks={}/{} template_trust={} template_roots={}/{} template_node_modules={} repos=[{}] blockers=[{}] deltas=[{}] next_target={}",
        snapshot.status,
        snapshot.score,
        snapshot.passed,
        snapshot.generated_at,
        snapshot.mode,
        snapshot.ready_for_commit_coordination,
        snapshot.repo_count,
        snapshot.active_output_count,
        snapshot.source_clean_count,
        snapshot.risk_review_count,
        snapshot.owner_review_count,
        snapshot.diff_check_failure_count,
        snapshot.dx_studio_score,
        snapshot.dx_studio_passed,
        snapshot.dx_studio_passed_checks,
        snapshot.dx_studio_total_checks,
        snapshot.template_trust_passed,
        snapshot.template_roots_scanned,
        snapshot.template_roots_total,
        snapshot.template_node_modules_found,
        repos,
        blockers,
        deltas,
        snapshot.next_target
    )
}

fn launch_receipt_review_prompt_context(snapshot: &DxLaunchReceiptReviewSnapshot) -> String {
    if !snapshot.root_exists {
        return format!(
            "missing launch receipt directory `{}`; run dx launch status --json when the CLI lane is ready",
            snapshot.root.display()
        );
    }

    if !snapshot.latest_present {
        return format!(
            "no latest launch status receipt at `{}`; dx launch receipts --json will remain cold-start until dx launch status --json writes metadata",
            snapshot.latest_path.display()
        );
    }

    let latest = snapshot
        .latest
        .as_ref()
        .map(|latest| {
            format!(
                "{} freshness={} status={} schema={} age_ms={} malformed={} next_action={}",
                latest.file_name,
                latest.freshness_state,
                latest.status.as_deref().unwrap_or("unknown"),
                latest.schema_version.as_deref().unwrap_or("missing"),
                latest
                    .age_ms
                    .map(|age| age.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                latest.malformed,
                latest.next_action.as_deref().unwrap_or("none")
            )
        })
        .unwrap_or_else(|| "latest missing".to_string());

    format!(
        "schema={} command={} status={} summary={} latest_present={} snapshots={} malformed={} stale={} expired={} latest=[{}] next_action={}",
        snapshot.schema_version,
        snapshot.command,
        snapshot.status,
        snapshot.operator_summary,
        snapshot.latest_present,
        snapshot.snapshot_count,
        snapshot.malformed_count,
        snapshot.stale_count,
        snapshot.expired_count,
        latest,
        snapshot.next_action
    )
}

fn forge_history_prompt_context(snapshot: &DxToolHistorySnapshot) -> String {
    let Some(bucket) = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "Forge History")
    else {
        return "Forge history bucket is not tracked yet".to_string();
    };

    let state = if !bucket.root_exists {
        format!("missing root {}", bucket.root_label)
    } else if bucket.count == 0 {
        "root present with no receipts".to_string()
    } else {
        format!("{} receipt(s)", bucket.count)
    };
    let latest_summaries = bucket
        .latest_summaries
        .iter()
        .map(forge_history_summary_prompt)
        .collect::<Vec<_>>();
    let latest_summaries = bounded_join(
        &latest_summaries,
        3,
        "no parsed Forge receipt summaries are visible yet",
    );

    format!("{state}; latest summaries: {latest_summaries}")
}

fn forge_history_summary_prompt(summary: &DxToolHistoryReceiptSummary) -> String {
    let mut parts = vec![
        summary.headline.clone(),
        format!("kind {}", summary.kind),
        summary.detail.clone(),
        format!("receipt {}", summary.label),
    ];

    if let Some(target_path) = summary.target_path.as_ref() {
        parts.push(format!("target {target_path}"));
    }

    if let Some(preview_path) = summary.restore_destination_root.as_ref() {
        parts.push(format!("preview {preview_path}"));
    }

    if summary.blocker_count > 0 {
        parts.push(format!("blockers {}", summary.blocker_count));
    }

    parts.join(", ")
}

pub(super) fn bounded_join(values: &[String], limit: usize, empty: &'static str) -> String {
    if values.is_empty() {
        return empty.to_string();
    }

    values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}
