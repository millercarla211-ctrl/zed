use crate::multibuffer_hint::MultibufferHint;
use client::{Client, UserStore, zed_urls};
use cloud_api_types::Plan;
use db::kvp::KeyValueStore;
use fs::Fs;
use gpui::{
    Action, AnyElement, App, AppContext, AsyncWindowContext, Context, Entity, EventEmitter,
    FocusHandle, Focusable, Global, IntoElement, KeyContext, Render, ScrollHandle, SharedString,
    Subscription, Task, WeakEntity, Window, actions,
};
use notifications::status_toast::StatusToast;
use project::agent_server_store::AllAgentServersSettings;
use schemars::JsonSchema;
use serde::Deserialize;
use settings::{SettingsStore, VsCodeSettingsSource};
use std::sync::Arc;
use ui::{
    Divider, KeyBinding, ParentElement as _, StatefulInteractiveElement, Vector, VectorName,
    WithScrollbar as _, prelude::*, rems_from_px,
};

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use web_preview::web_preview_view::WebPreviewView;
pub use workspace::welcome::ShowWelcome;
use workspace::welcome::WelcomePage;
use workspace::{
    AppState, ToggleWorkspaceSidebar, Workspace, WorkspaceId,
    dock::DockPosition,
    item::{Item, ItemEvent},
    notifications::NotifyResultExt as _,
    open_new, register_serializable_item, with_active_or_new_workspace,
};
use zed_actions::{OpenOnboarding, OpenSettings, assistant::ToggleFocus};

mod base_keymap_picker;
mod basics_page;
mod dx_launch_onboarding;
mod dx_provider_onboarding;
pub mod multibuffer_hint;
mod theme_preview;

use dx_launch_onboarding::{DxLaunchPreviewTarget, DxLaunchPreviewTargets};
use dx_provider_onboarding::DxProviderOnboardingStatus;

/// Imports settings from Visual Studio Code.
#[derive(Copy, Clone, Debug, Default, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = zed)]
#[serde(deny_unknown_fields)]
pub struct ImportVsCodeSettings {
    #[serde(default)]
    pub skip_prompt: bool,
}

/// Imports settings from Cursor editor.
#[derive(Copy, Clone, Debug, Default, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = zed)]
#[serde(deny_unknown_fields)]
pub struct ImportCursorSettings {
    #[serde(default)]
    pub skip_prompt: bool,
}

pub const FIRST_OPEN: &str = "first_open";
pub const DOCS_URL: &str = "https://zed.dev/docs/";

