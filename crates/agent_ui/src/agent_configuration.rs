mod add_llm_provider_modal;
pub mod configure_context_server_modal;
mod configure_context_server_tools_modal;
mod manage_profiles_modal;
mod tool_picker;

use std::{ops::Range, rc::Rc, sync::Arc};

use agent::ContextServerRegistry;
use anyhow::Result;
use cloud_api_types::Plan;
use collections::HashMap;
use context_server::ContextServerId;
use editor::{Editor, MultiBufferOffset, SelectionEffects, scroll::Autoscroll};
use extension::ExtensionManifest;
use extension_host::ExtensionStore;
use fs::Fs;
use gpui::{
    Action, Anchor, AnyView, App, AsyncWindowContext, Entity, EventEmitter, FocusHandle, Focusable,
    ScrollHandle, Subscription, Task, TaskExt, WeakEntity,
};
use itertools::Itertools;
use language::LanguageRegistry;
use language_model::{
    IconOrSvg, LanguageModelProvider, LanguageModelProviderId, LanguageModelRegistry,
    ZED_CLOUD_PROVIDER_ID,
};
use language_models::AllLanguageModelSettings;
use notifications::status_toast::StatusToast;
use project::{
    agent_server_store::{AgentId, AgentServerStore, ExternalAgentSource},
    context_server_store::{ContextServerConfiguration, ContextServerStatus, ContextServerStore},
};
use settings::{Settings, SettingsStore, update_settings_file};
use ui::{
    AiSettingItem, AiSettingItemSource, AiSettingItemStatus, ButtonStyle, Chip, ContextMenu,
    ContextMenuEntry, Disclosure, Divider, DividerColor, ElevationIndex, LabelSize, PopoverMenu,
    Switch, Tooltip, WithScrollbar, prelude::*,
};
use util::ResultExt as _;
use workspace::{Workspace, create_and_open_local_file};
use zed_actions::{ExtensionCategoryFilter, OpenBrowser};

pub(crate) use configure_context_server_modal::ConfigureContextServerModal;
pub(crate) use configure_context_server_tools_modal::ConfigureContextServerToolsModal;
pub(crate) use manage_profiles_modal::ManageProfilesModal;

use crate::{
    Agent,
    agent_configuration::add_llm_provider_modal::{AddLlmProviderModal, LlmCompatibleProvider},
    agent_connection_store::{AgentConnectionStatus, AgentConnectionStore},
    dx_agent_bridge::{
        DxAgentBridgeSnapshot, DxAgentReceipt, DxAgentRowAction, DxAgentSocialActionSummary,
        dx_agent_bridge_snapshot, dx_agent_cli_actions_allowed, dx_agent_cli_path,
        dx_agent_dx_home, dx_agent_receipt_root, run_dx_agent_command,
        run_dx_agent_import_summary_command, run_dx_agent_release_gate_command,
    },
};

pub struct AgentConfiguration {
    fs: Arc<dyn Fs>,
    language_registry: Arc<LanguageRegistry>,
    agent_server_store: Entity<AgentServerStore>,
    agent_connection_store: Entity<AgentConnectionStore>,
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    configuration_views_by_provider: HashMap<LanguageModelProviderId, AnyView>,
    context_server_store: Entity<ContextServerStore>,
    expanded_provider_configurations: HashMap<LanguageModelProviderId, bool>,
    context_server_registry: Entity<ContextServerRegistry>,
    _subscriptions: Vec<Subscription>,
    scroll_handle: ScrollHandle,
}

fn dx_agent_row_action<'a>(
    actions: &'a [DxAgentRowAction],
    id: &str,
) -> Option<&'a DxAgentRowAction> {
    actions.iter().find(|action| action.id == id)
}

