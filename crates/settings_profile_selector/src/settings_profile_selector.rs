use fuzzy::{StringMatch, StringMatchCandidate, match_strings};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, Focusable, Render, Task, WeakEntity, Window,
};
use picker::{Picker, PickerDelegate};
use settings::{ActiveSettingsProfileName, SettingsStore};
use ui::{HighlightedLabel, ListItem, ListItemSpacing, prelude::*};
use workspace::{ModalView, Workspace};

const MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES: usize = 100;
const MAX_SETTINGS_PROFILE_SELECTOR_MATCHES: usize =
    MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES + 1;

pub fn init(cx: &mut App) {
    cx.on_action(|_: &zed_actions::settings_profile_selector::Toggle, cx| {
        workspace::with_active_or_new_workspace(cx, |workspace, window, cx| {
            toggle_settings_profile_selector(workspace, window, cx);
        });
    });
}

fn toggle_settings_profile_selector(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    workspace.toggle_modal(window, cx, |window, cx| {
        let delegate = SettingsProfileSelectorDelegate::new(cx.entity().downgrade(), window, cx);
        SettingsProfileSelector::new(delegate, window, cx)
    });
}

fn capped_settings_profile_names(
    settings_store: &SettingsStore,
    active_profile_name: Option<&str>,
) -> Vec<Option<String>> {
    let mut configured_profiles = settings_store
        .configured_settings_profiles()
        .take(MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES)
        .map(|profile_name| Some(profile_name.to_string()))
        .collect::<Vec<_>>();

    let overflow_active_profile = active_profile_name.and_then(|active_profile_name| {
        settings_store
            .configured_settings_profiles()
            .skip(MAX_SETTINGS_PROFILE_SELECTOR_CONFIGURED_PROFILES)
            .find(|profile_name| *profile_name == active_profile_name)
    });

    if let Some(active_profile_name) = overflow_active_profile {
        configured_profiles.pop();
        configured_profiles.push(Some(active_profile_name.to_string()));
    }

    let mut profile_names = Vec::with_capacity(configured_profiles.len() + 1);
    profile_names.push(None);
    profile_names.extend(configured_profiles);
    profile_names
}

fn profile_match_candidates(profile_names: &[Option<String>]) -> Vec<StringMatchCandidate> {
    profile_names
        .iter()
        .enumerate()
        .take(MAX_SETTINGS_PROFILE_SELECTOR_MATCHES)
        .map(|(candidate_id, profile_name)| {
            StringMatchCandidate::new(candidate_id, &display_name(profile_name))
        })
        .collect()
}

fn empty_profile_matches(candidates: Vec<StringMatchCandidate>) -> Vec<StringMatch> {
    candidates
        .into_iter()
        .take(MAX_SETTINGS_PROFILE_SELECTOR_MATCHES)
        .map(|candidate| StringMatch {
            candidate_id: candidate.id,
            string: candidate.string,
            positions: Vec::new(),
            score: 0.0,
        })
        .collect()
}

pub struct SettingsProfileSelector {
    picker: Entity<Picker<SettingsProfileSelectorDelegate>>,
}

impl ModalView for SettingsProfileSelector {}

impl EventEmitter<DismissEvent> for SettingsProfileSelector {}

impl Focusable for SettingsProfileSelector {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for SettingsProfileSelector {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().w(rems(22.)).child(self.picker.clone())
    }
}

impl SettingsProfileSelector {
    pub fn new(
        delegate: SettingsProfileSelectorDelegate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let picker = cx.new(|cx| Picker::uniform_list(delegate, window, cx));
        Self { picker }
    }
}

pub struct SettingsProfileSelectorDelegate {
    matches: Vec<StringMatch>,
    profile_names: Vec<Option<String>>,
    original_profile_name: Option<String>,
    selected_profile_name: Option<String>,
    selected_index: usize,
    selection_completed: bool,
    selector: WeakEntity<SettingsProfileSelector>,
}

impl SettingsProfileSelectorDelegate {
    fn new(
        selector: WeakEntity<SettingsProfileSelector>,
        _: &mut Window,
        cx: &mut Context<SettingsProfileSelector>,
    ) -> Self {
        let settings_store = cx.global::<SettingsStore>();
        let profile_name = cx
            .try_global::<ActiveSettingsProfileName>()
            .map(|p| p.0.clone());
        let profile_names = capped_settings_profile_names(settings_store, profile_name.as_deref());
        let matches = empty_profile_matches(profile_match_candidates(&profile_names));

        let mut this = Self {
            matches,
            profile_names,
            original_profile_name: profile_name.clone(),
            selected_profile_name: None,
            selected_index: 0,
            selection_completed: false,
            selector,
        };

        if let Some(profile_name) = profile_name {
            this.select_if_matching(&profile_name);
        }

        this
    }