actions!(
    onboarding,
    [
        /// Finish the onboarding process.
        Finish,
        /// Sign in while in the onboarding flow.
        SignIn,
        /// Open the user account in zed.dev while in the onboarding flow.
        OpenAccount,
        /// Load the selected DX WWW preview target in onboarding.
        OpenDxWwwPreview,
        /// Load the bundled DX onboarding fallback preview.
        OpenBundledDxPreview,
        /// Resets the welcome screen hints to their initial state.
        ResetHints
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _cx| {
        workspace
            .register_action(|_workspace, _: &ResetHints, _, cx| MultibufferHint::set_count(0, cx));
    })
    .detach();

    cx.on_action(|_: &OpenOnboarding, cx| {
        with_active_or_new_workspace(cx, |workspace, window, cx| {
            workspace
                .with_local_workspace(window, cx, |workspace, window, cx| {
                    let existing = workspace
                        .active_pane()
                        .read(cx)
                        .items()
                        .find_map(|item| item.downcast::<Onboarding>());

                    if let Some(existing) = existing {
                        workspace.activate_item(&existing, true, true, window, cx);
                    } else {
                        let settings_page = Onboarding::new(workspace, cx);
                        workspace.add_item_to_active_pane(
                            Box::new(settings_page),
                            None,
                            true,
                            window,
                            cx,
                        )
                    }
                })
                .detach();
        });
    });

    cx.on_action(|_: &ShowWelcome, cx| {
        with_active_or_new_workspace(cx, |workspace, window, cx| {
            workspace
                .with_local_workspace(window, cx, |workspace, window, cx| {
                    let existing = workspace
                        .active_pane()
                        .read(cx)
                        .items()
                        .find_map(|item| item.downcast::<WelcomePage>());

                    if let Some(existing) = existing {
                        workspace.activate_item(&existing, true, true, window, cx);
                    } else {
                        let settings_page = cx
                            .new(|cx| WelcomePage::new(workspace.weak_handle(), false, window, cx));
                        workspace.add_item_to_active_pane(
                            Box::new(settings_page),
                            None,
                            true,
                            window,
                            cx,
                        )
                    }
                })
                .detach();
        });
    });

    cx.observe_new(|workspace: &mut Workspace, _window, _cx| {
        workspace.register_action(|_workspace, action: &ImportVsCodeSettings, window, cx| {
            let fs = <dyn Fs>::global(cx);
            let action = *action;

            let workspace = cx.weak_entity();

            window
                .spawn(cx, async move |cx: &mut AsyncWindowContext| {
                    handle_import_vscode_settings(
                        workspace,
                        VsCodeSettingsSource::VsCode,
                        action.skip_prompt,
                        fs,
                        cx,
                    )
                    .await
                })
                .detach();
        });

        workspace.register_action(|_workspace, action: &ImportCursorSettings, window, cx| {
            let fs = <dyn Fs>::global(cx);
            let action = *action;

            let workspace = cx.weak_entity();

            window
                .spawn(cx, async move |cx: &mut AsyncWindowContext| {
                    handle_import_vscode_settings(
                        workspace,
                        VsCodeSettingsSource::Cursor,
                        action.skip_prompt,
                        fs,
                        cx,
                    )
                    .await
                })
                .detach();
        });
    })
    .detach();

    base_keymap_picker::init(cx);

    register_serializable_item::<Onboarding>(cx);
    register_serializable_item::<WelcomePage>(cx);
}

pub fn show_onboarding_view(app_state: Arc<AppState>, cx: &mut App) -> Task<anyhow::Result<()>> {
    telemetry::event!("Onboarding Page Opened");
    open_new(
        Default::default(),
        app_state,
        cx,
        |workspace, window, cx| {
            {
                workspace.toggle_dock(DockPosition::Left, window, cx);
                let onboarding_page = Onboarding::new(workspace, cx);
                workspace.add_item_to_center(Box::new(onboarding_page.clone()), window, cx);

                window.focus(&onboarding_page.focus_handle(cx), cx);

                cx.notify();
            };
            let kvp = KeyValueStore::global(cx);
            db::write_and_log(cx, move || async move {
                kvp.write_kvp(FIRST_OPEN.to_string(), "false".to_string())
                    .await
            });
        },
    )
}

fn provider_status_icon(state: &str) -> IconName {
    match state {
        "ready" | "visible" => IconName::Check,
        "needs approval" => IconName::Warning,
        _ => IconName::Info,
    }
}

struct Onboarding {
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    user_store: Entity<UserStore>,
    scroll_handle: ScrollHandle,
    dx_preview_targets: DxLaunchPreviewTargets,
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    dx_web_preview: Option<Entity<WebPreviewView>>,
    _settings_subscription: Subscription,
}

