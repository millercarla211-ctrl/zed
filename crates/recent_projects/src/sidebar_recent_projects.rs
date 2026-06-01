use std::{path::PathBuf, sync::Arc};

use fuzzy_nucleo::{StringMatch, StringMatchCandidate, match_strings};
use gpui::{
    Action, AnyElement, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    Subscription, Task, TaskExt, WeakEntity, Window,
};
use picker::{
    Picker, PickerDelegate,
    highlighted_match_with_paths::{HighlightedMatch, HighlightedMatchWithPaths},
};
use remote::RemoteConnectionOptions;
use settings::Settings;
use ui::{ButtonLike, KeyBinding, ListItem, ListItemSpacing, Tooltip, prelude::*};
use ui_input::ErasedEditor;
use util::{ResultExt, paths::PathExt};
use workspace::{
    MultiWorkspace, OpenMode, OpenOptions, ProjectGroupKey, RecentWorkspace,
    SerializedWorkspaceLocation, Workspace, WorkspaceDb, notifications::DetachAndPromptErr,
};

use zed_actions::OpenRemote;

use crate::{highlights_for_path, icon_for_remote_connection, open_remote_project};

const MAX_SIDEBAR_RECENT_PROJECT_CANDIDATES: usize = 2_000;
const MAX_SIDEBAR_RECENT_PROJECT_MATCHES: usize = 100;
const MAX_SIDEBAR_RECENT_PROJECT_CANDIDATE_PATHS: usize = 64;
const MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS: usize = 16;
const SIDEBAR_RECENT_PROJECT_TRUNCATED_PATH_LABEL: &str = "...";