    fn select_if_matching(&mut self, profile_name: &str) {
        self.selected_index = self
            .matches
            .iter()
            .position(|mat| {
                self.profile_name_for_match(mat)
                    .and_then(|profile_name| profile_name.as_deref())
                    == Some(profile_name)
            })
            .unwrap_or(self.selected_index);
    }

    fn profile_name_for_match(&self, mat: &StringMatch) -> Option<&Option<String>> {
        self.profile_names.get(mat.candidate_id)
    }

    fn selected_profile_for_update(&self) -> Option<Option<String>> {
        self.matches
            .get(self.selected_index)
            .and_then(|mat| self.profile_name_for_match(mat))
            .cloned()
    }

    fn set_selected_profile(
        &self,
        cx: &mut Context<Picker<SettingsProfileSelectorDelegate>>,
    ) -> Option<String> {
        let profile_name = self.selected_profile_for_update()?;
        Self::update_active_profile_name_global(profile_name, cx)
    }

    fn clamped_match_index(&self, ix: usize) -> usize {
        self.matches
            .len()
            .checked_sub(1)
            .map_or(0, |last_index| ix.min(last_index))
    }

    fn clamp_selected_index_to_matches(&mut self) {
        self.selected_index = self.clamped_match_index(self.selected_index);
    }

    fn update_active_profile_name_global(
        profile_name: Option<String>,
        cx: &mut Context<Picker<SettingsProfileSelectorDelegate>>,
    ) -> Option<String> {
        if let Some(profile_name) = profile_name {
            cx.set_global(ActiveSettingsProfileName(profile_name.clone()));
            return Some(profile_name);
        }

        if cx.has_global::<ActiveSettingsProfileName>() {
            cx.remove_global::<ActiveSettingsProfileName>();
        }

        None
    }
}

impl PickerDelegate for SettingsProfileSelectorDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _: &mut Window, _: &mut App) -> std::sync::Arc<str> {
        "Select a settings profile...".into()
    }

    fn match_count(&self) -> usize {
        self.matches.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _: &mut Window,
        cx: &mut Context<Picker<SettingsProfileSelectorDelegate>>,
    ) {
        self.selected_index = self.clamped_match_index(ix);
        self.selected_profile_name = self.set_selected_profile(cx);
    }

    fn update_matches(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Picker<SettingsProfileSelectorDelegate>>,
    ) -> Task<()> {
        let background = cx.background_executor().clone();
        let candidates = profile_match_candidates(&self.profile_names);

        cx.spawn_in(window, async move |this, cx| {
            let matches = if query.is_empty() {
                empty_profile_matches(candidates)
            } else {
                match_strings(
                    &candidates,
                    &query,
                    false,
                    true,
                    MAX_SETTINGS_PROFILE_SELECTOR_MATCHES,
                    &Default::default(),
                    background,
                )
                .await
            };

            this.update_in(cx, |this, _, cx| {
                this.delegate.matches = matches;
                this.delegate.clamp_selected_index_to_matches();
                this.delegate.selected_profile_name = this.delegate.set_selected_profile(cx);
            })
            .ok();
        })
    }

    fn confirm(
        &mut self,
        _: bool,
        _: &mut Window,
        cx: &mut Context<Picker<SettingsProfileSelectorDelegate>>,
    ) {
        self.selection_completed = true;
        self.selector
            .update(cx, |_, cx| {
                cx.emit(DismissEvent);
            })
            .ok();
    }

    fn dismissed(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Picker<SettingsProfileSelectorDelegate>>,
    ) {
        if !self.selection_completed {
            SettingsProfileSelectorDelegate::update_active_profile_name_global(
                self.original_profile_name.clone(),
                cx,
            );
        }
        self.selector.update(cx, |_, cx| cx.emit(DismissEvent)).ok();
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _: &mut Window,
        _: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let mat = self.matches.get(ix)?;
        let profile_name = self.profile_name_for_match(mat)?;

        Some(
            ListItem::new(ix)
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .child(HighlightedLabel::new(
                    display_name(profile_name),
                    mat.positions.clone(),
                )),
        )
    }
}