impl Onboarding {
    fn new(workspace: &Workspace, cx: &mut App) -> Entity<Self> {
        let font_family_cache = theme::FontFamilyCache::global(cx);

        let installed_agents = cx
            .global::<SettingsStore>()
            .get::<AllAgentServersSettings>(None)
            .clone();
        let client = Client::global(cx);
        let status = *client.status().borrow();
        let plan = workspace.user_store().read(cx).plan();
        let zed_agent_state = if status.is_signed_out()
            || matches!(
                status,
                client::Status::AuthenticationError | client::Status::ConnectionError
            ) {
            "signed_out"
        } else if status.is_signing_in() {
            "signing_in"
        } else {
            match plan {
                Some(Plan::ZedPro) => "pro",
                Some(Plan::ZedProTrial) => "trial",
                Some(Plan::ZedBusiness) => "business",
                Some(Plan::ZedStudent) => "student",
                Some(Plan::ZedFree) | None => "free",
            }
        };
        let agents_installed = basics_page::FEATURED_AGENT_IDS
            .iter()
            .filter(|id| installed_agents.contains_key(**id))
            .copied()
            .collect::<Vec<_>>();
        telemetry::event!(
            "Welcome Agent Setup Viewed",
            zed_agent = zed_agent_state,
            agents_installed = agents_installed,
        );
        let dx_preview_targets = DxLaunchPreviewTargets::detect();

        cx.new(|cx| {
            cx.spawn(async move |this, cx| {
                font_family_cache.prefetch(cx).await;
                this.update(cx, |_, cx| {
                    cx.notify();
                })
            })
            .detach();

            Self {
                workspace: workspace.weak_handle(),
                focus_handle: cx.focus_handle(),
                scroll_handle: ScrollHandle::new(),
                user_store: workspace.user_store().clone(),
                dx_preview_targets,
                #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                dx_web_preview: None,
                _settings_subscription: cx
                    .observe_global::<SettingsStore>(move |_, cx| cx.notify()),
            }
        })
    }

    fn on_finish(_: &Finish, _: &mut Window, cx: &mut App) {
        telemetry::event!("Finish Setup");
        go_to_welcome_page(cx);
    }

    fn handle_sign_in(&mut self, _: &SignIn, window: &mut Window, cx: &mut Context<Self>) {
        let client = Client::global(cx);
        let workspace = self.workspace.clone();

        window
            .spawn(cx, async move |mut cx| {
                client
                    .sign_in_with_optional_connect(true, &cx)
                    .await
                    .notify_workspace_async_err(workspace, &mut cx);
            })
            .detach();
    }

    fn handle_open_account(_: &OpenAccount, _: &mut Window, cx: &mut App) {
        cx.open_url(&zed_urls::account_url(cx))
    }

    fn handle_open_dx_www_preview(
        &mut self,
        _: &OpenDxWwwPreview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(target) = self.dx_preview_targets.dx_www.clone() {
            self.load_dx_preview_target(target, window, cx);
        }
    }

    fn handle_open_bundled_dx_preview(
        &mut self,
        _: &OpenBundledDxPreview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.load_dx_preview_target(self.dx_preview_targets.fallback.clone(), window, cx);
    }

