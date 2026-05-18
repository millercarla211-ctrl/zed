pub(crate) const AGENT_BROWSER_PAYLOAD_QUEUE_FILE_NAME: &str = "latest-agent-browser-payload.json";
pub(crate) const MANAGED_CHROME_EXECUTIONS_DIR_NAME: &str = "chrome-executions";
pub(crate) const MANAGED_CHROME_RUN_REQUEST_PREFIX: &str = "managed-chrome-run-request-";
pub(crate) const MANAGED_CHROME_EXECUTION_RECEIPT_PREFIX: &str =
    "managed-chrome-execution-receipt-";
pub(crate) const MANAGED_CHROME_RUN_REQUEST_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_run_request.v1";
pub(crate) const MANAGED_CHROME_EXECUTION_RECEIPT_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_execution_receipt.v1";
pub(crate) const MANAGED_CHROME_ACTION_CARD_SCHEMA: &str =
    "zed.web_preview.managed_chrome_action_card.v1";
pub(crate) const MANAGED_CHROME_ACTION_CARD_DISPLAY_SCHEMA: &str =
    "zed.web_preview.managed_chrome_action_card_display.v1";
pub(crate) const PC_USE_PAYLOAD_QUEUE_FILE_NAME: &str = "latest-zed-pc-use-payload.json";
pub(crate) const PC_USE_RUNNER_RECEIPT_FILE_NAME: &str = "latest-zed-pc-use-runner-receipt.json";
pub(crate) const PC_USE_RUNNER_RECEIPT_PREFIX: &str = "zed-pc-use-runner-receipt-";
pub(crate) const PC_USE_PAYLOAD_QUEUE_ITEM_SCHEMA: &str =
    "zed.agent_plugins.pc_use.action_payload_queue_item.v1";
pub(crate) const PC_USE_PAYLOAD_SCHEMA: &str = "zed.agent_plugins.pc_use.action_payload.v1";
pub(crate) const PC_USE_RUNNER_RECEIPT_SCHEMA: &str = "zed.agent_plugins.pc_use.runner_receipt.v1";
pub(crate) const PC_USE_PROOF_SUMMARY_SCHEMA: &str = "zed.web_preview.pc_use_proof_summary.v1";
pub(crate) const PC_USE_PROOF_CARD_SCHEMA: &str = "zed.web_preview.pc_use_proof_card.v1";

pub(crate) const AGENT_BROWSER_EXECUTOR_VALIDATION_PROGRESS_SCHEMA: &str =
    "zed.web_preview.agent_browser_executor_validation_progress.v1";
pub(crate) const AGENT_BROWSER_NATIVE_DISPATCH_RECEIPT_MATRIX_SCHEMA: &str =
    "zed.web_preview.native_dispatch_receipt_matrix.v1";
pub(crate) const AGENT_BROWSER_STATUS_PACKET_SCHEMA: &str =
    "zed.web_preview.agent_browser_status_packet.v1";
pub(crate) const AGENT_BROWSER_STATUS_PACKET_SUMMARY_SCHEMA: &str =
    "zed.web_preview.agent_browser_status_packet_summary.v1";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_BUNDLE_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_bundle.v1";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_BUNDLE_SUMMARY_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_bundle_summary.v1";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_RESULT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_result.v1";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_ALLOWED_STATUS_VALUES: &[&str] =
    &["not_run", "pass", "fail", "blocked", "skipped"];
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_REQUIRED_CHECK_IDS: &[&str] = &[
    "editor_typing",
    "webpreview_input",
    "git_sync",
    "just_dry_run",
    "final_runtime_capacity",
    "final_headroom_recovery_sequence",
    "panel_live_validation",
    "agent_runtime_panel_live_contract",
    "native_executor_receipts",
    "payload_bridge",
    "managed_chrome",
    "pc_use",
];
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_RESULT_IMPORT_RECEIPT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_result_import_receipt.v1";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_OBSERVABILITY_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_validation_observability.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_PROOF_CAPACITY_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_proof_capacity.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_TARGET_DRIVE_POLICY_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_target_drive_policy.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_RECOVERY_PLAN_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_recovery_plan.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_RECOVERY_CARD_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_recovery_card.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_INSPECTION_CHECKLIST_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_inspection_checklist.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_SIZE_INSPECTION_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_size_inspection.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_CLEANUP_RESULT_TEMPLATE_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_cleanup_result_template.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_CLEANUP_RESULT_GATE_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_cleanup_result_gate.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_CLEANUP_RESULT_IMPORT_RECEIPT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_cleanup_result_import_receipt.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_READINESS_GATE_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_readiness_gate.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_RECLAIM_CANDIDATES_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_reclaim_candidates.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_RECOVERY_SEQUENCE_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_headroom_recovery_sequence.v1";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_BLOCKER_BOARD_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_runtime_blocker_board.v1";
pub(crate) const AGENT_BROWSER_FINAL_PROOF_AUDIT_SCHEMA: &str =
    "zed.web_preview.agent_browser_final_proof_audit.v1";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_DIR_NAME: &str = "browser-final-validation";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_RESULT_FILE_NAME: &str =
    "latest-agent-browser-final-validation-result.json";
