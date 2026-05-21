use ui::IconName;

use crate::dx_check_score::DxCheckScoreSnapshot;
use crate::dx_deploy_targets::{DxDeployReceiptBucket, DxDeployTarget, DxDeployTargetSnapshot};
use crate::dx_launch_contracts::DxLaunchContractSnapshot;
use crate::dx_launch_receipts::DxLaunchReceiptReviewSnapshot;
use crate::dx_launch_status::DxLaunchStatusSnapshot;
use crate::dx_launch_workspace::DxReceiptSnapshot;
use crate::dx_proof_freshness::DxProofFreshnessSnapshot;
use crate::dx_receipt_history::{DxToolHistoryReceiptSummary, DxToolHistorySnapshot};
use crate::dx_runtime_proof_status::{DxRuntimeProofReceiptSummary, DxRuntimeProofStatusSnapshot};
use crate::dx_source_sets::{DxSourceItem, DxSourceKind};

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

pub(crate) fn deploy_readiness_prompt(
    target: &DxDeployTarget,
    snapshot: &DxDeployTargetSnapshot,
) -> String {
    let latest = if snapshot.latest_receipts.is_empty() {
        "No deploy readiness receipts are present yet.".to_string()
    } else {
        format!(
            "Latest deploy readiness receipts: {}.",
            snapshot.latest_receipts.join(", ")
        )
    };
    let receipt_buckets = snapshot
        .receipt_buckets
        .iter()
        .map(deploy_receipt_bucket_prompt)
        .collect::<Vec<_>>()
        .join(", ");
    let receipt_buckets = if receipt_buckets.is_empty() {
        "No deploy receipt buckets are tracked yet.".to_string()
    } else {
        format!("Deploy receipt buckets: {receipt_buckets}.")
    };

    format!(
        "Inspect DX deploy readiness for {platform} target `{label}` at `{path}`. Read existing managed receipts under `tools/dx-deploy` if present; current deploy receipt count is {receipt_count}. {latest} {receipt_buckets} Report env, URL, log, rollback, and permission gaps. Do not deploy, run builds, start local servers, invoke browser automation, mutate files, or call external platform CLIs unless I explicitly approve a governed tool request.",
        platform = target.platform,
        label = target.label,
        path = target.path,
        receipt_count = snapshot.receipt_count,
        latest = latest,
        receipt_buckets = receipt_buckets,
    )
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

pub(crate) fn runtime_proof_prompt(
    check_score: &DxCheckScoreSnapshot,
    receipt_snapshot: &DxReceiptSnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
    runtime_proof_status: &DxRuntimeProofStatusSnapshot,
) -> String {
    let check_items = check_score
        .items
        .iter()
        .map(|item| format!("{}={}", item.label, item.state))
        .collect::<Vec<_>>();
    let check_items = bounded_join(&check_items, 6, "No Check score items are visible yet");
    let check_blockers = bounded_join(&check_score.blockers, 4, "No current Check blockers");
    let receipt_root = if receipt_snapshot.root_exists {
        format!(
            "receipt root present at `{}`",
            receipt_snapshot.root.display()
        )
    } else {
        format!(
            "receipt root missing at `{}`",
            receipt_snapshot.root.display()
        )
    };
    let latest_receipts = bounded_join(&receipt_snapshot.latest, 4, "No latest DX receipts");
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| {
            let latest = if bucket.latest.is_empty() {
                if bucket.root_exists {
                    "no latest receipt paths".to_string()
                } else {
                    format!("missing root {}", bucket.root_label)
                }
            } else {
                format!("latest {}", bucket.latest.join(", "))
            };

            format!(
                "{}: {} receipt(s), {}, {}; {}",
                bucket.label, bucket.count, bucket.status, bucket.description, latest
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let proof_rows = if proof_rows.is_empty() {
        "No proof freshness rows are available yet.".to_string()
    } else {
        format!("Current proof freshness rows: {proof_rows}.")
    };
    let deploy_target_rows = deploy_targets
        .targets
        .iter()
        .take(3)
        .map(|target| format!("{} {} at {}", target.platform, target.label, target.path))
        .collect::<Vec<_>>();
    let deploy_target_rows = bounded_join(&deploy_target_rows, 3, "No deploy targets detected");
    let deploy_receipts = deploy_targets
        .receipt_buckets
        .iter()
        .map(deploy_receipt_bucket_prompt)
        .collect::<Vec<_>>();
    let deploy_receipts = bounded_join(
        &deploy_receipts,
        8,
        "No deploy receipt buckets are tracked yet",
    );
    let runtime_status = runtime_proof_status_prompt_context(runtime_proof_status);

    format!(
        "Prepare the DX runtime proof handoff for this workspace. Current Check score: {score}/100 ({state}). Check items: {check_items}. Check blockers: {check_blockers}. Current receipts: {receipt_root}; latest receipts: {latest_receipts}. Deploy targets: {deploy_target_rows}. Deploy receipt buckets: {deploy_receipts}. Runtime proof status: {runtime_status}. {proof_rows} First use plan_dx_runtime_proof to write the governed manual validation checklist without running validation. If I provide operator evidence from that governed validation window, use import_dx_runtime_proof to write only managed runtime proof import/status receipts. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions unless I explicitly approve the governed tool request.",
        score = check_score.score,
        state = check_score.state,
    )
}

pub(crate) fn runtime_proof_import_prompt(
    check_score: &DxCheckScoreSnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
    runtime_proof_status: &DxRuntimeProofStatusSnapshot,
) -> String {
    let check_blockers = bounded_join(&check_score.blockers, 4, "No current Check blockers");
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| {
            let latest = if bucket.latest.is_empty() {
                "no latest receipts".to_string()
            } else {
                format!("latest {}", bucket.latest.join(", "))
            };
            format!(
                "{}={} ({}, {}; {})",
                bucket.label, bucket.count, bucket.status, bucket.description, latest
            )
        })
        .collect::<Vec<_>>();
    let proof_rows = bounded_join(&proof_rows, 4, "No proof freshness rows are available yet");
    let deploy_target_rows = deploy_targets
        .targets
        .iter()
        .take(3)
        .map(|target| format!("{} {} at {}", target.platform, target.label, target.path))
        .collect::<Vec<_>>();
    let deploy_target_rows = bounded_join(&deploy_target_rows, 3, "No deploy targets detected");
    let runtime_status = runtime_proof_status_prompt_context(runtime_proof_status);

    format!(
        "Prepare the DX runtime proof import handoff for this workspace. Current Check score: {score}/100 ({state}). Check blockers: {check_blockers}. Proof freshness rows: {proof_rows}. Deploy targets: {deploy_target_rows}. Runtime proof status: {runtime_status}. Operator evidence from the governed validation window is required before calling import_dx_runtime_proof. If I have not provided that evidence yet, draft the exact fields I need to provide and stop. When evidence is provided, use import_dx_runtime_proof with operator_status set to passed, blocked, or failed; include proof_summary, evidence lines, blockers, final_command, source, write_runtime_proof_receipt=true, and receipt_root_mode=workspace. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions.",
        score = check_score.score,
        state = check_score.state,
    )
}

pub(crate) fn runtime_proof_evidence_template_prompt(
    check_score: &DxCheckScoreSnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
    runtime_proof_status: &DxRuntimeProofStatusSnapshot,
) -> String {
    let check_blockers = bounded_join(&check_score.blockers, 4, "No current Check blockers");
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| format!("{}={} ({})", bucket.label, bucket.count, bucket.status))
        .collect::<Vec<_>>();
    let proof_rows = bounded_join(&proof_rows, 5, "No proof freshness rows are available yet");
    let deploy_target_rows = deploy_targets
        .targets
        .iter()
        .take(3)
        .map(|target| format!("{} {} at {}", target.platform, target.label, target.path))
        .collect::<Vec<_>>();
    let deploy_target_rows = bounded_join(&deploy_target_rows, 3, "No deploy targets detected");
    let runtime_status = runtime_proof_status_prompt_context(runtime_proof_status);
    let evidence_template = runtime_proof_evidence_template(runtime_proof_status);

    format!(
        "Draft a fillable DX runtime proof evidence template for this workspace and stop before importing anything. Current Check score: {score}/100 ({state}). Check blockers: {check_blockers}. Proof freshness rows: {proof_rows}. Deploy targets: {deploy_target_rows}. Runtime proof status: {runtime_status}. Use this template shape exactly and leave placeholders where evidence is missing: {evidence_template}. Do not call import_dx_runtime_proof until I provide completed operator evidence from the governed validation window. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions.",
        score = check_score.score,
        state = check_score.state,
    )
}

pub(crate) fn launch_handoff_prompt(
    contracts: &DxLaunchContractSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
) -> String {
    let contract_context = launch_contract_prompt_context(contracts);
    let launch_context = launch_status_prompt_context(launch_status);
    let receipt_context = launch_receipt_review_prompt_context(launch_receipts);

    format!(
        "Review the DX launch handoff for this Zed workspace. Launch contract metadata: {contract_context}. Launch aggregate: {launch_context}. Launch receipt diagnostics: {receipt_context}. Use the visible source-owned import-manifest and handoff packet metadata to summarize packet coverage, polling order, diagnostics commands, action-map safety, command fanout, redaction posture, cached receipt fallback, and missing proof. If the operator asks for a refresh, draft the exact `dx launch import-manifest --json`, `dx launch handoff --json`, `dx launch status --json`, `dx launch receipts --json`, or `dx launch release-gate --json` step, but do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn receipt_review_prompt(
    receipt_snapshot: &DxReceiptSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
    launch_contracts: &DxLaunchContractSnapshot,
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

    format!(
        "Inspect the current DX launch receipts for this workspace. {receipt_root}. Receipt buckets: {receipt_buckets}. Latest receipts: {latest_receipts}. Launch aggregate: {launch_context}. Launch handoff contracts: {launch_contract_context}. Launch receipt diagnostics: {launch_receipt_context}. Tool history buckets: {tool_buckets}. Forge history context: {forge_history}. Proof freshness buckets: {proof_rows}. Deploy receipt buckets: {deploy_rows}. Summarize the latest launch status, launch receipt freshness, malformed retained snapshots, handoff packet coverage, metasearch, source attachment, serializer/RLM context, execution, runner-gate, reduced-context, execution-preview, external-execution, media, Forge, restore-approval, restore-target plan, runtime-proof plan/import/status, and deploy receipts. Report missing receipt roots gracefully and give the next safe action without running builds, local servers, browser input, external serializer/RLM code, restore-to-target actions, deploys, shell commands, or model calls."
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

fn deploy_receipt_bucket_prompt(bucket: &DxDeployReceiptBucket) -> String {
    let mut parts = vec![format!(
        "{}={} ({})",
        bucket.label, bucket.count, bucket.status
    )];

    if let Some(summary) = bucket.latest_summary.as_ref() {
        let mut latest = vec![
            format!("latest {}", summary.label),
            format!("headline {}", summary.headline),
        ];

        if let Some(status) = summary.status.as_ref() {
            latest.push(format!("receipt_status {status}"));
        }

        if let Some(target) = summary.target.as_ref() {
            latest.push(format!("target {target}"));
        }

        if let Some(url) = summary.url.as_ref() {
            latest.push(format!("url {url}"));
        }

        if summary.blocker_count > 0 {
            latest.push(format!("blockers {}", summary.blocker_count));
        }

        parts.push(latest.join(", "));
    }

    parts.join("; ")
}

fn runtime_proof_status_prompt_context(snapshot: &DxRuntimeProofStatusSnapshot) -> String {
    let latest_plan = snapshot
        .latest_plan
        .as_ref()
        .map(|plan| {
            let requirements = runtime_proof_plan_requirements(plan);
            let command = plan
                .expected_final_command
                .clone()
                .unwrap_or_else(|| "unknown command".to_string());
            format!(
                "latest plan {} status {} command {} steps {} required {} minimum_evidence {} examples {} requirements {} blockers {}",
                plan.label,
                plan.status,
                command,
                plan.checklist_step_count,
                plan.required_step_count,
                runtime_proof_minimum_evidence(plan),
                bounded_join(
                    &plan.accepted_evidence_examples,
                    3,
                    "no accepted evidence examples"
                ),
                requirements,
                plan.blocker_count
            )
        })
        .unwrap_or_else(|| "no latest plan receipt".to_string());
    let latest_import = snapshot
        .latest_import
        .as_ref()
        .map(|receipt| runtime_proof_receipt_prompt_context("import", receipt))
        .unwrap_or_else(|| "no latest import receipt".to_string());
    let latest_status = snapshot
        .latest_status
        .as_ref()
        .map(|receipt| runtime_proof_receipt_prompt_context("status", receipt))
        .unwrap_or_else(|| "no latest status receipt".to_string());
    let blockers = bounded_join(&snapshot.blockers, 3, "no runtime proof status blockers");

    format!(
        "{}; {} plan receipt(s), {} import receipt(s), {} status receipt(s); {}; {}; {}; blockers: {}",
        snapshot.claim_state,
        snapshot.plan_receipt_count,
        snapshot.import_receipt_count,
        snapshot.status_receipt_count,
        latest_plan,
        latest_import,
        latest_status,
        blockers
    )
}

fn runtime_proof_receipt_prompt_context(
    kind: &str,
    receipt: &DxRuntimeProofReceiptSummary,
) -> String {
    let summary = receipt
        .proof_summary
        .clone()
        .unwrap_or_else(|| "no summary".to_string());
    let command = receipt
        .final_command
        .clone()
        .unwrap_or_else(|| "no final command".to_string());
    let source = receipt
        .source
        .clone()
        .unwrap_or_else(|| "no source".to_string());
    let evidence_sample = bounded_join(&receipt.evidence_samples, 1, "no evidence sample");

    format!(
        "latest {kind} {} status {} operator {} claim_ready {} evidence {} blockers {} summary {} command {} source {} sample {}",
        receipt.label,
        receipt.validation_status,
        receipt.operator_status,
        receipt.can_claim_runtime_green,
        receipt.evidence_count,
        receipt.blocker_count,
        summary,
        command,
        source,
        evidence_sample
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

fn runtime_proof_evidence_template(snapshot: &DxRuntimeProofStatusSnapshot) -> String {
    let final_command = snapshot
        .latest_plan
        .as_ref()
        .and_then(|plan| plan.expected_final_command.clone())
        .unwrap_or_else(|| "just run".to_string());
    let minimum_evidence = snapshot
        .latest_plan
        .as_ref()
        .map(runtime_proof_minimum_evidence)
        .unwrap_or(1);
    let accepted_examples = snapshot
        .latest_plan
        .as_ref()
        .map(|plan| {
            bounded_join(
                &plan.accepted_evidence_examples,
                5,
                "final command exit status, visible Zed/DX window title, Agent panel route or action exercised",
            )
        })
        .unwrap_or_else(|| {
            "final command exit status, visible Zed/DX window title, Agent panel route or action exercised"
                .to_string()
        });

    format!(
        "operator_status=<passed|blocked|failed>; proof_summary=<one sentence>; final_command={final_command}; source=<governed validation window>; evidence=<at least {minimum_evidence} line(s): {accepted_examples}>; blockers=<empty when passed, otherwise blocker lines>; write_runtime_proof_receipt=true; receipt_root_mode=workspace"
    )
}

fn runtime_proof_minimum_evidence(
    plan: &crate::dx_runtime_proof_status::DxRuntimeProofPlanSummary,
) -> usize {
    plan.minimum_evidence_lines_for_pass.max(1)
}

fn runtime_proof_plan_requirements(
    plan: &crate::dx_runtime_proof_status::DxRuntimeProofPlanSummary,
) -> String {
    let mut requirements = Vec::new();

    if plan.requires_clean_git {
        requirements.push("clean_git");
    }
    if plan.requires_diff_check {
        requirements.push("diff_check");
    }
    if plan.requires_visual_evidence {
        requirements.push("visual_evidence");
    }
    if plan.requires_import {
        requirements.push("runtime_proof_import");
    }

    if requirements.is_empty() {
        "none".to_string()
    } else {
        requirements.join(",")
    }
}

fn bounded_join(values: &[String], limit: usize, empty: &'static str) -> String {
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