    fn load_dx_preview_target(
        &mut self,
        target: DxLaunchPreviewTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dx_preview_targets.primary = target.clone();
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            let preview = self.ensure_dx_web_preview(window, cx);
            preview.update(cx, |preview, cx| {
                preview.load_onboarding_url(&target.url, window, cx);
            });
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let _ = window;
        cx.notify();
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn ensure_dx_web_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<WebPreviewView> {
        if let Some(preview) = self.dx_web_preview.clone() {
            return preview;
        }

        let workspace = self.workspace.clone();
        let url = self.dx_preview_targets.primary.url.clone();
        let preview = cx.new(|cx| {
            WebPreviewView::new_for_onboarding(
                workspace,
                url,
                Some("DX Launch Preview".into()),
                window,
                cx,
            )
        });
        self.dx_web_preview = Some(preview.clone());
        preview
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn deactivate_dx_web_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(preview) = self.dx_web_preview.as_ref() {
            preview.update(cx, |preview, cx| preview.deactivated(window, cx));
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn render_web_preview_canvas(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let preview = self.ensure_dx_web_preview(window, cx);
        div().size_full().child(preview).into_any_element()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    fn render_web_preview_canvas(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().colors().surface_background)
            .child(
                Label::new("DX onboarding Web Preview is available on supported desktop runtimes.")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .into_any_element()
    }

    fn render_dx_launch_hero(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let border_variant = cx.theme().colors().border_variant;
        let panel_background = cx.theme().colors().panel_background;
        let editor_background = cx.theme().colors().editor_background;
        let target = self.dx_preview_targets.primary.clone();

        v_flex()
            .w_full()
            .gap_5()
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    .justify_between()
                    .child(
                        h_flex()
                            .min_w_0()
                            .gap_4()
                            .child(Vector::square(VectorName::ZedLogo, rems(2.5)))
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .child(
                                        Headline::new("DX Launch Workspace")
                                            .size(HeadlineSize::Small),
                                    )
                                    .child(
                                        Label::new(
                                            "Web Preview onboarding for DX WWW, Forge, agents, and source-owned packages",
                                        )
                                        .color(Color::Muted)
                                        .size(LabelSize::Small),
                                    ),
                            ),
                    )
                    .child(
                        Button::new("skip_dx_onboarding", "Skip")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Medium)
                            .key_binding(KeyBinding::for_action_in(&Finish, &self.focus_handle, cx))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Finish.boxed_clone(), cx);
                            }),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(border_variant)
                    .bg(linear_gradient(
                        140.,
                        linear_color_stop(panel_background, 0.88),
                        linear_color_stop(editor_background, 1.0),
                    ))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Label::new("Native editor. Live browser canvas. DX launch flow.")
                                    .size(LabelSize::Large),
                            )
                            .child(
                                Label::new(
                                    "The first-run path now opens with a real Web Preview surface. It prefers the selected DX WWW target and falls back to a bundled original 3D launch page when the workspace target is missing.",
                                )
                                .color(Color::Muted),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .flex_wrap()
                                    .children([
                                        self.render_capability_pill("Native Zed performance", cx),
                                        self.render_capability_pill("Web Preview hero", cx),
                                        self.render_capability_pill("DX WWW / Forge", cx),
                                        self.render_capability_pill("Agents", cx),
                                        self.render_capability_pill("Browser checks", cx),
                                        self.render_capability_pill("Source packages", cx),
                                    ]),
                            ),
                    )
                    .child(self.render_provider_strategy(cx))
                    .child(self.render_preview_contract_status(&target, cx))
                    .child(self.render_preview_frame(target, window, cx))
                    .child(self.render_quick_launch_actions(cx))
                    .when(self.dx_preview_targets.dx_www.is_none(), |this| {
                        this.child(
                            Label::new(self.dx_preview_targets.missing_dx_www_detail())
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_preview_contract_status(
        &self,
        target: &DxLaunchPreviewTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .child(
                v_flex()
                    .min_w_0()
                    .child(Label::new("Preview Contract").size(LabelSize::Small))
                    .child(
                        Label::new(target.detail.clone())
                            .size(LabelSize::Small)
                            .color(Color::Muted)
                            .truncate(),
                    ),
            )
            .child(
                h_flex().w_full().gap_2().flex_wrap().children(
                    self.dx_preview_targets
                        .preview_status_rows(target)
                        .into_iter()
                        .map(|row| {
                            self.render_provider_status_row(row.label, row.detail, row.state, cx)
                        }),
                ),
            )
            .into_any_element()
    }

    fn render_provider_strategy(&self, cx: &mut Context<Self>) -> AnyElement {
        let status = DxProviderOnboardingStatus::detect(cx);

        v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .justify_between()
                    .child(
                        v_flex()
                            .min_w_0()
                            .child(Label::new("Provider Readiness").size(LabelSize::Small))
                            .child(
                                Label::new(status.summary.clone())
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(
                        Button::new("dx_provider_readiness_settings", "Provider Settings")
                            .size(ButtonSize::Small)
                            .style(ButtonStyle::Outlined)
                            .on_click(|_, window, cx| {
                                window.dispatch_action(OpenSettings.boxed_clone(), cx);
                            }),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .flex_wrap()
                    .child(self.render_provider_status_row(
                        "Native",
                        status.native_provider_label(),
                        status.state,
                        cx,
                    ))
                    .child(self.render_provider_status_row(
                        "DX Receipts",
                        status.receipt_label(),
                        if status.provider_receipt_present
                            || status.model_receipt_present
                            || status.contract_receipt_present
                        {
                            "visible"
                        } else {
                            "missing"
                        },
                        cx,
                    ))
                    .child(self.render_provider_status_row(
                        "Catalog",
                        status.catalog_label(),
                        if status.catalog_present {
                            "visible"
                        } else {
                            "missing"
                        },
                        cx,
                    )),
            )
            .child(
                Label::new(status.next_action)
                    .size(LabelSize::Small)
                    .color(Color::Muted)
                    .truncate(),
            )
            .into_any_element()
    }

    fn render_provider_status_row(
        &self,
        label: &'static str,
        detail: String,
        state: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .min_w(rems(12.))
            .flex_1()
            .gap_0p5()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().panel_background)
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Icon::new(provider_status_icon(state))
                            .size(IconSize::XSmall)
                            .color(match state {
                                "ready" | "visible" => Color::Success,
                                "needs approval" => Color::Warning,
                                _ => Color::Muted,
                            }),
                    )
                    .child(Label::new(label).size(LabelSize::XSmall)),
            )
            .child(
                Label::new(detail)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
            .into_any_element()
    }

    fn render_capability_pill(&self, label: &'static str, cx: &mut Context<Self>) -> AnyElement {
        div()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .child(Label::new(label).size(LabelSize::Small).color(Color::Muted))
            .into_any_element()
    }

    fn render_preview_frame(
        &mut self,
        target: DxLaunchPreviewTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let border_variant = cx.theme().colors().border_variant;
        let surface_background = cx.theme().colors().surface_background;
        let has_dx_www = self.dx_preview_targets.dx_www.is_some();

        v_flex()
            .w_full()
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(border_variant)
            .bg(surface_background)
            .child(
                h_flex()
                    .min_w_0()
                    .w_full()
                    .gap_2()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border_variant)
                    .child(
                        v_flex()
                            .min_w_0()
                            .child(Label::new(target.title).size(LabelSize::Small))
                            .child(
                                Label::new(target.detail)
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("dx_load_www_preview", "Load G:\\WWW")
                                    .size(ButtonSize::Small)
                                    .style(ButtonStyle::Outlined)
                                    .disabled(!has_dx_www)
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(OpenDxWwwPreview.boxed_clone(), cx);
                                    }),
                            )
                            .child(
                                Button::new("dx_load_bundled_preview", "Load 3D fallback")
                                    .size(ButtonSize::Small)
                                    .style(ButtonStyle::Outlined)
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(
                                            OpenBundledDxPreview.boxed_clone(),
                                            cx,
                                        );
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(480.))
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.render_web_preview_canvas(window, cx)),
            )
            .into_any_element()
    }

    fn render_quick_launch_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        let focus = self.focus_handle.clone();
        let has_dx_www = self.dx_preview_targets.dx_www.is_some();

        h_flex()
            .w_full()
            .gap_2()
            .flex_wrap()
            .child(
                Button::new("dx_demo_web_preview", "Web Preview")
                    .style(ButtonStyle::Outlined)
                    .disabled(!has_dx_www)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(OpenDxWwwPreview.boxed_clone(), cx);
                    }),
            )
            .child(
                Button::new("dx_demo_3d_preview", "3D Scene")
                    .style(ButtonStyle::Outlined)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(OpenBundledDxPreview.boxed_clone(), cx);
                    }),
            )
            .child(
                Button::new("dx_open_agent_panel", "Open Agent Panel")
                    .style(ButtonStyle::Filled)
                    .key_binding(
                        KeyBinding::for_action_in(&ToggleFocus, &self.focus_handle, cx)
                            .size(rems_from_px(12.)),
                    )
                    .on_click(move |_, window, cx| {
                        focus.dispatch_action(&ToggleWorkspaceSidebar, window, cx);
                        focus.dispatch_action(&ToggleFocus, window, cx);
                    }),
            )
            .child(
                Button::new("dx_open_automations", "Automations")
                    .style(ButtonStyle::Outlined)
                    .on_click(|_, window, cx| {
                        window
                            .dispatch_action(zed_actions::OpenProjectDebugTasks.boxed_clone(), cx);
                    }),
            )
            .child(
                Button::new("dx_open_provider_settings", "Provider Settings")
                    .style(ButtonStyle::Outlined)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(OpenSettings.boxed_clone(), cx);
                    }),
            )
            .child(
                Button::new("dx_skip_onboarding_action", "Skip")
                    .style(ButtonStyle::Subtle)
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Finish.boxed_clone(), cx);
                    }),
            )
            .into_any_element()
    }

    fn render_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        crate::basics_page::render_basics_page(&self.user_store, cx).into_any_element()
    }
}

