use crate::dx_launch_audit::DxLaunchAuditSnapshot;
use crate::dx_launch_contracts::DxLaunchContractSnapshot;
use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;
use crate::dx_launch_receipts::DxLaunchReceiptReviewSnapshot;
use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;
use crate::dx_launch_status::DxLaunchStatusSnapshot;
use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

pub(super) fn launch_status_prompt_context(snapshot: &DxLaunchStatusSnapshot) -> String {
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

pub(super) fn launch_contract_prompt_context(snapshot: &DxLaunchContractSnapshot) -> String {
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

pub(super) fn launch_readiness_prompt_context(snapshot: &DxLaunchReadinessSnapshot) -> String {
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

pub(super) fn launch_audit_prompt_context(snapshot: &DxLaunchAuditSnapshot) -> String {
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

pub(super) fn launch_www_evidence_prompt_context(snapshot: &DxWwwLaunchEvidenceSnapshot) -> String {
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

pub(super) fn launch_source_audit_prompt_context(snapshot: &DxLaunchSourceAuditSnapshot) -> String {
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

pub(super) fn launch_receipt_review_prompt_context(
    snapshot: &DxLaunchReceiptReviewSnapshot,
) -> String {
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

pub(super) fn bounded_join(values: &[String], limit: usize, empty: &'static str) -> String {
    if values.is_empty() {
        return empty.to_string();
    }

    let mut rendered = values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let remaining_count = values.len().saturating_sub(limit);
    if remaining_count > 0 {
        rendered.push_str(&format!(", +{} more", remaining_count));
    }

    rendered
}