fn dx_agent_action_summary(actions: &[DxAgentRowAction]) -> Option<String> {
    if actions.is_empty() {
        return None;
    }

    let summary = actions
        .iter()
        .take(3)
        .map(|action| {
            let state = if action.enabled { "ready" } else { "disabled" };
            let validation = if !action.command.is_empty() && !action.secrets_exposed {
                "validated"
            } else {
                "blocked"
            };
            let user_action = if action.user_action_required {
                ", user action"
            } else {
                ""
            };
            let receipt = if action.writes_receipt {
                format!(" -> {}", action.receipt_filename)
            } else {
                String::new()
            };
            format!(
                "{} {} {}{}{}",
                action.label, state, validation, user_action, receipt
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    Some(format!("Actions: {summary}"))
}

fn dx_agent_action_tooltip(action: Option<&DxAgentRowAction>, fallback: &str) -> String {
    action
        .map(|action| {
            let refresh = if action.refresh_command.is_empty() {
                "without refresh handoff"
            } else {
                "with fixed refresh handoff"
            };
            format!(
                "{}; writes {}; {}",
                fallback, action.receipt_filename, refresh
            )
        })
        .unwrap_or_else(|| fallback.to_string())
}

impl AgentConfiguration {
    pub fn new(
        fs: Arc<dyn Fs>,
        agent_server_store: Entity<AgentServerStore>,
        agent_connection_store: Entity<AgentConnectionStore>,
        context_server_store: Entity<ContextServerStore>,
        context_server_registry: Entity<ContextServerRegistry>,
        language_registry: Arc<LanguageRegistry>,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        let subscriptions = vec![
            cx.subscribe_in(
                &LanguageModelRegistry::global(cx),
                window,
                |this, _, event: &language_model::Event, window, cx| match event {
                    language_model::Event::AddedProvider(provider_id) => {
                        let provider = LanguageModelRegistry::read_global(cx).provider(provider_id);
                        if let Some(provider) = provider {
                            this.add_provider_configuration_view(&provider, window, cx);
                        }
                    }
                    language_model::Event::RemovedProvider(provider_id) => {
                        this.remove_provider_configuration_view(provider_id);
                    }
                    _ => {}
                },
            ),
            cx.subscribe(&agent_server_store, |_, _, _, cx| cx.notify()),
            cx.observe(&agent_connection_store, |_, _, cx| cx.notify()),
            cx.subscribe(&context_server_store, |_, _, _, cx| cx.notify()),
        ];

        let mut this = Self {
            fs,
            language_registry,
            workspace,
            focus_handle,
            configuration_views_by_provider: HashMap::default(),
            agent_server_store,
            agent_connection_store,
            context_server_store,
            expanded_provider_configurations: HashMap::default(),
            context_server_registry,
            _subscriptions: subscriptions,
            scroll_handle: ScrollHandle::new(),
        };

        this.build_provider_configuration_views(window, cx);
        this
    }

    fn build_provider_configuration_views(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let providers = LanguageModelRegistry::read_global(cx).visible_providers();
        for provider in providers {
            self.add_provider_configuration_view(&provider, window, cx);
        }
    }

    fn remove_provider_configuration_view(&mut self, provider_id: &LanguageModelProviderId) {
        self.configuration_views_by_provider.remove(provider_id);
        self.expanded_provider_configurations.remove(provider_id);
    }

    fn add_provider_configuration_view(
        &mut self,
        provider: &Arc<dyn LanguageModelProvider>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let configuration_view = provider.configuration_view(
            language_model::ConfigurationViewTargetAgent::ZedAgent,
            window,
            cx,
        );
        self.configuration_views_by_provider
            .insert(provider.id(), configuration_view);
    }
}

impl Focusable for AgentConfiguration {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub enum AssistantConfigurationEvent {
    NewThread(Arc<dyn LanguageModelProvider>),
}

impl EventEmitter<AssistantConfigurationEvent> for AgentConfiguration {}

enum AgentIcon {
    Name(IconName),
    Path(SharedString),
}

impl AgentConfiguration {
    fn render_section_title(
        &mut self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        menu: AnyElement,
    ) -> impl IntoElement {
        h_flex()
            .p_4()
            .pb_0()
            .mb_2p5()
            .items_start()
            .justify_between()
            .child(
                v_flex()
                    .w_full()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .pr_1()
                            .w_full()
                            .gap_2()
                            .justify_between()
                            .flex_wrap()
                            .child(Headline::new(title.into()))
                            .child(menu),
                    )
                    .child(Label::new(description.into()).color(Color::Muted)),
            )
    }

    fn render_dx_agents_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = dx_agent_bridge_snapshot(cx);
        let actions_allowed = snapshot.enabled && snapshot.cli_actions_allowed;
        let controls = h_flex()
            .gap_1()
            .child(
                Button::new("dx-agents-refresh-status", "Refresh")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(vec!["agents", "status", "--json"], cx);
                    })),
            )
            .child(
                Button::new("dx-agents-contract", "Contract")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(vec!["agents", "contract", "--json"], cx);
                    })),
            )
            .child(
                Button::new("dx-agents-import-summary", "Summary")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_metadata_action(
                            vec!["agents", "import-summary", "--json"],
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("dx-agents-release-gate", "Gate")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_metadata_action(
                            vec!["agents", "release-gate", "--json"],
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("dx-agents-social-list", "Social")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(
                            vec!["agents", "social", "list", "--json"],
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("dx-agents-automations-list", "Automations")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(
                            vec!["agents", "automate", "list", "--json"],
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("dx-agents-receipts", "Receipts")
                    .style(ButtonStyle::Outlined)
                    .label_size(LabelSize::Small)
                    .disabled(!actions_allowed)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(
                            vec!["agents", "receipts", "list", "--json"],
                            cx,
                        );
                    })),
            );

        v_flex()
            .min_w_0()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(self.render_section_title(
                "DX Agents",
                "CLI-first DX Agents bridge status, social readiness, background receipts, and managed provider/model catalog discovery.",
                controls.into_any_element(),
            ))
            .child(
                v_flex()
                    .pl_4()
                    .pb_4()
                    .pr_5()
                    .w_full()
                    .gap_2()
                    .child(self.render_dx_agents_status_item(&snapshot, cx))
                    .child(self.render_dx_agents_contract_item(&snapshot, cx))
                    .child(self.render_dx_agents_import_summary_item(&snapshot, cx))
                    .child(self.render_dx_agents_release_gate_item(&snapshot, cx))
                    .child(self.render_dx_agents_social_items(&snapshot, cx))
                    .child(self.render_dx_agents_automation_items(&snapshot, cx))
                    .child(self.render_dx_agents_receipt_items(&snapshot, cx))
                    .child(self.render_dx_agents_catalog_items(&snapshot, cx)),
            )
    }

    fn render_dx_agents_status_item(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let status = if !snapshot.enabled || !snapshot.root_exists {
            AiSettingItemStatus::Stopped
        } else if snapshot.last_error.is_some() || snapshot.status == "warning" {
            AiSettingItemStatus::Starting
        } else {
            AiSettingItemStatus::Running
        };
        let detail = if let Some(error) = snapshot.last_error.as_ref() {
            error.clone()
        } else {
            format!(
                "{} task(s), {} automation(s), receipts {}, cli {}",
                snapshot.active_task_count,
                snapshot.automation_count,
                if snapshot.root_exists {
                    "ready"
                } else {
                    "missing"
                },
                snapshot.cli_path
            )
        };

        AiSettingItem::new(
            "dx-agents-bridge",
            "DX Agents Runtime",
            status,
            AiSettingItemSource::Custom,
        )
        .icon(
            Icon::new(IconName::ZedAgent)
                .size(IconSize::Small)
                .color(Color::Muted),
        )
        .detail_label(detail)
        .when(snapshot.enabled && snapshot.cli_actions_allowed, |this| {
            this.action(
                IconButton::new("dx-agents-run-receipt", IconName::PlayOutlined)
                    .icon_color(Color::Muted)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Write a DX Agents run receipt"))
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(vec!["agents", "run", "--json"], cx);
                    })),
            )
        })
    }

    fn render_dx_agents_contract_item(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        let summary = &snapshot.contract_summary;
        let status = if !summary.present {
            AiSettingItemStatus::Stopped
        } else if summary.redaction_requires_review || summary.status == "warning" {
            AiSettingItemStatus::Starting
        } else {
            AiSettingItemStatus::Running
        };
        let mut stack = v_flex().gap_1().child(
            AiSettingItem::new(
                "dx-agents-contract-summary",
                "Bridge Contract",
                status,
                AiSettingItemSource::Custom,
            )
            .icon(
                Icon::new(IconName::FileTextOutlined)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .detail_label(format!(
                "{} command(s), {} receipt(s), catalog {}, redaction {}",
                summary.command_count,
                summary.receipt_count,
                summary.provider_catalog_source,
                summary.redaction_summary
            )),
        );

        if !summary.present {
            stack = stack.child(
                Label::new("No contract receipt yet. Run the Contract action.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if let Some(note) = summary.receipt_notes.first() {
            stack = stack.child(
                Label::new(note.clone())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        if summary.redaction_requires_review {
            stack = stack.child(
                Label::new("Contract redaction flags need review before launch.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if summary.present {
            stack = stack.child(
                Label::new(format!("Next: {}", summary.next_action))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        stack.into_any_element()
    }

    fn render_dx_agents_import_summary_item(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        let summary = &snapshot.import_summary;
        let status = if !summary.present
            || summary.release_gate_failed_count > 0
            || !summary.no_command_fanout
        {
            AiSettingItemStatus::Stopped
        } else if summary.release_gate_warning_count > 0 || summary.status == "warning" {
            AiSettingItemStatus::Starting
        } else {
            AiSettingItemStatus::Running
        };
        let mut stack = v_flex().gap_1().child(
            AiSettingItem::new(
                "dx-agents-import-summary",
                "Import Summary",
                status,
                AiSettingItemSource::Custom,
            )
            .icon(
                Icon::new(IconName::FileTextOutlined)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .detail_label(format!(
                "release {}, action map {}, {} action(s), freshness {}",
                summary.release_gate_status,
                summary.action_map_status,
                summary.action_count,
                summary.freshness_state
            )),
        );

        if !summary.present {
            stack = stack.child(
                Label::new("No import summary receipt yet. Run the Summary action.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if !summary.operator_summary.is_empty() {
            stack = stack.child(
                Label::new(summary.operator_summary.clone())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        if let Some(reason) = summary.blocking_reasons.first() {
            stack = stack.child(
                Label::new(format!("Blocked: {reason}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if let Some(reason) = summary.warning_reasons.first() {
            stack = stack.child(
                Label::new(format!("Warning: {reason}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if summary.present {
            let states = if summary.recovery_states.is_empty() {
                "none".to_string()
            } else {
                summary.recovery_states.join(", ")
            };
            stack = stack.child(
                Label::new(format!(
                    "Recovery: {} via {}, states {}",
                    summary.recovery_controls_status, summary.recovery_render_first, states
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            );
        }

        stack.into_any_element()
    }

    fn render_dx_agents_release_gate_item(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        let summary = &snapshot.release_gate;
        let status = if !summary.present || summary.failed_count > 0 || !summary.no_command_fanout {
            AiSettingItemStatus::Stopped
        } else if summary.warning_count > 0 || summary.status == "warning" {
            AiSettingItemStatus::Starting
        } else {
            AiSettingItemStatus::Running
        };
        let mut stack = v_flex().gap_1().child(
            AiSettingItem::new(
                "dx-agents-release-gate",
                "Release Gate",
                status,
                AiSettingItemSource::Custom,
            )
            .icon(
                Icon::new(IconName::Check)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .detail_label(format!(
                "{} passed / {} total, {} warning(s), {} blocker(s)",
                summary.passed_count,
                summary.acceptance_count,
                summary.warning_count,
                summary.failed_count
            )),
        );

        if !summary.present {
            stack = stack.child(
                Label::new("No release gate receipt yet. Run the Gate action.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if !summary.operator_summary.is_empty() {
            stack = stack.child(
                Label::new(summary.operator_summary.clone())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        if let Some(reason) = summary.blocking_reasons.first() {
            stack = stack.child(
                Label::new(format!("Blocked: {reason}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if let Some(reason) = summary.warning_reasons.first() {
            stack = stack.child(
                Label::new(format!("Warning: {reason}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if summary.present {
            stack = stack.child(
                Label::new(format!(
                    "Manifest {}, smoke {}, receipts {}",
                    summary.import_manifest_status,
                    summary.smoke_status,
                    summary.receipt_inbox_status
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            );
            stack = stack.child(
                Label::new(format!(
                    "Action map {}, retention {}, packets {}, fixtures {}",
                    summary.action_map_status,
                    summary.retention_preview_status,
                    summary.packet_count,
                    summary.fixture_family_count
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            );
            stack = stack.child(
                Label::new(format!(
                    "Recovery: {} via {}, {} fixture(s), retained overflow {}",
                    summary.recovery_controls_status,
                    summary.recovery_render_first,
                    summary.recovery_fixture_count,
                    summary.retained_run_overflow_count
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            );
        }

        if let Some(row) = summary.acceptance_rows.first() {
            stack = stack.child(
                Label::new(row.clone())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        stack.into_any_element()
    }

    fn render_dx_agents_social_items(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut stack = v_flex()
            .gap_1()
            .child(Label::new("Social Accounts").size(LabelSize::Small));

        if snapshot.social_accounts.is_empty() {
            stack = stack.child(
                Label::new("No social receipt yet. Run the bridge refresh or social list command.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else {
            for account in snapshot.social_accounts.iter().take(4) {
                let state = if account.connected {
                    "connected"
                } else if account.qr_connect_supported {
                    "qr ready"
                } else if account.configured {
                    "configured"
                } else {
                    "needs setup"
                };
                let ready_action_count = account
                    .actions
                    .iter()
                    .filter(|action| action.enabled)
                    .count();
                let mut item = AiSettingItem::new(
                    format!("dx-agent-social-{}", account.platform),
                    account.label.clone(),
                    if account.connected {
                        AiSettingItemStatus::Running
                    } else {
                        AiSettingItemStatus::Stopped
                    },
                    AiSettingItemSource::Custom,
                )
                .icon(
                    Icon::new(IconName::Link)
                        .size(IconSize::Small)
                        .color(Color::Muted),
                )
                .detail_label(format!(
                    "{} - {} - {} ready action(s) - {}",
                    account.platform, state, ready_action_count, account.next_action
                ));

                if snapshot.enabled && snapshot.cli_actions_allowed {
                    let refresh_action = dx_agent_row_action(&account.actions, "refresh");
                    let refresh_enabled = refresh_action.map_or(true, |action| action.enabled);
                    let refresh_tooltip =
                        dx_agent_action_tooltip(refresh_action, "Refresh redacted social receipt");
                    item = item.action(
                        IconButton::new(
                            format!("dx-agent-social-list-{}", account.platform),
                            IconName::RotateCw,
                        )
                        .icon_color(Color::Muted)
                        .icon_size(IconSize::Small)
                        .disabled(!refresh_enabled)
                        .tooltip(Tooltip::text(refresh_tooltip))
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.run_dx_agents_bridge_action(
                                vec!["agents", "social", "list", "--json"],
                                cx,
                            );
                        })),
                    );

                    let platform = account.platform.clone();
                    if account.connected {
                        let disconnect_action = dx_agent_row_action(&account.actions, "disconnect");
                        let disconnect_enabled =
                            disconnect_action.map_or(true, |action| action.enabled);
                        let disconnect_tooltip = dx_agent_action_tooltip(
                            disconnect_action,
                            "Prepare redacted disconnect/revoke receipt",
                        );
                        item = item.action(
                            IconButton::new(
                                format!("dx-agent-social-disconnect-{}", account.platform),
                                IconName::Trash,
                            )
                            .icon_color(Color::Muted)
                            .icon_size(IconSize::Small)
                            .disabled(!disconnect_enabled)
                            .tooltip(Tooltip::text(disconnect_tooltip))
                            .on_click(cx.listener(
                                move |this, _, _window, cx| {
                                    this.run_dx_agents_bridge_action(
                                        vec![
                                            "agents".to_string(),
                                            "social".to_string(),
                                            "disconnect".to_string(),
                                            "--platform".to_string(),
                                            platform.clone(),
                                            "--json".to_string(),
                                        ],
                                        cx,
                                    );
                                },
                            )),
                        );
                    } else {
                        let connect_action = dx_agent_row_action(&account.actions, "connect");
                        let connect_enabled = connect_action.map_or(true, |action| action.enabled);
                        let connect_tooltip = dx_agent_action_tooltip(
                            connect_action,
                            "Prepare redacted connect/QR receipt",
                        );
                        item = item.action(
                            IconButton::new(
                                format!("dx-agent-social-connect-{}", account.platform),
                                IconName::Link,
                            )
                            .icon_color(Color::Muted)
                            .icon_size(IconSize::Small)
                            .disabled(!connect_enabled)
                            .tooltip(Tooltip::text(connect_tooltip))
                            .on_click(cx.listener(
                                move |this, _, _window, cx| {
                                    this.run_dx_agents_bridge_action(
                                        vec![
                                            "agents".to_string(),
                                            "social".to_string(),
                                            "connect".to_string(),
                                            "--platform".to_string(),
                                            platform.clone(),
                                            "--json".to_string(),
                                        ],
                                        cx,
                                    );
                                },
                            )),
                        );
                    }
                }

                stack = stack.child(item);
                if let Some(action_summary) = dx_agent_action_summary(&account.actions) {
                    stack = stack.child(
                        Label::new(action_summary)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    );
                }
            }

            if snapshot.social_connect.present {
                stack = stack
                    .child(self.render_dx_agents_social_action_receipt(&snapshot.social_connect));
            }

            if snapshot.social_disconnect.present {
                stack = stack.child(
                    self.render_dx_agents_social_action_receipt(&snapshot.social_disconnect),
                );
            }
        }

        stack.into_any_element()
    }

    fn render_dx_agents_automation_items(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut stack = v_flex()
            .gap_1()
            .child(Label::new("Automations").size(LabelSize::Small));

        if snapshot.automations.is_empty() {
            stack = stack.child(
                Label::new(
                    "No automation receipt yet. Run the bridge refresh or automation list command.",
                )
                .size(LabelSize::Small)
                .color(Color::Muted),
            );
        } else {
            for automation in snapshot.automations.iter().take(4) {
                let ready_action_count = automation
                    .actions
                    .iter()
                    .filter(|action| action.enabled)
                    .count();
                let detail = format!(
                    "{} - {} - {} - {} ready action(s) - {}",
                    automation.source,
                    automation.schedule_kind,
                    automation.status,
                    ready_action_count,
                    automation.next_action
                );
                let mut item = AiSettingItem::new(
                    format!("dx-agent-automation-{}", automation.id),
                    automation.id.clone(),
                    if automation.enabled {
                        AiSettingItemStatus::Running
                    } else {
                        AiSettingItemStatus::Stopped
                    },
                    AiSettingItemSource::Custom,
                )
                .icon(
                    Icon::new(IconName::ListTodo)
                        .size(IconSize::Small)
                        .color(Color::Muted),
                )
                .detail_label(detail);

                if snapshot.enabled && snapshot.cli_actions_allowed {
                    let refresh_action = dx_agent_row_action(&automation.actions, "refresh");
                    let refresh_enabled = refresh_action.map_or(true, |action| action.enabled);
                    let refresh_tooltip = dx_agent_action_tooltip(
                        refresh_action,
                        "Refresh redacted automation receipt",
                    );
                    item = item.action(
                        IconButton::new(
                            format!("dx-agent-automation-refresh-{}", automation.id),
                            IconName::RotateCw,
                        )
                        .icon_color(Color::Muted)
                        .icon_size(IconSize::Small)
                        .disabled(!refresh_enabled)
                        .tooltip(Tooltip::text(refresh_tooltip))
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.run_dx_agents_bridge_action(
                                vec!["agents", "automate", "list", "--json"],
                                cx,
                            );
                        })),
                    );

                    let run_action = dx_agent_row_action(&automation.actions, "run");
                    let run_enabled =
                        run_action.map_or(automation.enabled, |action| action.enabled);
                    let run_tooltip = dx_agent_action_tooltip(
                        run_action,
                        "Write redacted automation run receipt",
                    );
                    item = item.action(
                        IconButton::new(
                            format!("dx-agent-automation-run-{}", automation.id),
                            IconName::PlayOutlined,
                        )
                        .icon_color(Color::Muted)
                        .icon_size(IconSize::Small)
                        .disabled(!run_enabled)
                        .tooltip(Tooltip::text(run_tooltip))
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.run_dx_agents_bridge_action(vec!["agents", "run", "--json"], cx);
                        })),
                    );
                }

                stack = stack.child(item);
                if let Some(action_summary) = dx_agent_action_summary(&automation.actions) {
                    stack = stack.child(
                        Label::new(action_summary)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    );
                }
            }
        }

        stack.into_any_element()
    }

    fn render_dx_agents_receipt_items(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let index = &snapshot.receipt_index;
        let unsafe_count = snapshot
            .receipts
            .iter()
            .filter(|receipt| !receipt.safe_to_render)
            .count();
        let redacted_count = snapshot
            .receipts
            .iter()
            .filter(|receipt| receipt.metadata_redacted)
            .count();
        let status = if !index.present || index.last_error.is_some() || unsafe_count > 0 {
            AiSettingItemStatus::Stopped
        } else if index.status == "warning" || index.active_task_count > 0 {
            AiSettingItemStatus::Starting
        } else {
            AiSettingItemStatus::Running
        };
        let mut index_item = AiSettingItem::new(
            "dx-agents-receipt-index",
            "Background Receipts",
            status,
            AiSettingItemSource::Custom,
        )
        .icon(
            Icon::new(IconName::FileTextOutlined)
                .size(IconSize::Small)
                .color(Color::Muted),
        )
        .detail_label(format!(
            "{} returned / {} known, {} active, {} redacted",
            index.returned_receipt_count,
            index.receipt_count,
            index.active_task_count,
            redacted_count
        ));

        if snapshot.enabled && snapshot.cli_actions_allowed {
            index_item = index_item.action(
                IconButton::new("dx-agents-receipts-list", IconName::RotateCw)
                    .icon_color(Color::Muted)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Refresh DX Agents receipt index"))
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.run_dx_agents_bridge_action(
                            vec!["agents", "receipts", "list", "--json"],
                            cx,
                        );
                    })),
            );
        }

        let mut stack = v_flex()
            .gap_1()
            .child(Label::new("Background Receipts").size(LabelSize::Small))
            .child(index_item);

        if !index.present {
            stack = stack.child(
                Label::new("No receipt index yet. Run the receipt refresh action.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if let Some(error) = index.last_error.as_ref() {
            stack = stack.child(
                Label::new(format!("Index error: {error}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else {
            if let Some(path) = index.latest_receipt_path.as_ref() {
                stack = stack.child(
                    Label::new(format!("Latest: {path}"))
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                );
            }
            stack = stack.child(
                Label::new(format!("Next: {}", index.next_action))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        if snapshot.receipts.is_empty() {
            stack = stack.child(
                Label::new("No renderable receipt rows in the latest receipt index.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else {
            for receipt in snapshot.receipts.iter().take(4) {
                stack = stack.child(self.render_dx_agents_receipt_row(receipt));
            }
        }

        stack.into_any_element()
    }

    fn render_dx_agents_receipt_row(&self, receipt: &DxAgentReceipt) -> impl IntoElement {
        let status = if !receipt.safe_to_render || receipt.last_error.is_some() {
            AiSettingItemStatus::Stopped
        } else if receipt.active_task {
            AiSettingItemStatus::Starting
        } else {
            AiSettingItemStatus::Running
        };
        let redaction = if receipt.metadata_redacted {
            "redacted"
        } else {
            "metadata"
        };
        let mut stack = v_flex().gap_0p5().child(
            AiSettingItem::new(
                format!("dx-agent-receipt-{}", receipt.id),
                receipt.id.clone(),
                status,
                AiSettingItemSource::Custom,
            )
            .icon(
                Icon::new(IconName::FileTextOutlined)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .detail_label(format!(
                "{} - {} - {} bytes - {}",
                receipt.kind, receipt.status, receipt.size_bytes, redaction
            )),
        );

        if !receipt.schema_version.is_empty() {
            stack = stack.child(
                Label::new(format!("Schema: {}", receipt.schema_version))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }
        if receipt.safe_to_render && !receipt.command.is_empty() {
            stack = stack.child(
                Label::new(format!("Command: {}", receipt.command))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }
        if !receipt.task_id.is_empty() {
            stack = stack.child(
                Label::new(format!("Task: {}", receipt.task_id))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }
        if !receipt.receipt_path.is_empty() {
            stack = stack.child(
                Label::new(receipt.receipt_path.clone())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }
        if let Some(error) = receipt.last_error.as_ref() {
            stack = stack.child(
                Label::new(format!("Error: {error}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        } else if !receipt.next_action.is_empty() {
            stack = stack.child(
                Label::new(format!("Next: {}", receipt.next_action))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }
        if !receipt.generated_at.is_empty() || !receipt.modified_at.is_empty() {
            stack = stack.child(
                Label::new(format!(
                    "Generated {} - modified {}",
                    receipt.generated_at, receipt.modified_at
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            );
        }

        stack.into_any_element()
    }

    fn render_dx_agents_social_action_receipt(
        &self,
        receipt: &DxAgentSocialActionSummary,
    ) -> impl IntoElement {
        let connected = if receipt.connected.unwrap_or(false) {
            "connected"
        } else {
            "not connected"
        };
        let explicit_action = if receipt.explicit_user_action_required {
            "user action required"
        } else {
            "no user action required"
        };
        let detail = if receipt.action == "connect" {
            let support = if receipt.connect_supported {
                "supported"
            } else {
                "unsupported"
            };
            let qr = if receipt.qr_supported {
                "QR ready"
            } else {
                "QR unavailable"
            };
            let link = if receipt.link_supported {
                "link ready"
            } else {
                "link unavailable"
            };
            format!(
                "{} ({}) connect {}, via {}, {}, {}, {}, {}, config {}",
                receipt.label,
                receipt.platform,
                support,
                receipt.connect_method,
                qr,
                link,
                connected,
                explicit_action,
                receipt.safe_config_state
            )
        } else {
            let support = if receipt.disconnect_supported {
                "supported"
            } else {
                "not needed"
            };
            let revoke = if receipt.manual_revoke_required {
                "provider revoke required"
            } else {
                "no provider revoke required"
            };
            format!(
                "{} ({}) disconnect {}, {}, {}, {}, config {}",
                receipt.label,
                receipt.platform,
                support,
                revoke,
                connected,
                explicit_action,
                receipt.safe_config_state
            )
        };

        v_flex()
            .gap_0p5()
            .child(
                Label::new(format!(
                    "Last {} receipt: {} - {}",
                    receipt.action, receipt.status, detail
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            )
            .child(
                Label::new(format!("Next: {}", receipt.next_action))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .into_any_element()
    }

    fn render_dx_agents_catalog_items(
        &self,
        snapshot: &DxAgentBridgeSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if !snapshot.show_managed_providers {
            return v_flex()
                .gap_1()
                .child(Label::new("Provider Catalog").size(LabelSize::Small))
                .child(
                    Label::new("Managed provider rows are hidden by DX Agents settings.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element();
        }

        let cache_state = if snapshot.catalog.present && !snapshot.catalog.stale {
            "fast cache ready"
        } else if snapshot.catalog.present {
            "cache stale"
        } else {
            "cache missing"
        };
        let provider_detail = format!(
            "{} provider(s), {} model(s), {}",
            snapshot.catalog.provider_count, snapshot.catalog.model_count, cache_state
        );
        let active_provider = snapshot
            .providers
            .iter()
            .find(|provider| provider.active)
            .map(|provider| provider.display_name.clone())
            .unwrap_or_else(|| "No active DX provider".to_string());

        let catalog_status = if snapshot.catalog.present && !snapshot.catalog.stale {
            AiSettingItemStatus::Running
        } else {
            AiSettingItemStatus::Starting
        };
        let mut catalog_item = AiSettingItem::new(
            "dx-agents-provider-catalog",
            "Managed Providers",
            catalog_status,
            AiSettingItemSource::Custom,
        )
        .icon(
            Icon::new(IconName::Sliders)
                .size(IconSize::Small)
                .color(Color::Muted),
        )
        .detail_label(provider_detail);

        if snapshot.enabled && snapshot.cli_actions_allowed {
            catalog_item = catalog_item
                .action(
                    IconButton::new("dx-agents-providers-list", IconName::RotateCw)
                        .icon_color(Color::Muted)
                        .icon_size(IconSize::Small)
                        .tooltip(Tooltip::text("Refresh DX provider receipt"))
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.run_dx_agents_bridge_action(
                                vec!["providers", "list", "--json"],
                                cx,
                            );
                        })),
                )
                .action(
                    IconButton::new("dx-agents-models-list", IconName::ListTodo)
                        .icon_color(Color::Muted)
                        .icon_size(IconSize::Small)
                        .tooltip(Tooltip::text("Refresh DX model receipt"))
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.run_dx_agents_bridge_action(vec!["models", "list", "--json"], cx);
                        })),
                )
                .action(
                    IconButton::new("dx-agents-provider-catalog-regenerate", IconName::Download)
                        .icon_color(Color::Muted)
                        .icon_size(IconSize::Small)
                        .tooltip(Tooltip::text("Regenerate DX provider catalog receipt"))
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.run_dx_agents_bridge_action(
                                vec!["providers", "catalog", "regenerate", "--json"],
                                cx,
                            );
                        })),
                );
        }

        let mut stack = v_flex()
            .gap_1()
            .child(Label::new("Provider Catalog").size(LabelSize::Small))
            .child(catalog_item)
            .child(
                Label::new(active_provider)
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .child(
                Label::new(snapshot.catalog.path.display().to_string())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );

        if let Some(source_hash) = snapshot.catalog.source_hash.as_ref() {
            stack = stack.child(
                Label::new(format!("Source hash: {source_hash}"))
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            );
        }

        stack
            .child(
                Label::new(format!(
                    "Refresh command: {}",
                    snapshot.catalog.safe_regeneration_command
                ))
                .size(LabelSize::Small)
                .color(Color::Muted),
            )
            .into_any_element()
    }

    fn run_dx_agents_bridge_action<T>(&mut self, args: Vec<T>, cx: &mut Context<Self>)
    where
        T: Into<String>,
    {
        if !dx_agent_cli_actions_allowed(cx) {
            return;
        }

        let cli_path = dx_agent_cli_path(cx);
        let dx_home = dx_agent_dx_home(cx);
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let task =
            cx.background_spawn(async move { run_dx_agent_command(cli_path, args, dx_home) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |_this, cx| {
                if let Err(error) = result {
                    log::warn!("DX Agents bridge action failed: {error}");
                }
                cx.notify();
            })
            .log_err();
        })
        .detach();
    }

    fn run_dx_agents_metadata_action<T>(&mut self, args: Vec<T>, cx: &mut Context<Self>)
    where
        T: Into<String>,
    {
        enum MetadataAction {
            ImportSummary,
            ReleaseGate,
        }

        if !dx_agent_cli_actions_allowed(cx) {
            return;
        }

        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let action = if args.len() == 3
            && args[0] == "agents"
            && args[1] == "import-summary"
            && args[2] == "--json"
        {
            MetadataAction::ImportSummary
        } else if args.len() == 3
            && args[0] == "agents"
            && args[1] == "release-gate"
            && args[2] == "--json"
        {
            MetadataAction::ReleaseGate
        } else {
            log::warn!("Unsupported DX Agents metadata action: {}", args.join(" "));
            return;
        };

        let dx_home = dx_agent_dx_home(cx);
        let receipt_root = dx_agent_receipt_root(cx);
        let task = cx.background_spawn(async move {
            match action {
                MetadataAction::ImportSummary => {
                    run_dx_agent_import_summary_command(dx_home, receipt_root)
                }
                MetadataAction::ReleaseGate => {
                    run_dx_agent_release_gate_command(dx_home, receipt_root)
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |_this, cx| {
                if let Err(error) = result {
                    log::warn!("DX Agents metadata action failed: {error}");
                }
                cx.notify();
            })
            .log_err();
        })
        .detach();
    }

    fn render_provider_configuration_block(
        &mut self,
        provider: &Arc<dyn LanguageModelProvider>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let provider_id = provider.id().0;
        let provider_name = provider.name().0;
        let provider_id_string = SharedString::from(format!("provider-disclosure-{provider_id}"));
        let model_count = provider.provided_models(cx).len();

        let configuration_view = self
            .configuration_views_by_provider
            .get(&provider.id())
            .cloned();

        let is_expanded = self
            .expanded_provider_configurations
            .get(&provider.id())
            .copied()
            .unwrap_or(false);

        let is_zed_provider = provider.id() == ZED_CLOUD_PROVIDER_ID;
        let current_plan = if is_zed_provider {
            self.workspace
                .upgrade()
                .and_then(|workspace| workspace.read(cx).user_store().read(cx).plan())
        } else {
            None
        };

        let is_signed_in = self
            .workspace
            .read_with(cx, |workspace, _| {
                !workspace.client().status().borrow().is_signed_out()
            })
            .unwrap_or(false);

        v_flex()
            .min_w_0()
            .w_full()
            .when(is_expanded, |this| this.mb_2())
            .child(
                div()
                    .px_2()
                    .child(Divider::horizontal().color(DividerColor::BorderFaded)),
            )
            .child(
                h_flex()
                    .map(|this| {
                        if is_expanded {
                            this.mt_2().mb_1()
                        } else {
                            this.my_2()
                        }
                    })
                    .w_full()
                    .justify_between()
                    .child(
                        h_flex()
                            .id(provider_id_string.clone())
                            .cursor_pointer()
                            .px_2()
                            .py_0p5()
                            .w_full()
                            .justify_between()
                            .rounded_sm()
                            .hover(|hover| hover.bg(cx.theme().colors().element_hover))
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_1p5()
                                    .child(
                                        match provider.icon() {
                                            IconOrSvg::Svg(path) => Icon::from_external_svg(path),
                                            IconOrSvg::Icon(name) => Icon::new(name),
                                        }
                                        .size(IconSize::Small)
                                        .color(Color::Muted),
                                    )
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .gap_1()
                                            .child(Label::new(provider_name.clone()))
                                            .map(|this| {
                                                if is_zed_provider && is_signed_in {
                                                    this.child(
                                                        self.render_zed_plan_info(current_plan, cx),
                                                    )
                                                } else {
                                                    this.when(
                                                        provider.is_authenticated(cx)
                                                            && !is_expanded,
                                                        |parent| {
                                                            parent.child(
                                                                Icon::new(IconName::Check)
                                                                    .color(Color::Success),
                                                            )
                                                        },
                                                    )
                                                }
                                            }),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(
                                        Chip::new(format!("A {model_count}"))
                                            .height(px(18.))
                                            .label_size(LabelSize::XSmall)
                                            .tooltip(Tooltip::text(format!(
                                                "{model_count} models supported by {provider_name}",
                                            ))),
                                    )
                                    .child(
                                        Disclosure::new(provider_id_string, is_expanded)
                                            .opened_icon(IconName::ChevronDown)
                                            .closed_icon(IconName::ChevronRight),
                                    ),
                            )
                            .on_click(cx.listener({
                                let provider_id = provider.id();
                                move |this, _event, _window, _cx| {
                                    let is_expanded = this
                                        .expanded_provider_configurations
                                        .entry(provider_id.clone())
                                        .or_insert(false);

                                    *is_expanded = !*is_expanded;
                                }
                            })),
                    ),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .w_full()
                    .px_2()
                    .gap_1()
                    .when(is_expanded, |parent| match configuration_view {
                        Some(configuration_view) => parent.child(configuration_view),
                        None => parent.child(Label::new(format!(
                            "No configuration view for {provider_name}",
                        ))),
                    })
                    .when(is_expanded && provider.is_authenticated(cx), |parent| {
                        parent.child(
                            Button::new(
                                SharedString::from(format!("new-thread-{provider_id}")),
                                "Start New Thread",
                            )
                            .full_width()
                            .style(ButtonStyle::Outlined)
                            .layer(ElevationIndex::ModalSurface)
                            .start_icon(
                                Icon::new(IconName::Thread)
                                    .size(IconSize::Small)
                                    .color(Color::Muted),
                            )
                            .label_size(LabelSize::Small)
                            .on_click(cx.listener({
                                let provider = provider.clone();
                                move |_this, _event, _window, cx| {
                                    cx.emit(AssistantConfigurationEvent::NewThread(
                                        provider.clone(),
                                    ))
                                }
                            })),
                        )
                    })
                    .when(
                        is_expanded && is_removable_provider(&provider.id(), cx),
                        |this| {
                            this.child(
                                Button::new(
                                    SharedString::from(format!("delete-provider-{provider_id}")),
                                    "Remove Provider",
                                )
                                .full_width()
                                .style(ButtonStyle::Outlined)
                                .start_icon(
                                    Icon::new(IconName::Trash)
                                        .size(IconSize::Small)
                                        .color(Color::Muted),
                                )
                                .label_size(LabelSize::Small)
                                .on_click(cx.listener({
                                    let provider = provider.clone();
                                    move |this, _event, window, cx| {
                                        this.delete_provider(provider.clone(), window, cx);
                                    }
                                })),
                            )
                        },
                    ),
            )
    }

    fn delete_provider(
        &mut self,
        provider: Arc<dyn LanguageModelProvider>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let fs = self.fs.clone();
        let provider_id = provider.id();

        cx.spawn_in(window, async move |_, cx| {
            cx.update(|_window, cx| {
                update_settings_file(fs.clone(), cx, {
                    let provider_id = provider_id.clone();
                    move |settings, _| {
                        if let Some(ref mut openai_compatible) = settings
                            .language_models
                            .as_mut()
                            .and_then(|lm| lm.openai_compatible.as_mut())
                        {
                            let key_to_remove: Arc<str> = Arc::from(provider_id.0.as_ref());
                            openai_compatible.remove(&key_to_remove);
                        }
                    }
                });
            })
            .log_err();

            cx.update(|_window, cx| {
                LanguageModelRegistry::global(cx).update(cx, {
                    let provider_id = provider_id.clone();
                    move |registry, cx| {
                        registry.unregister_provider(provider_id, cx);
                    }
                })
            })
            .log_err();

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn render_provider_configuration_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let providers = LanguageModelRegistry::read_global(cx).visible_providers();

        let popover_menu = PopoverMenu::new("add-provider-popover")
            .trigger(
                Button::new("add-provider", "Add Provider")
                    .style(ButtonStyle::Outlined)
                    .start_icon(
                        Icon::new(IconName::Plus)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
                    .label_size(LabelSize::Small),
            )
            .menu({
                let workspace = self.workspace.clone();
                move |window, cx| {
                    Some(ContextMenu::build(window, cx, |menu, _window, _cx| {
                        menu.header("Compatible APIs").entry("OpenAI", None, {
                            let workspace = workspace.clone();
                            move |window, cx| {
                                workspace
                                    .update(cx, |workspace, cx| {
                                        AddLlmProviderModal::toggle(
                                            LlmCompatibleProvider::OpenAi,
                                            workspace,
                                            window,
                                            cx,
                                        );
                                    })
                                    .log_err();
                            }
                        })
                    }))
                }
            })
            .anchor(gpui::Anchor::TopRight)
            .offset(gpui::Point {
                x: px(0.0),
                y: px(2.0),
            });

        v_flex()
            .min_w_0()
            .w_full()
            .child(self.render_section_title(
                "LLM Providers",
                "Add at least one provider to use AI-powered features with Zed's native agent.",
                popover_menu.into_any_element(),
            ))
            .child(
                div()
                    .w_full()
                    .pl(DynamicSpacing::Base08.rems(cx))
                    .pr(DynamicSpacing::Base20.rems(cx))
                    .children(
                        providers.into_iter().map(|provider| {
                            self.render_provider_configuration_block(&provider, cx)
                        }),
                    ),
            )
    }

    fn render_zed_plan_info(&self, plan: Option<Plan>, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(plan) = plan {
            let free_chip_bg = cx
                .theme()
                .colors()
                .editor_background
                .opacity(0.5)
                .blend(cx.theme().colors().text_accent.opacity(0.05));

            let pro_chip_bg = cx
                .theme()
                .colors()
                .editor_background
                .opacity(0.5)
                .blend(cx.theme().colors().text_accent.opacity(0.2));

            let (plan_name, label_color, bg_color) = match plan {
                Plan::ZedFree => ("Free", Color::Default, free_chip_bg),
                Plan::ZedProTrial => ("Pro Trial", Color::Accent, pro_chip_bg),
                Plan::ZedPro => ("Pro", Color::Accent, pro_chip_bg),
                Plan::ZedBusiness => ("Business", Color::Accent, pro_chip_bg),
                Plan::ZedStudent => ("Student", Color::Accent, pro_chip_bg),
            };

            Chip::new(plan_name.to_string())
                .bg_color(bg_color)
                .label_color(label_color)
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    fn render_context_servers_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let context_server_ids = self.context_server_store.read(cx).server_ids();

        let add_server_popover = PopoverMenu::new("add-server-popover")
            .trigger(
                Button::new("add-server", "Add Server")
                    .style(ButtonStyle::Outlined)
                    .start_icon(
                        Icon::new(IconName::Plus)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
                    .label_size(LabelSize::Small),
            )
            .menu({
                move |window, cx| {
                    Some(ContextMenu::build(window, cx, |menu, _window, _cx| {
                        menu.entry("Add Custom Server", None, {
                            |window, cx| {
                                window.dispatch_action(crate::AddContextServer.boxed_clone(), cx)
                            }
                        })
                        .entry("Install from Extensions", None, {
                            |window, cx| {
                                window.dispatch_action(
                                    zed_actions::Extensions {
                                        category_filter: Some(
                                            ExtensionCategoryFilter::ContextServers,
                                        ),
                                        id: None,
                                    }
                                    .boxed_clone(),
                                    cx,
                                )
                            }
                        })
                    }))
                }
            })
            .anchor(gpui::Anchor::TopRight)
            .offset(gpui::Point {
                x: px(0.0),
                y: px(2.0),
            });

        v_flex()
            .min_w_0()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(self.render_section_title(
                "Model Context Protocol (MCP) Servers",
                "All MCP servers connected directly or via a Zed extension.",
                add_server_popover.into_any_element(),
            ))
            .child(
                v_flex()
                    .pl_4()
                    .pb_4()
                    .pr_5()
                    .w_full()
                    .gap_1()
                    .map(|parent| {
                        if context_server_ids.is_empty() {
                            parent.child(
                                h_flex()
                                    .p_4()
                                    .justify_center()
                                    .border_1()
                                    .border_dashed()
                                    .border_color(cx.theme().colors().border.opacity(0.6))
                                    .rounded_sm()
                                    .child(
                                        Label::new("No MCP servers added yet.")
                                            .color(Color::Muted)
                                            .size(LabelSize::Small),
                                    ),
                            )
                        } else {
                            parent.children(itertools::intersperse_with(
                                context_server_ids.iter().cloned().map(|context_server_id| {
                                    self.render_context_server(context_server_id, cx)
                                        .into_any_element()
                                }),
                                || {
                                    Divider::horizontal()
                                        .color(DividerColor::BorderFaded)
                                        .into_any_element()
                                },
                            ))
                        }
                    }),
            )
    }

    fn render_context_server(
        &self,
        context_server_id: ContextServerId,
        cx: &Context<Self>,
    ) -> impl use<> + IntoElement {
        let server_status = self
            .context_server_store
            .read(cx)
            .status_for_server(&context_server_id)
            .unwrap_or(ContextServerStatus::Stopped);
        let server_configuration = self
            .context_server_store
            .read(cx)
            .configuration_for_server(&context_server_id);

        let is_running = matches!(server_status, ContextServerStatus::Running);
        let item_id = SharedString::from(context_server_id.0.clone());
        // Servers without a configuration can only be provided by extensions.
        let provided_by_extension = server_configuration.as_ref().is_none_or(|config| {
            matches!(
                config.as_ref(),
                ContextServerConfiguration::Extension { .. }
            )
        });

        let display_name = if provided_by_extension {
            resolve_extension_for_context_server(&context_server_id, cx)
                .map(|(_, manifest)| {
                    let name = manifest.name.as_str();
                    let stripped = name
                        .strip_suffix(" MCP Server")
                        .or_else(|| name.strip_suffix(" MCP"))
                        .or_else(|| name.strip_suffix(" Context Server"))
                        .unwrap_or(name);
                    SharedString::from(stripped.to_string())
                })
                .unwrap_or_else(|| item_id.clone())
        } else {
            item_id.clone()
        };

        let error = if let ContextServerStatus::Error(error) = server_status.clone() {
            Some(error)
        } else {
            None
        };
        let auth_required = matches!(server_status, ContextServerStatus::AuthRequired);
        let client_secret_required = matches!(
            server_status,
            ContextServerStatus::ClientSecretRequired { .. }
        );
        let authenticating = matches!(server_status, ContextServerStatus::Authenticating);
        let context_server_store = self.context_server_store.clone();
        let workspace = self.workspace.clone();
        let language_registry = self.language_registry.clone();

        let tool_count = self
            .context_server_registry
            .read(cx)
            .tools_for_server(&context_server_id)
            .count();

        let source = if provided_by_extension {
            AiSettingItemSource::Extension
        } else {
            AiSettingItemSource::Custom
        };

        let status = match server_status {
            ContextServerStatus::Starting => AiSettingItemStatus::Starting,
            ContextServerStatus::Running => AiSettingItemStatus::Running,
            ContextServerStatus::Error(_) => AiSettingItemStatus::Error,
            ContextServerStatus::Stopped => AiSettingItemStatus::Stopped,
            ContextServerStatus::AuthRequired => AiSettingItemStatus::AuthRequired,
            ContextServerStatus::ClientSecretRequired { .. } => {
                AiSettingItemStatus::ClientSecretRequired
            }
            ContextServerStatus::Authenticating => AiSettingItemStatus::Authenticating,
        };

        let is_remote = server_configuration
            .as_ref()
            .map(|config| matches!(config.as_ref(), ContextServerConfiguration::Http { .. }))
            .unwrap_or(false);

        let should_show_logout_button = server_configuration.as_ref().is_some_and(|config| {
            matches!(config.as_ref(), ContextServerConfiguration::Http { .. })
                && !config.has_static_auth_header()
        });

        let context_server_configuration_menu = PopoverMenu::new("context-server-config-menu")
            .trigger_with_tooltip(
                IconButton::new("context-server-config-menu", IconName::Settings)
                    .icon_color(Color::Muted)
                    .icon_size(IconSize::Small),
                Tooltip::text("Configure MCP Server"),
            )
            .anchor(Anchor::TopRight)
            .menu({
                let fs = self.fs.clone();
                let context_server_id = context_server_id.clone();
                let language_registry = self.language_registry.clone();
                let workspace = self.workspace.clone();
                let context_server_registry = self.context_server_registry.clone();
                let context_server_store = context_server_store.clone();

                move |window, cx| {
                    Some(ContextMenu::build(window, cx, |menu, _window, _cx| {
                        menu.entry("Configure Server", None, {
                            let context_server_id = context_server_id.clone();
                            let language_registry = language_registry.clone();
                            let workspace = workspace.clone();
                            move |window, cx| {
                                if is_remote {
                                    crate::agent_configuration::configure_context_server_modal::ConfigureContextServerModal::show_modal_for_existing_server(
                                        context_server_id.clone(),
                                        language_registry.clone(),
                                        workspace.clone(),
                                        window,
                                        cx,
                                    )
                                    .detach();
                                } else {
                                    ConfigureContextServerModal::show_modal_for_existing_server(
                                        context_server_id.clone(),
                                        language_registry.clone(),
                                        workspace.clone(),
                                        window,
                                        cx,
                                    )
                                    .detach();
                                }
                            }
                        }).when(tool_count > 0, |this| this.entry("View Tools", None, {
                            let context_server_id = context_server_id.clone();
                            let context_server_registry = context_server_registry.clone();
                            let workspace = workspace.clone();
                            move |window, cx| {
                                let context_server_id = context_server_id.clone();
                                workspace.update(cx, |workspace, cx| {
                                    ConfigureContextServerToolsModal::toggle(
                                        context_server_id,
                                        context_server_registry.clone(),
                                        workspace,
                                        window,
                                        cx,
                                    );
                                })
                                .ok();
                            }
                        }))
                        .when(should_show_logout_button, |this| {
                            this.entry("Log Out", None, {
                                let context_server_store = context_server_store.clone();
                                let context_server_id = context_server_id.clone();
                                move |_window, cx| {
                                    context_server_store.update(cx, |store, cx| {
                                        store.logout_server(&context_server_id, cx).log_err();
                                    });
                                }
                            })
                        })
                        .separator()
                        .entry("Uninstall", None, {
                            let fs = fs.clone();
                            let context_server_id = context_server_id.clone();
                            let workspace = workspace.clone();
                            move |_, cx| {
                                let uninstall_extension_task = match (
                                    provided_by_extension,
                                    resolve_extension_for_context_server(&context_server_id, cx),
                                ) {
                                    (true, Some((id, manifest))) => {
                                        if extension_only_provides_context_server(manifest.as_ref())
                                        {
                                            ExtensionStore::global(cx).update(cx, |store, cx| {
                                                store.uninstall_extension(id, cx)
                                            })
                                        } else {
                                            workspace.update(cx, |workspace, cx| {
                                                show_unable_to_uninstall_extension_with_context_server(workspace, context_server_id.clone(), cx);
                                            }).log_err();
                                            Task::ready(Ok(()))
                                        }
                                    }
                                    _ => Task::ready(Ok(())),
                                };

                                cx.spawn({
                                    let fs = fs.clone();
                                    let context_server_id = context_server_id.clone();
                                    async move |cx| {
                                        uninstall_extension_task.await?;
                                        cx.update(|cx| {
                                            update_settings_file(
                                                fs.clone(),
                                                cx,
                                                {
                                                    let context_server_id =
                                                        context_server_id.clone();
                                                    move |settings, _| {
                                                        settings.project
                                                            .context_servers
                                                            .remove(&context_server_id.0);
                                                    }
                                                },
                                            )
                                        });
                                        anyhow::Ok(())
                                    }
                                })
                                .detach_and_log_err(cx);
                            }
                        })
                    }))
                }
            });

        let feedback_base_container =
            || h_flex().py_1().min_w_0().w_full().gap_1().justify_between();

        let details: Option<AnyElement> = if let Some(error) = error {
            Some(
                feedback_base_container()
                    .child(
                        h_flex()
                            .pr_4()
                            .min_w_0()
                            .w_full()
                            .gap_2()
                            .child(
                                Icon::new(IconName::XCircle)
                                    .size(IconSize::XSmall)
                                    .color(Color::Error),
                            )
                            .child(div().min_w_0().flex_1().child(
                                Label::new(error).color(Color::Muted).size(LabelSize::Small),
                            )),
                    )
                    .when(should_show_logout_button, |this| {
                        this.child(
                            Button::new("error-logout-server", "Log Out")
                                .style(ButtonStyle::Outlined)
                                .label_size(LabelSize::Small)
                                .on_click({
                                    let context_server_store = context_server_store.clone();
                                    let context_server_id = context_server_id.clone();
                                    move |_event, _window, cx| {
                                        context_server_store.update(cx, |store, cx| {
                                            store.logout_server(&context_server_id, cx).log_err();
                                        });
                                    }
                                }),
                        )
                    })
                    .into_any_element(),
            )
        } else if auth_required {
            Some(
                feedback_base_container()
                    .child(
                        h_flex()
                            .pr_4()
                            .min_w_0()
                            .w_full()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Info)
                                    .size(IconSize::XSmall)
                                    .color(Color::Muted),
                            )
                            .child(
                                Label::new("Authenticate to connect this server")
                                    .color(Color::Muted)
                                    .size(LabelSize::Small),
                            ),
                    )
                    .child(
                        Button::new("authenticate-server", "Authenticate")
                            .style(ButtonStyle::Outlined)
                            .label_size(LabelSize::Small)
                            .on_click({
                                let context_server_id = context_server_id.clone();
                                move |_event, _window, cx| {
                                    context_server_store.update(cx, |store, cx| {
                                        store.authenticate_server(&context_server_id, cx).log_err();
                                    });
                                }
                            }),
                    )
                    .into_any_element(),
            )
        } else if client_secret_required {
            Some(
                feedback_base_container()
                    .child(
                        h_flex()
                            .pr_4()
                            .min_w_0()
                            .w_full()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Info)
                                    .size(IconSize::XSmall)
                                    .color(Color::Muted),
                            )
                            .child(
                                Label::new("Enter a client secret to connect this server")
                                    .color(Color::Muted)
                                    .size(LabelSize::Small),
                            ),
                    )
                    .child(
                        Button::new("enter-client-secret", "Enter Client Secret")
                            .style(ButtonStyle::Outlined)
                            .label_size(LabelSize::Small)
                            .on_click({
                                let context_server_id = context_server_id.clone();
                                move |_event, window, cx| {
                                    ConfigureContextServerModal::show_modal_for_existing_server(
                                        context_server_id.clone(),
                                        language_registry.clone(),
                                        workspace.clone(),
                                        window,
                                        cx,
                                    )
                                    .detach();
                                }
                            }),
                    )
                    .into_any_element(),
            )
        } else if authenticating {
            Some(
                h_flex()
                    .mt_1()
                    .pr_4()
                    .min_w_0()
                    .w_full()
                    .gap_2()
                    .child(div().size_3().flex_shrink_0())
                    .child(
                        Label::new("Authenticating…")
                            .color(Color::Muted)
                            .size(LabelSize::Small),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let tool_label = if is_running {
            Some(if tool_count == 1 {
                SharedString::from("1 tool")
            } else {
                SharedString::from(format!("{} tools", tool_count))
            })
        } else {
            None
        };

        AiSettingItem::new(item_id, display_name, status, source)
            .action(context_server_configuration_menu)
            .action(
                Switch::new("context-server-switch", is_running.into()).on_click({
                    let context_server_manager = self.context_server_store.clone();
                    let fs = self.fs.clone();

                    move |state, _window, cx| {
                        let is_enabled = match state {
                            ToggleState::Unselected | ToggleState::Indeterminate => {
                                context_server_manager.update(cx, |this, cx| {
                                    this.stop_server(&context_server_id, cx).log_err();
                                });
                                false
                            }
                            ToggleState::Selected => {
                                context_server_manager.update(cx, |this, cx| {
                                    if let Some(server) = this.get_server(&context_server_id) {
                                        this.start_server(server, cx);
                                    }
                                });
                                true
                            }
                        };
                        update_settings_file(fs.clone(), cx, {
                            let context_server_id = context_server_id.clone();

                            move |settings, _| {
                                settings
                                    .project
                                    .context_servers
                                    .entry(context_server_id.0)
                                    .or_insert_with(|| {
                                        settings::ContextServerSettingsContent::Extension {
                                            enabled: is_enabled,
                                            remote: false,
                                            settings: serde_json::json!({}),
                                        }
                                    })
                                    .set_enabled(is_enabled);
                            }
                        });
                    }
                }),
            )
            .when_some(tool_label, |this, label| this.detail_label(label))
            .when_some(details, |this, details| this.details(details))
    }

    fn render_agent_servers_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let agent_server_store = self.agent_server_store.read(cx);

        let agents = agent_server_store
            .external_agents()
            .cloned()
            .collect::<Vec<_>>();

        let agents: Vec<_> = agents
            .into_iter()
            .map(|name| {
                let icon = if let Some(icon_path) = agent_server_store.agent_icon(&name) {
                    AgentIcon::Path(icon_path)
                } else {
                    AgentIcon::Name(IconName::Sparkle)
                };
                let display_name = agent_server_store
                    .agent_display_name(&name)
                    .unwrap_or_else(|| name.0.clone());
                let source = agent_server_store.agent_source(&name).unwrap_or_default();
                (name, icon, display_name, source)
            })
            .sorted_unstable_by_key(|(_, _, display_name, _)| display_name.to_lowercase())
            .collect();

        let add_agent_popover = PopoverMenu::new("add-agent-server-popover")
            .trigger(
                Button::new("add-agent", "Add Agent")
                    .style(ButtonStyle::Outlined)
                    .start_icon(
                        Icon::new(IconName::Plus)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
                    .label_size(LabelSize::Small),
            )
            .menu({
                move |window, cx| {
                    Some(ContextMenu::build(window, cx, |menu, _window, _cx| {
                        menu.entry("Install from Registry", None, {
                            |window, cx| {
                                window.dispatch_action(Box::new(zed_actions::AcpRegistry), cx)
                            }
                        })
                        .entry("Add Custom Agent", None, {
                            move |window, cx| {
                                if let Some(workspace) = Workspace::for_window(window, cx) {
                                    let workspace = workspace.downgrade();
                                    window
                                        .spawn(cx, async |cx| {
                                            open_new_agent_servers_entry_in_settings_editor(
                                                workspace, cx,
                                            )
                                            .await
                                        })
                                        .detach_and_log_err(cx);
                                }
                            }
                        })
                        .separator()
                        .header("Learn More")
                        .item(
                            ContextMenuEntry::new("ACP Docs")
                                .icon(IconName::ArrowUpRight)
                                .icon_color(Color::Muted)
                                .icon_position(IconPosition::End)
                                .handler({
                                    move |window, cx| {
                                        window.dispatch_action(
                                            Box::new(OpenBrowser {
                                                url: "https://agentclientprotocol.com/".into(),
                                            }),
                                            cx,
                                        );
                                    }
                                }),
                        )
                    }))
                }
            })
            .anchor(gpui::Anchor::TopRight)
            .offset(gpui::Point {
                x: px(0.0),
                y: px(2.0),
            });

        v_flex()
            .min_w_0()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                v_flex()
                    .child(self.render_section_title(
                        "External Agents",
                        "All agents connected through the Agent Client Protocol.",
                        add_agent_popover.into_any_element(),
                    ))
                    .child(
                        v_flex()
                            .p_4()
                            .pt_0()
                            .gap_2()
                            .children(Itertools::intersperse_with(
                                agents
                                    .into_iter()
                                    .map(|(name, icon, display_name, source)| {
                                        self.render_agent_server(
                                            icon,
                                            name,
                                            display_name,
                                            source,
                                            cx,
                                        )
                                        .into_any_element()
                                    }),
                                || {
                                    Divider::horizontal()
                                        .color(DividerColor::BorderFaded)
                                        .into_any_element()
                                },
                            )),
                    ),
            )
    }

    fn render_agent_server(
        &self,
        icon: AgentIcon,
        id: impl Into<SharedString>,
        display_name: impl Into<SharedString>,
        source: ExternalAgentSource,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = id.into();
        let display_name = display_name.into();

        let icon = match icon {
            AgentIcon::Name(icon_name) => Icon::new(icon_name)
                .size(IconSize::Small)
                .color(Color::Muted),
            AgentIcon::Path(icon_path) => Icon::from_external_svg(icon_path)
                .size(IconSize::Small)
                .color(Color::Muted),
        };

        let source_kind = match source {
            ExternalAgentSource::Extension => AiSettingItemSource::Extension,
            ExternalAgentSource::Registry => AiSettingItemSource::Registry,
            ExternalAgentSource::Custom => AiSettingItemSource::Custom,
        };

        let agent_server_name = AgentId(id.clone());
        let agent = Agent::Custom {
            id: agent_server_name.clone(),
        };

        let (connection_status, running_version) = {
            let connection_store = self.agent_connection_store.read(cx);
            (
                connection_store.connection_status(&agent, cx),
                connection_store.agent_version(&agent, cx),
            )
        };

        let restart_button = matches!(
            connection_status,
            AgentConnectionStatus::Connected | AgentConnectionStatus::Connecting
        )
        .then(|| {
            IconButton::new(
                SharedString::from(format!("restart-{}", id)),
                IconName::RotateCw,
            )
            .disabled(connection_status == AgentConnectionStatus::Connecting)
            .icon_color(Color::Muted)
            .icon_size(IconSize::Small)
            .tooltip(Tooltip::text("Restart Agent Connection"))
            .on_click(cx.listener({
                let agent = agent.clone();
                move |this, _, _window, cx| {
                    let server: Rc<dyn agent_servers::AgentServer> =
                        Rc::new(agent_servers::CustomAgentServer::new(agent.id()));
                    this.agent_connection_store.update(cx, |store, cx| {
                        store.restart_connection(agent.clone(), server, cx);
                    });
                }
            }))
        });

        let uninstall_button = match source {
            ExternalAgentSource::Extension => Some(
                IconButton::new(
                    SharedString::from(format!("uninstall-{}", id)),
                    IconName::Trash,
                )
                .icon_color(Color::Muted)
                .icon_size(IconSize::Small)
                .tooltip(Tooltip::text("Uninstall Agent Extension"))
                .on_click(cx.listener(move |this, _, _window, cx| {
                    let agent_name = agent_server_name.clone();

                    if let Some(ext_id) = this.agent_server_store.update(cx, |store, _cx| {
                        store.get_extension_id_for_agent(&agent_name)
                    }) {
                        ExtensionStore::global(cx)
                            .update(cx, |store, cx| store.uninstall_extension(ext_id, cx))
                            .detach_and_log_err(cx);
                    }
                })),
            ),
            ExternalAgentSource::Registry => {
                let fs = self.fs.clone();
                Some(
                    IconButton::new(
                        SharedString::from(format!("uninstall-{}", id)),
                        IconName::Trash,
                    )
                    .icon_color(Color::Muted)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Remove Registry Agent"))
                    .on_click(cx.listener(move |_, _, _window, cx| {
                        let agent_name = agent_server_name.clone();
                        update_settings_file(fs.clone(), cx, move |settings, _| {
                            let Some(agent_servers) = settings.agent_servers.as_mut() else {
                                return;
                            };
                            if let Some(entry) = agent_servers.get(agent_name.0.as_ref())
                                && matches!(
                                    entry,
                                    settings::CustomAgentServerSettings::Registry { .. }
                                )
                            {
                                agent_servers.remove(agent_name.0.as_ref());
                            }
                        });
                    })),
                )
            }
            ExternalAgentSource::Custom => {
                let fs = self.fs.clone();
                Some(
                    IconButton::new(
                        SharedString::from(format!("uninstall-{}", id)),
                        IconName::Trash,
                    )
                    .icon_color(Color::Muted)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Remove Custom Agent"))
                    .on_click(cx.listener(move |_, _, _window, cx| {
                        let agent_name = agent_server_name.clone();
                        update_settings_file(fs.clone(), cx, move |settings, _| {
                            let Some(agent_servers) = settings.agent_servers.as_mut() else {
                                return;
                            };
                            if let Some(entry) = agent_servers.get(agent_name.0.as_ref())
                                && matches!(
                                    entry,
                                    settings::CustomAgentServerSettings::Custom { .. }
                                )
                            {
                                agent_servers.remove(agent_name.0.as_ref());
                            }
                        });
                    })),
                )
            }
        };

        let status = match connection_status {
            AgentConnectionStatus::Disconnected => AiSettingItemStatus::Stopped,
            AgentConnectionStatus::Connecting => AiSettingItemStatus::Starting,
            AgentConnectionStatus::Connected => AiSettingItemStatus::Running,
        };

        AiSettingItem::new(id, display_name, status, source_kind)
            .icon(icon)
            .when_some(running_version, |this, version| this.detail_label(version))
            .when_some(restart_button, |this, button| this.action(button))
            .when_some(uninstall_button, |this, button| this.action(button))
    }
}

impl Render for AgentConfiguration {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("assistant-configuration")
            .key_context("AgentConfiguration")
            .track_focus(&self.focus_handle(cx))
            .relative()
            .size_full()
            .pb_8()
            .bg(cx.theme().colors().panel_background)
            .child(
                div()
                    .size_full()
                    .child(
                        v_flex()
                            .id("assistant-configuration-content")
                            .track_scroll(&self.scroll_handle)
                            .size_full()
                            .min_w_0()
                            .overflow_y_scroll()
                            .child(self.render_agent_servers_section(cx))
                            .child(self.render_dx_agents_section(cx))
                            .child(self.render_context_servers_section(cx))
                            .child(self.render_provider_configuration_section(cx)),
                    )
                    .vertical_scrollbar_for(&self.scroll_handle, window, cx),
            )
    }
}

fn extension_only_provides_context_server(manifest: &ExtensionManifest) -> bool {
    manifest.context_servers.len() == 1
        && manifest.themes.is_empty()
        && manifest.icon_themes.is_empty()
        && manifest.languages.is_empty()
        && manifest.grammars.is_empty()
        && manifest.language_servers.is_empty()
        && manifest.slash_commands.is_empty()
        && manifest.snippets.is_none()
        && manifest.debug_locators.is_empty()
}

pub(crate) fn resolve_extension_for_context_server(
    id: &ContextServerId,
    cx: &App,
) -> Option<(Arc<str>, Arc<ExtensionManifest>)> {
    ExtensionStore::global(cx)
        .read(cx)
        .installed_extensions()
        .iter()
        .find(|(_, entry)| entry.manifest.context_servers.contains_key(&id.0))
        .map(|(id, entry)| (id.clone(), entry.manifest.clone()))
}

// This notification appears when trying to delete
// an MCP server extension that not only provides
// the server, but other things, too, like language servers and more.
fn show_unable_to_uninstall_extension_with_context_server(
    workspace: &mut Workspace,
    id: ContextServerId,
    cx: &mut App,
) {
    let workspace_handle = workspace.weak_handle();
    let context_server_id = id.clone();

    let status_toast = StatusToast::new(
        format!(
            "The {} extension provides more than just the MCP server. Proceed to uninstall anyway?",
            id.0
        ),
        cx,
        move |this, _cx| {
            let workspace_handle = workspace_handle.clone();

            this.icon(
                Icon::new(IconName::Warning)
                    .size(IconSize::Small)
                    .color(Color::Warning),
            )
            .dismiss_button(true)
            .action("Uninstall", move |_, _cx| {
                if let Some((extension_id, _)) =
                    resolve_extension_for_context_server(&context_server_id, _cx)
                {
                    ExtensionStore::global(_cx).update(_cx, |store, cx| {
                        store
                            .uninstall_extension(extension_id, cx)
                            .detach_and_log_err(cx);
                    });

                    workspace_handle
                        .update(_cx, |workspace, cx| {
                            let fs = workspace.app_state().fs.clone();
                            cx.spawn({
                                let context_server_id = context_server_id.clone();
                                async move |_workspace_handle, cx| {
                                    cx.update(|cx| {
                                        update_settings_file(fs, cx, move |settings, _| {
                                            settings
                                                .project
                                                .context_servers
                                                .remove(&context_server_id.0);
                                        });
                                    });
                                    anyhow::Ok(())
                                }
                            })
                            .detach_and_log_err(cx);
                        })
                        .log_err();
                }
            })
        },
    );

    workspace.toggle_status_toast(status_toast, cx);
}

async fn open_new_agent_servers_entry_in_settings_editor(
    workspace: WeakEntity<Workspace>,
    cx: &mut AsyncWindowContext,
) -> Result<()> {
    let settings_editor = workspace
        .update_in(cx, |_, window, cx| {
            create_and_open_local_file(paths::settings_file(), window, cx, || {
                settings::initial_user_settings_content().as_ref().into()
            })
        })?
        .await?
        .downcast::<Editor>()
        .unwrap();

    settings_editor
        .downgrade()
        .update_in(cx, |item, window, cx| {
            let text = item.buffer().read(cx).snapshot(cx).text();

            let settings = cx.global::<SettingsStore>();

            let mut unique_server_name = None;
            let Some(edits) = settings
                .edits_for_update(&text, |settings| {
                    let server_name: Option<String> = (0..u8::MAX)
                        .map(|i| {
                            if i == 0 {
                                "your_agent".to_string()
                            } else {
                                format!("your_agent_{}", i)
                            }
                        })
                        .find(|name| {
                            !settings
                                .agent_servers
                                .as_ref()
                                .is_some_and(|agent_servers| {
                                    agent_servers.contains_key(name.as_str())
                                })
                        });
                    if let Some(server_name) = server_name {
                        unique_server_name = Some(SharedString::from(server_name.clone()));
                        settings.agent_servers.get_or_insert_default().insert(
                            server_name,
                            settings::CustomAgentServerSettings::Custom {
                                path: "path_to_executable".into(),
                                args: vec![],
                                env: HashMap::default(),
                                default_mode: None,
                                default_model: None,
                                favorite_models: vec![],
                                default_config_options: Default::default(),
                                favorite_config_option_values: Default::default(),
                            },
                        );
                    }
                })
                .log_err()
            else {
                return;
            };

            if edits.is_empty() {
                return;
            }

            let ranges = edits
                .iter()
                .map(|(range, _)| range.clone())
                .collect::<Vec<_>>();

            item.edit(
                edits.into_iter().map(|(range, s)| {
                    (
                        MultiBufferOffset(range.start)..MultiBufferOffset(range.end),
                        s,
                    )
                }),
                cx,
            );
            if let Some((unique_server_name, buffer)) =
                unique_server_name.zip(item.buffer().read(cx).as_singleton())
            {
                let snapshot = buffer.read(cx).snapshot();
                if let Some(range) =
                    find_text_in_buffer(&unique_server_name, ranges[0].start, &snapshot)
                {
                    item.change_selections(
                        SelectionEffects::scroll(Autoscroll::newest()),
                        window,
                        cx,
                        |selections| {
                            selections.select_ranges(vec![
                                MultiBufferOffset(range.start)..MultiBufferOffset(range.end),
                            ]);
                        },
                    );
                }
            }
        })
}

fn find_text_in_buffer(
    text: &str,
    start: usize,
    snapshot: &language::BufferSnapshot,
) -> Option<Range<usize>> {
    let chars = text.chars().collect::<Vec<char>>();

    let mut offset = start;
    let mut char_offset = 0;
    for c in snapshot.chars_at(start) {
        if char_offset >= chars.len() {
            break;
        }
        offset += 1;

        if c == chars[char_offset] {
            char_offset += 1;
        } else {
            char_offset = 0;
        }
    }

    if char_offset == chars.len() {
        Some(offset.saturating_sub(chars.len())..offset)
    } else {
        None
    }
}

// OpenAI-compatible providers are user-configured and can be removed,
// whereas built-in providers (like Anthropic, OpenAI, Google, etc.) can't.
//
// If in the future we have more "API-compatible-type" of providers,
// they should be included here as removable providers.
fn is_removable_provider(provider_id: &LanguageModelProviderId, cx: &App) -> bool {
    AllLanguageModelSettings::get_global(cx)
        .openai_compatible
        .contains_key(provider_id.0.as_ref())
}