fn display_name(profile_name: &Option<String>) -> String {
    profile_name.clone().unwrap_or("Disabled".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor;
    use gpui::{TestAppContext, UpdateGlobal, VisualTestContext};
    use menu::{Cancel, Confirm, SelectNext, SelectPrevious};
    use project::{FakeFs, Project};
    use serde_json::json;
    use settings::Settings;
    use theme_settings::ThemeSettings;
    use workspace::{self, AppState, MultiWorkspace};
    use zed_actions::settings_profile_selector;

    async fn init_test(
        user_settings_json: serde_json::Value,
        cx: &mut TestAppContext,
    ) -> (Entity<Workspace>, &mut VisualTestContext) {
        cx.update(|cx| {
            let state = AppState::test(cx);
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            settings::init(cx);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            super::init(cx);
            editor::init(cx);
            state
        });

        cx.update(|cx| {
            SettingsStore::update_global(cx, |store, cx| {
                store
                    .set_user_settings(&user_settings_json.to_string(), cx)
                    .unwrap();
            });
        });

        let fs = FakeFs::new(cx.executor());
        let project = Project::test(fs, ["/test".as_ref()], cx).await;
        let window = cx.add_window(|window, cx| MultiWorkspace::test_new(project, window, cx));
        let cx = VisualTestContext::from_window(*window, cx).into_mut();
        let workspace = window
            .read_with(cx, |mw, _| mw.workspace().clone())
            .unwrap();

        cx.update(|_, cx| {
            assert!(!cx.has_global::<ActiveSettingsProfileName>());
        });

        (workspace, cx)
    }

    #[track_caller]
    fn active_settings_profile_picker(
        workspace: &Entity<Workspace>,
        cx: &mut VisualTestContext,
    ) -> Entity<Picker<SettingsProfileSelectorDelegate>> {
        workspace.update(cx, |workspace, cx| {
            workspace
                .active_modal::<SettingsProfileSelector>(cx)
                .expect("settings profile selector is not open")
                .read(cx)
                .picker
                .clone()
        })
    }

    #[gpui::test]
    async fn test_settings_profile_selector_state(cx: &mut TestAppContext) {
        let classroom_and_streaming_profile_name = "Classroom / Streaming".to_string();
        let demo_videos_profile_name = "Demo Videos".to_string();

        let user_settings_json = json!({
            "buffer_font_size": 10.0,
            "profiles": {
                classroom_and_streaming_profile_name.clone(): {
                    "settings": {
                        "buffer_font_size": 20.0,
                    }
                },
                demo_videos_profile_name.clone(): {
                    "settings": {
                        "buffer_font_size": 15.0
                    }
                }
            }
        });
        let (workspace, cx) = init_test(user_settings_json, cx).await;

        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.matches.len(), 3);
            assert_eq!(picker.delegate.matches[0].string, display_name(&None));
            assert_eq!(
                picker.delegate.matches[1].string,
                classroom_and_streaming_profile_name
            );
            assert_eq!(picker.delegate.matches[2].string, demo_videos_profile_name);
            assert_eq!(picker.delegate.matches.get(3), None);

            assert_eq!(picker.delegate.selected_index, 0);
            assert_eq!(picker.delegate.selected_profile_name, None);

            assert_eq!(cx.try_global::<ActiveSettingsProfileName>(), None);
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(10.0));
        });

        cx.dispatch_action(Confirm);

        cx.update(|_, cx| {
            assert_eq!(cx.try_global::<ActiveSettingsProfileName>(), None);
        });

        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);
        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 1);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(20.0));
        });

        cx.dispatch_action(Cancel);

        cx.update(|_, cx| {
            assert_eq!(cx.try_global::<ActiveSettingsProfileName>(), None);
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(10.0));
        });

        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);

        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 1);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(20.0));
        });

        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 2);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(demo_videos_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(demo_videos_profile_name.clone())
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(15.0));
        });

        cx.dispatch_action(Confirm);

        cx.update(|_, cx| {
            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(demo_videos_profile_name.clone())
            );
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(15.0));
        });

        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 2);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(demo_videos_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(demo_videos_profile_name.clone())
            );
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(15.0));
        });

        cx.dispatch_action(SelectPrevious);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 1);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(20.0));
        });

        cx.dispatch_action(Cancel);

        cx.update(|_, cx| {
            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(demo_videos_profile_name.clone())
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(15.0));
        });

        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 2);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(demo_videos_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(demo_videos_profile_name)
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(15.0));
        });

        cx.dispatch_action(SelectPrevious);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 1);
            assert_eq!(
                picker.delegate.selected_profile_name,
                Some(classroom_and_streaming_profile_name.clone())
            );

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                Some(classroom_and_streaming_profile_name)
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(20.0));
        });

        cx.dispatch_action(SelectPrevious);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(picker.delegate.selected_index, 0);
            assert_eq!(picker.delegate.selected_profile_name, None);

            assert_eq!(
                cx.try_global::<ActiveSettingsProfileName>()
                    .map(|p| p.0.clone()),
                None
            );

            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(10.0));
        });

        cx.dispatch_action(Confirm);

        cx.update(|_, cx| {
            assert_eq!(cx.try_global::<ActiveSettingsProfileName>(), None);
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(10.0));
        });
    }

    #[gpui::test]
    async fn test_settings_profile_with_user_base(cx: &mut TestAppContext) {
        let user_settings_json = json!({
            "buffer_font_size": 10.0,
            "profiles": {
                "Explicit User": {
                    "base": "user",
                    "settings": {
                        "buffer_font_size": 20.0
                    }
                },
                "Implicit User": {
                    "settings": {
                        "buffer_font_size": 20.0
                    }
                }
            }
        });
        let (workspace, cx) = init_test(user_settings_json, cx).await;

        // Select "Explicit User" (index 1) — profile applies on top of user settings.
        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);
        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(
                picker.delegate.selected_profile_name.as_deref(),
                Some("Explicit User")
            );
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(20.0));
        });

        cx.dispatch_action(Confirm);

        // Select "Implicit User" (index 2) — no base specified, same behavior.
        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);
        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(
                picker.delegate.selected_profile_name.as_deref(),
                Some("Implicit User")
            );
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(20.0));
        });

        cx.dispatch_action(Confirm);
    }

    #[gpui::test]
    async fn test_settings_profile_with_default_base(cx: &mut TestAppContext) {
        let user_settings_json = json!({
            "buffer_font_size": 10.0,
            "profiles": {
                "Clean Slate": {
                    "base": "default"
                },
                "Custom on Defaults": {
                    "base": "default",
                    "settings": {
                        "buffer_font_size": 30.0
                    }
                }
            }
        });
        let (workspace, cx) = init_test(user_settings_json, cx).await;

        // User has buffer_font_size: 10, factory default is 15.
        cx.update(|_, cx| {
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(10.0));
        });

        // "Clean Slate" has base: "default" with no settings overrides,
        // so we get the factory default (15), not the user's value (10).
        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);
        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(
                picker.delegate.selected_profile_name.as_deref(),
                Some("Clean Slate")
            );
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(15.0));
        });

        // "Custom on Defaults" has base: "default" with buffer_font_size: 30,
        // so the profile's override (30) applies on top of the factory default,
        // not on top of the user's value (10).
        cx.dispatch_action(SelectNext);

        picker.read_with(cx, |picker, cx| {
            assert_eq!(
                picker.delegate.selected_profile_name.as_deref(),
                Some("Custom on Defaults")
            );
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(30.0));
        });

        cx.dispatch_action(Confirm);

        cx.update(|_, cx| {
            assert_eq!(ThemeSettings::get_global(cx).buffer_font_size(cx), px(30.0));
        });
    }

    #[gpui::test]
    async fn test_settings_profile_selector_is_in_user_configuration_order(
        cx: &mut TestAppContext,
    ) {
        // Must be unique names (HashMap)
        let user_settings_json = json!({
            "profiles": {
                "z": { "settings": {} },
                "e": { "settings": {} },
                "d": { "settings": {} },
                " ": { "settings": {} },
                "r": { "settings": {} },
                "u": { "settings": {} },
                "l": { "settings": {} },
                "3": { "settings": {} },
                "s": { "settings": {} },
                "!": { "settings": {} },
            }
        });
        let (workspace, cx) = init_test(user_settings_json, cx).await;

        cx.dispatch_action(settings_profile_selector::Toggle);
        let picker = active_settings_profile_picker(&workspace, cx);

        picker.read_with(cx, |picker, _| {
            assert_eq!(picker.delegate.matches.len(), 11);
            assert_eq!(picker.delegate.matches[0].string, display_name(&None));
            assert_eq!(picker.delegate.matches[1].string, "z");
            assert_eq!(picker.delegate.matches[2].string, "e");
            assert_eq!(picker.delegate.matches[3].string, "d");
            assert_eq!(picker.delegate.matches[4].string, " ");
            assert_eq!(picker.delegate.matches[5].string, "r");
            assert_eq!(picker.delegate.matches[6].string, "u");
            assert_eq!(picker.delegate.matches[7].string, "l");
            assert_eq!(picker.delegate.matches[8].string, "3");
            assert_eq!(picker.delegate.matches[9].string, "s");
            assert_eq!(picker.delegate.matches[10].string, "!");
        });
    }
}