pub(crate) const AGENT_BROWSER_FINAL_VALIDATION_RESULT_ARCHIVE_PREFIX: &str =
    "agent-browser-final-validation-result-";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_CLEANUP_RESULT_DIR_NAME: &str =
    "browser-final-runtime-headroom-cleanup-results";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_CLEANUP_RESULT_FILE_NAME: &str =
    "latest-agent-browser-final-runtime-headroom-cleanup-result.json";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_HEADROOM_CLEANUP_RESULT_ARCHIVE_PREFIX: &str =
    "agent-browser-final-runtime-headroom-cleanup-result-";

pub(crate) const AGENT_BROWSER_FUNCTION_SURFACES_SCHEMA: &str =
    "zed.web_preview.agent_browser_function_surfaces.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_DECK_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_deck.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_DISPLAY_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_display.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_AFFORDANCE_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_affordance.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_CONTROL_STATE_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_control_state.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_CONTROL_EVENT_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_control_event.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_CONTROL_RESULT_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_control_result.v1";
pub(crate) const AGENT_BROWSER_PANEL_CONTROL_RESULT_LEDGER_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_control_result_ledger.v1";
pub(crate) const AGENT_BROWSER_PANEL_CONTROL_RESULT_IMPORT_RECEIPT_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_control_result_import_receipt.v1";
pub(crate) const AGENT_BROWSER_PANEL_CONTROL_RESULT_DIR_NAME: &str =
    "browser-panel-control-results";
pub(crate) const AGENT_BROWSER_PANEL_CONTROL_RESULT_FILE_NAME: &str =
    "latest-agent-browser-panel-control-result.json";
pub(crate) const AGENT_BROWSER_PANEL_CONTROL_RESULT_ARCHIVE_PREFIX: &str =
    "agent-browser-panel-control-result-";
pub(crate) const AGENT_BROWSER_PANEL_LIVE_VALIDATION_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_live_validation.v1";
pub(crate) const AGENT_BROWSER_PANEL_LIVE_VALIDATION_RESULT_GATE_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_live_validation_result_gate.v1";
pub(crate) const AGENT_BROWSER_PANEL_LIVE_VALIDATION_EXERCISE_PLAN_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_live_validation_exercise_plan.v1";
pub(crate) const AGENT_BROWSER_PANEL_LIVE_UI_PROOF_CHECKLIST_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_live_ui_proof_checklist.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_INTERACTION_VALIDATION_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_interaction_validation.v1";
pub(crate) const AGENT_BROWSER_PANEL_CARD_RENDER_CONTRACT_SCHEMA: &str =
    "zed.web_preview.agent_browser_panel_card_render_contract.v1";
pub(crate) const INSPECTED_ELEMENT_SCHEMA: &str = "zed.web_preview.inspected_element.v1";
pub(crate) const INSPECTED_ELEMENT_EVIDENCE_CARD_SCHEMA: &str =
    "zed.web_preview.inspected_element_evidence_card.v1";
pub(crate) const DEVTOOLS_EVIDENCE_CARD_SCHEMA: &str = "zed.web_preview.devtools_evidence_card.v1";