impl Render for Onboarding {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .image_cache(gpui::retain_all("onboarding-page"))
            .key_context({
                let mut ctx = KeyContext::new_with_defaults();
                ctx.add("Onboarding");
                ctx.add("menu");
                ctx
            })
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .on_action(Self::on_finish)
            .on_action(cx.listener(Self::handle_sign_in))
            .on_action(Self::handle_open_account)
            .on_action(cx.listener(Self::handle_open_dx_www_preview))
            .on_action(cx.listener(Self::handle_open_bundled_dx_preview))
            .on_action(cx.listener(|_, _: &menu::SelectNext, window, cx| {
                window.focus_next(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &menu::SelectPrevious, window, cx| {
                window.focus_prev(cx);
                cx.notify();
            }))
            .vertical_scrollbar_for(&self.scroll_handle, window, cx)
            .child(
                div()
                    .id("page-content")
                    .size_full()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .min_w_0()
                            .max_w(rems_from_px(1180.))
                            .w_full()
                            .mx_auto()
                            .p_8()
                            .gap_6()
                            .child(self.render_dx_launch_hero(window, cx))
                            .child(Divider::horizontal().color(ui::DividerColor::BorderVariant))
                            .child(self.render_page(cx)),
                    )
                    .track_scroll(&self.scroll_handle),
            )
    }
}