fn sidebar_recent_project_candidate_string(workspace: &RecentWorkspace) -> String {
    workspace
        .identity_paths
        .ordered_paths()
        .take(MAX_SIDEBAR_RECENT_PROJECT_CANDIDATE_PATHS)
        .map(|path| path.compact().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("")
}

fn capped_sidebar_recent_project_paths(workspace: &RecentWorkspace) -> (Vec<PathBuf>, bool) {
    let mut paths = workspace
        .identity_paths
        .ordered_paths()
        .take(MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS.saturating_add(1))
        .map(|path| path.compact())
        .collect::<Vec<_>>();
    let paths_truncated = paths.len() > MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS;
    paths.truncate(MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS);
    (paths, paths_truncated)
}

fn sidebar_recent_project_tooltip_path(
    rendered_paths: &[PathBuf],
    paths_truncated: bool,
    location: &SerializedWorkspaceLocation,
) -> SharedString {
    let mut path_labels = rendered_paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if paths_truncated {
        path_labels.push(SIDEBAR_RECENT_PROJECT_TRUNCATED_PATH_LABEL.to_string());
    }

    let path_label = path_labels.join("\n");
    match location {
        SerializedWorkspaceLocation::Remote(options) => {
            let host = options.display_name();
            if path_labels.len() == 1 {
                format!("{} ({})", path_labels[0], host).into()
            } else {
                format!("{}\n({})", path_label, host).into()
            }
        }
        _ => path_label.into(),
    }
}

pub struct SidebarRecentProjects {
    pub picker: Entity<Picker<SidebarRecentProjectsDelegate>>,
    _subscription: Subscription,
}

impl SidebarRecentProjects {
    pub fn popover(
        workspace: WeakEntity<Workspace>,
        window_project_groups: Vec<ProjectGroupKey>,
        _focus_handle: FocusHandle,
        _multi_workspace: Option<WeakEntity<MultiWorkspace>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let fs = workspace
            .upgrade()
            .map(|ws| ws.read(cx).app_state().fs.clone());

        cx.new(|cx| {
            let delegate = SidebarRecentProjectsDelegate {
                workspace,
                window_project_groups,
                workspaces: Vec::new(),
                filtered_workspaces: Vec::new(),
                selected_index: 0,
                has_any_non_local_projects: false,
                focus_handle: cx.focus_handle(),
            };

            let picker: Entity<Picker<SidebarRecentProjectsDelegate>> = cx.new(|cx| {
                Picker::list(delegate, window, cx)
                    .list_measure_all()
                    .show_scrollbar(true)
            });

            let picker_focus_handle = picker.focus_handle(cx);
            picker.update(cx, |picker, _| {
                picker.delegate.focus_handle = picker_focus_handle;
            });

            let _subscription =
                cx.subscribe(&picker, |_this: &mut Self, _, _, cx| cx.emit(DismissEvent));

            let db = WorkspaceDb::global(cx);
            cx.spawn_in(window, async move |this, cx| {
                let Some(fs) = fs else { return };
                let workspaces = db
                    .recent_project_workspaces(fs.as_ref())
                    .await
                    .log_err()
                    .unwrap_or_default();
                this.update_in(cx, move |this, window, cx| {
                    this.picker.update(cx, move |picker, cx| {
                        picker.delegate.set_workspaces(workspaces);
                        picker.update_matches(picker.query(cx), window, cx)
                    })
                })
                .ok();
            })
            .detach();

            picker.focus_handle(cx).focus(window, cx);

            Self {
                picker,
                _subscription,
            }
        })
    }
}

impl EventEmitter<DismissEvent> for SidebarRecentProjects {}

impl Focusable for SidebarRecentProjects {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for SidebarRecentProjects {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("SidebarRecentProjects")
            .w(rems(18.))
            .child(self.picker.clone())
    }
}

pub struct SidebarRecentProjectsDelegate {
    workspace: WeakEntity<Workspace>,
    window_project_groups: Vec<ProjectGroupKey>,
    workspaces: Vec<RecentWorkspace>,
    filtered_workspaces: Vec<StringMatch>,
    selected_index: usize,
    has_any_non_local_projects: bool,
    focus_handle: FocusHandle,
}

impl SidebarRecentProjectsDelegate {
    pub fn set_workspaces(&mut self, workspaces: Vec<RecentWorkspace>) {
        self.has_any_non_local_projects = workspaces
            .iter()
            .any(|workspace| !matches!(workspace.location, SerializedWorkspaceLocation::Local));
        self.workspaces = workspaces;
    }

    fn clamp_selected_index(&mut self) {
        match self.filtered_workspaces.len().checked_sub(1) {
            Some(max_index) => {
                self.selected_index = self.selected_index.min(max_index);
            }
            None => self.selected_index = 0,
        }
    }
}

impl EventEmitter<DismissEvent> for SidebarRecentProjectsDelegate {}

impl PickerDelegate for SidebarRecentProjectsDelegate {
    type ListItem = AnyElement;

    fn placeholder_text(&self, _window: &mut Window, _cx: &mut App) -> Arc<str> {
        "Search recent projects…".into()
    }

    fn render_editor(
        &self,
        editor: &Arc<dyn ErasedEditor>,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Div {
        h_flex()
            .flex_none()
            .h_9()
            .px_2p5()
            .justify_between()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .child(editor.render(window, cx))
    }

    fn match_count(&self) -> usize {
        self.filtered_workspaces.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) {
        self.selected_index = ix;
        self.clamp_selected_index();
    }

    fn update_matches(
        &mut self,
        query: String,
        _: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        let query = query.trim_start();
        let case = fuzzy_nucleo::Case::smart_if_uppercase_in(query);
        let is_empty_query = query.is_empty();

        let current_workspace_id = self
            .workspace
            .upgrade()
            .and_then(|ws| ws.read(cx).database_id());

        let candidates: Vec<_> = self
            .workspaces
            .iter()
            .enumerate()
            .filter(|(_, workspace)| {
                Some(workspace.workspace_id) != current_workspace_id
                    && !self
                        .window_project_groups
                        .iter()
                        .any(|key| key.matches(&workspace.project_group_key()))
            })
            .take(MAX_SIDEBAR_RECENT_PROJECT_CANDIDATES)
            .map(|(id, workspace)| {
                let combined_string = sidebar_recent_project_candidate_string(workspace);
                StringMatchCandidate::new(id, &combined_string)
            })
            .collect();

        self.filtered_workspaces = if is_empty_query {
            candidates
                .into_iter()
                .take(MAX_SIDEBAR_RECENT_PROJECT_MATCHES)
                .map(|candidate| StringMatch {
                    candidate_id: candidate.id,
                    score: 0.0,
                    positions: Vec::new(),
                    string: candidate.string,
                })
                .collect()
        } else {
            match_strings(
                &candidates,
                query,
                case,
                fuzzy_nucleo::LengthPenalty::On,
                MAX_SIDEBAR_RECENT_PROJECT_MATCHES,
            )
        };

        self.selected_index = 0;
        self.clamp_selected_index();
        Task::ready(())
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(hit) = self.filtered_workspaces.get(self.selected_index) else {
            return;
        };
        let Some(recent_workspace) = self.workspaces.get(hit.candidate_id) else {
            return;
        };

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        match &recent_workspace.location {
            SerializedWorkspaceLocation::Local => {
                if let Some(handle) = window.window_handle().downcast::<MultiWorkspace>() {
                    let paths = recent_workspace.paths.paths().to_vec();
                    cx.defer(move |cx| {
                        if let Some(task) = handle
                            .update(cx, |multi_workspace, window, cx| {
                                multi_workspace.open_project(paths, OpenMode::Activate, window, cx)
                            })
                            .log_err()
                        {
                            task.detach_and_log_err(cx);
                        }
                    });
                }
            }
            SerializedWorkspaceLocation::Remote(connection) => {
                let mut connection = connection.clone();
                workspace.update(cx, |workspace, cx| {
                    let app_state = workspace.app_state().clone();
                    let replace_window = window.window_handle().downcast::<MultiWorkspace>();
                    let open_options = OpenOptions {
                        requesting_window: replace_window,
                        ..Default::default()
                    };
                    if let RemoteConnectionOptions::Ssh(connection) = &mut connection {
                        crate::RemoteSettings::get_global(cx)
                            .fill_connection_options_from_settings(connection);
                    };
                    let paths = recent_workspace.paths.paths().to_vec();
                    cx.spawn_in(window, async move |_, cx| {
                        open_remote_project(connection.clone(), paths, app_state, open_options, cx)
                            .await
                    })
                    .detach_and_prompt_err(
                        "Failed to open project",
                        window,
                        cx,
                        |_, _, _| None,
                    );
                });
            }
        }
        cx.emit(DismissEvent);
    }

    fn dismissed(&mut self, _window: &mut Window, _cx: &mut Context<Picker<Self>>) {}

    fn no_matches_text(&self, _window: &mut Window, _cx: &mut App) -> Option<SharedString> {
        let text = if self.workspaces.is_empty() {
            "Recently opened projects will show up here"
        } else {
            "No matches"
        };
        Some(text.into())
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let hit = self.filtered_workspaces.get(ix)?;
        let workspace = self.workspaces.get(hit.candidate_id)?;

        let (rendered_paths, paths_truncated) = capped_sidebar_recent_project_paths(workspace);
        let tooltip_path = sidebar_recent_project_tooltip_path(
            &rendered_paths,
            paths_truncated,
            &workspace.location,
        );

        let mut path_start_offset = 0;
        let mut match_labels: Vec<_> = rendered_paths
            .iter()
            .map(|path| {
                let (label, path_match) =
                    highlights_for_path(path.as_ref(), &hit.positions, path_start_offset);
                path_start_offset += path_match.text.len();
                label
            })
            .collect();
        if paths_truncated {
            match_labels.push(Some(HighlightedMatch {
                text: SIDEBAR_RECENT_PROJECT_TRUNCATED_PATH_LABEL.to_string(),
                highlight_positions: Vec::new(),
                color: Color::Default,
            }));
        }

        let prefix = match &workspace.location {
            SerializedWorkspaceLocation::Remote(options) => {
                Some(SharedString::from(options.display_name()))
            }
            _ => None,
        };

        let highlighted_match = HighlightedMatchWithPaths {
            prefix,
            match_label: HighlightedMatch::join(match_labels.into_iter().flatten(), ", "),
            paths: Vec::new(),
            active: false,
        };

        let icon = icon_for_remote_connection(match &workspace.location {
            SerializedWorkspaceLocation::Local => None,
            SerializedWorkspaceLocation::Remote(options) => Some(options),
        });

        Some(
            ListItem::new(ix)
                .toggle_state(selected)
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .child(
                    h_flex()
                        .gap_3()
                        .flex_grow_1()
                        .when(self.has_any_non_local_projects, |this| {
                            this.child(Icon::new(icon).color(Color::Muted))
                        })
                        .child(highlighted_match.render(window, cx)),
                )
                .tooltip(move |_, cx| {
                    Tooltip::with_meta(
                        "Open Project in This Window",
                        None,
                        tooltip_path.clone(),
                        cx,
                    )
                })
                .into_any_element(),
        )
    }

    fn render_footer(&self, _: &mut Window, cx: &mut Context<Picker<Self>>) -> Option<AnyElement> {
        let focus_handle = self.focus_handle.clone();

        Some(
            v_flex()
                .p_1p5()
                .flex_1()
                .gap_1()
                .border_t_1()
                .border_color(cx.theme().colors().border_variant)
                .child({
                    let open_action = workspace::Open {
                        create_new_window: false,
                    };

                    ButtonLike::new("open_local_folder")
                        .child(
                            h_flex()
                                .w_full()
                                .gap_1()
                                .justify_between()
                                .child(Label::new("Add Local Folders"))
                                .child(KeyBinding::for_action_in(&open_action, &focus_handle, cx)),
                        )
                        .on_click(cx.listener(move |_, _, window, cx| {
                            window.dispatch_action(open_action.boxed_clone(), cx);
                            cx.emit(DismissEvent);
                        }))
                })
                .child(
                    ButtonLike::new("open_remote_folder")
                        .child(
                            h_flex()
                                .w_full()
                                .gap_1()
                                .justify_between()
                                .child(Label::new("Add Remote Folder"))
                                .child(KeyBinding::for_action(
                                    &OpenRemote {
                                        from_existing_connection: false,
                                        create_new_window: false,
                                    },
                                    cx,
                                )),
                        )
                        .on_click(cx.listener(|_, _, window, cx| {
                            window.dispatch_action(
                                OpenRemote {
                                    from_existing_connection: false,
                                    create_new_window: false,
                                }
                                .boxed_clone(),
                                cx,
                            );
                            cx.emit(DismissEvent);
                        })),
                )
                .into_any(),
        )
    }
}