pub(crate) const AGENT_PLUGIN_CATALOG_SUMMARY_SCHEMA: &str = "zed.agent_plugins.catalog_summary.v1";
pub(crate) const AGENT_PLUGIN_BOOTSTRAP_READINESS_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_readiness.v1";
pub(crate) const AGENT_PLUGIN_BOOTSTRAP_MANIFEST_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_manifest.v1";
pub(crate) const AGENT_PLUGIN_BOOTSTRAP_PREPARE_REQUEST_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_prepare_request.v1";
pub(crate) const AGENT_PLUGIN_BOOTSTRAP_ASSET_PLAN_SCHEMA: &str =
    "zed.agent_plugins.bootstrap_asset_plan.v1";
pub(crate) const AGENT_PLUGIN_MANAGED_ASSET_OPERATOR_RECIPE_SCHEMA: &str =
    "zed.agent_plugins.managed_asset_operator_recipe.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_BLOCKERS_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_blocker_summary.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_SCORECARD_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_readiness_scorecard.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_OPERATOR_HANDOFF_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_operator_handoff.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_GATE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_claim_gate.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_CLAIM_READINESS_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_claim_readiness.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_GATE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_gate.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_BADGE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_badge.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_proof_guide.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_GUIDE_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_proof_guide_summary.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_report_packet.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_REPORT_PACKET_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_report_packet_summary.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_readiness_card.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_REPORT_READINESS_CARD_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_report_readiness_card_summary.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_FINAL_PROOF_AUDIT_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_final_proof_audit_summary.v1";
pub(crate) const AGENT_PLUGIN_BROWSER_PANEL_LIVE_PROOF_STATUS_SCHEMA: &str =
    "zed.agent_plugins.browser_panel_live_proof_status.v1";
pub(crate) const AGENT_PLUGIN_BROWSER_PANEL_LIVE_PROOF_READINESS_CARD_SCHEMA: &str =
    "zed.agent_plugins.browser_panel_live_proof_readiness_card.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_GREEN_PROOF_PATH_SCHEMA: &str =
    "zed.agent_plugins.runtime_green_proof_path.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_DIGEST_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_digest.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_MATRIX_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_plugin_matrix.v1";
pub(crate) const AGENT_PLUGIN_RUNTIME_OBSERVABILITY_WATCH_ROLLUP_SCHEMA: &str =
    "zed.agent_plugins.runtime_observability_regression_watch_rollup.v1";
pub(crate) const AGENT_PLUGIN_ASSET_PROVISIONING_RESULT_SCHEMA: &str =
    "zed.agent_plugins.asset_provisioning_result.v1";
pub(crate) const AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_SCHEMA: &str =
    "zed.agent_plugins.asset_provisioning_receipt.v1";
pub(crate) const AGENT_PLUGIN_ASSET_READINESS_SUMMARY_SCHEMA: &str =
    "zed.agent_plugins.asset_readiness_summary.v1";
pub(crate) const AGENT_PLUGIN_ASSET_PROVISIONING_RECEIPT_FILE_NAME: &str =
    "agent-plugin-asset-provisioning.json";
pub(crate) const AGENT_CHROME_PLAYWRIGHT_ADAPTER_MANIFEST_SCHEMA: &str =
    "zed.agent_plugins.managed_chrome_playwright_adapter_manifest.v1";
pub(crate) const AGENT_CHROME_PLAYWRIGHT_ADAPTER_ROOT_NAME: &str = "zed-managed-chrome-runner";
pub(crate) const AGENT_CHROME_PLAYWRIGHT_RUNNER_SCRIPT_NAME: &str = "managed_chrome_runner.mjs";
pub(crate) const PREPARE_AGENT_PLUGIN_RUNTIME_TOOL: &str = "prepare_agent_plugin_runtime";
pub(crate) const PREPARE_AGENT_PLUGIN_MANAGED_ASSETS_TOOL: &str =
    "prepare_agent_plugin_managed_assets";
pub(crate) const PREPARE_MANAGED_CHROME_PLAYWRIGHT_ADAPTER_TOOL: &str =
    "prepare_managed_chrome_playwright_adapter";
pub(crate) const AGENT_BROWSER_FINAL_RUNTIME_MIN_FREE_BYTES: u64 = 18 * 1024 * 1024 * 1024;