impl EventEmitter<ItemEvent> for Onboarding {}

impl Focusable for Onboarding {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for Onboarding {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "Onboarding".into()
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Onboarding Page Opened")
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn can_split(&self) -> bool {
        true
    }

    fn deactivated(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        self.deactivate_dx_web_preview(window, cx);
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let _ = (window, cx);
    }

    fn workspace_deactivated(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        self.deactivate_dx_web_preview(window, cx);
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let _ = (window, cx);
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        Task::ready(Some(cx.new(|cx| Onboarding {
            workspace: self.workspace.clone(),
            user_store: self.user_store.clone(),
            scroll_handle: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            dx_preview_targets: self.dx_preview_targets.clone(),
            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            dx_web_preview: None,
            _settings_subscription: cx.observe_global::<SettingsStore>(move |_, cx| cx.notify()),
        })))
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(workspace::item::ItemEvent)) {
        f(*event)
    }
}

fn go_to_welcome_page(cx: &mut App) {
    with_active_or_new_workspace(cx, |workspace, window, cx| {
        let Some((onboarding_id, onboarding_idx)) = workspace
            .active_pane()
            .read(cx)
            .items()
            .enumerate()
            .find_map(|(idx, item)| {
                let _ = item.downcast::<Onboarding>()?;
                Some((item.item_id(), idx))
            })
        else {
            return;
        };

        workspace.active_pane().update(cx, |pane, cx| {
            // Get the index here to get around the borrow checker
            let idx = pane.items().enumerate().find_map(|(idx, item)| {
                let _ = item.downcast::<WelcomePage>()?;
                Some(idx)
            });

            if let Some(idx) = idx {
                pane.activate_item(idx, true, true, window, cx);
            } else {
                let item = Box::new(
                    cx.new(|cx| WelcomePage::new(workspace.weak_handle(), false, window, cx)),
                );
                pane.add_item(item, true, true, Some(onboarding_idx), window, cx);
            }

            pane.remove_item(onboarding_id, false, false, window, cx);
        });
    });
}

pub async fn handle_import_vscode_settings(
    workspace: WeakEntity<Workspace>,
    source: VsCodeSettingsSource,
    skip_prompt: bool,
    fs: Arc<dyn Fs>,
    cx: &mut AsyncWindowContext,
) {
    use util::truncate_and_remove_front;

    let vscode_settings =
        match settings::VsCodeSettings::load_user_settings(source, fs.clone()).await {
            Ok(vscode_settings) => vscode_settings,
            Err(err) => {
                zlog::error!("{err:?}");
                let _ = cx.prompt(
                    gpui::PromptLevel::Info,
                    &format!("Could not find or load a {source} settings file"),
                    None,
                    &["Ok"],
                );
                return;
            }
        };

    if !skip_prompt {
        let prompt = cx.prompt(
            gpui::PromptLevel::Warning,
            &format!(
                "Importing {} settings may overwrite your existing settings. \
                Will import settings from {}",
                vscode_settings.source,
                truncate_and_remove_front(&vscode_settings.path.to_string_lossy(), 128),
            ),
            None,
            &["Ok", "Cancel"],
        );
        let result = cx.spawn(async move |_| prompt.await.ok()).await;
        if result != Some(0) {
            return;
        }
    };

    let Ok(result_channel) = cx.update(|_, cx| {
        let source = vscode_settings.source;
        let path = vscode_settings.path.clone();
        let result_channel = cx
            .global::<SettingsStore>()
            .import_vscode_settings(fs, vscode_settings);
        zlog::info!("Imported {source} settings from {}", path.display());
        result_channel
    }) else {
        return;
    };

    let result = result_channel.await;
    workspace
        .update_in(cx, |workspace, _, cx| match result {
            Ok(_) => {
                let confirmation_toast = StatusToast::new(
                    format!("Your {} settings were successfully imported.", source),
                    cx,
                    |this, _| {
                        this.icon(
                            Icon::new(IconName::Check)
                                .size(IconSize::Small)
                                .color(Color::Success),
                        )
                        .dismiss_button(true)
                    },
                );
                SettingsImportState::update(cx, |state, _| match source {
                    VsCodeSettingsSource::VsCode => {
                        state.vscode = true;
                    }
                    VsCodeSettingsSource::Cursor => {
                        state.cursor = true;
                    }
                });
                workspace.toggle_status_toast(confirmation_toast, cx);
            }
            Err(_) => {
                let error_toast = StatusToast::new(
                    "Failed to import settings. See log for details",
                    cx,
                    |this, _| {
                        this.icon(
                            Icon::new(IconName::Close)
                                .size(IconSize::Small)
                                .color(Color::Error),
                        )
                        .action("Open Log", |window, cx| {
                            window.dispatch_action(workspace::OpenLog.boxed_clone(), cx)
                        })
                        .dismiss_button(true)
                    },
                );
                workspace.toggle_status_toast(error_toast, cx);
            }
        })
        .ok();
}

#[derive(Default, Copy, Clone)]
pub struct SettingsImportState {
    pub cursor: bool,
    pub vscode: bool,
}

impl Global for SettingsImportState {}

impl SettingsImportState {
    pub fn global(cx: &App) -> Self {
        cx.try_global().cloned().unwrap_or_default()
    }
    pub fn update<R>(cx: &mut App, f: impl FnOnce(&mut Self, &mut App) -> R) -> R {
        cx.update_default_global(f)
    }
}

impl workspace::SerializableItem for Onboarding {
    fn serialized_item_kind() -> &'static str {
        "OnboardingPage"
    }

    fn cleanup(
        workspace_id: workspace::WorkspaceId,
        alive_items: Vec<workspace::ItemId>,
        _window: &mut Window,
        cx: &mut App,
    ) -> gpui::Task<gpui::Result<()>> {
        workspace::delete_unloaded_items(
            alive_items,
            workspace_id,
            "onboarding_pages",
            &persistence::OnboardingPagesDb::global(cx),
            cx,
        )
    }

    fn deserialize(
        _project: Entity<project::Project>,
        workspace: WeakEntity<Workspace>,
        workspace_id: workspace::WorkspaceId,
        item_id: workspace::ItemId,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Task<gpui::Result<Entity<Self>>> {
        let db = persistence::OnboardingPagesDb::global(cx);
        window.spawn(cx, async move |cx| {
            if let Some(_) = db.get_onboarding_page(item_id, workspace_id)? {
                workspace.update(cx, |workspace, cx| Onboarding::new(workspace, cx))
            } else {
                Err(anyhow::anyhow!("No onboarding page to deserialize"))
            }
        })
    }

    fn serialize(
        &mut self,
        workspace: &mut Workspace,
        item_id: workspace::ItemId,
        _closing: bool,
        _window: &mut Window,
        cx: &mut ui::Context<Self>,
    ) -> Option<gpui::Task<gpui::Result<()>>> {
        let workspace_id = workspace.database_id()?;

        let db = persistence::OnboardingPagesDb::global(cx);
        Some(
            cx.background_spawn(
                async move { db.save_onboarding_page(item_id, workspace_id).await },
            ),
        )
    }

    fn should_serialize(&self, event: &Self::Event) -> bool {
        event == &ItemEvent::UpdateTab
    }
}

mod persistence {
    use db::{
        query,
        sqlez::{domain::Domain, thread_safe_connection::ThreadSafeConnection},
        sqlez_macros::sql,
    };
    use workspace::WorkspaceDb;

    pub struct OnboardingPagesDb(ThreadSafeConnection);

    impl Domain for OnboardingPagesDb {
        const NAME: &str = stringify!(OnboardingPagesDb);

        const MIGRATIONS: &[&str] = &[
            sql!(
                        CREATE TABLE onboarding_pages (
                            workspace_id INTEGER,
                            item_id INTEGER UNIQUE,
                            page_number INTEGER,

                            PRIMARY KEY(workspace_id, item_id),
                            FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
                            ON DELETE CASCADE
                        ) STRICT;
            ),
            sql!(
                        CREATE TABLE onboarding_pages_2 (
                            workspace_id INTEGER,
                            item_id INTEGER UNIQUE,

                            PRIMARY KEY(workspace_id, item_id),
                            FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
                            ON DELETE CASCADE
                        ) STRICT;
                        INSERT INTO onboarding_pages_2 SELECT workspace_id, item_id FROM onboarding_pages;
                        DROP TABLE onboarding_pages;
                        ALTER TABLE onboarding_pages_2 RENAME TO onboarding_pages;
            ),
        ];
    }

    db::static_connection!(OnboardingPagesDb, [WorkspaceDb]);

    impl OnboardingPagesDb {
        query! {
            pub async fn save_onboarding_page(
                item_id: workspace::ItemId,
                workspace_id: workspace::WorkspaceId
            ) -> Result<()> {
                INSERT OR REPLACE INTO onboarding_pages(item_id, workspace_id)
                VALUES (?, ?)
            }
        }

        query! {
            pub fn get_onboarding_page(
                item_id: workspace::ItemId,
                workspace_id: workspace::WorkspaceId
            ) -> Result<Option<workspace::ItemId>> {
                SELECT item_id
                FROM onboarding_pages
                WHERE item_id = ? AND workspace_id = ?
            }
        }
    }
}
