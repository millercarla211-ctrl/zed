use std::{path::PathBuf, sync::Arc};

use fs::Fs;
use gpui::{AppContext, Entity, Global, MenuItem};
use smallvec::SmallVec;
use ui::{App, Context};
use util::{ResultExt, paths::PathExt};

use crate::{
    NewWindow, SerializedWorkspaceLocation, WorkspaceId, path_list::PathList,
    persistence::WorkspaceDb,
};

const MAX_HISTORY_ENTRIES: usize = 256;
const MAX_HISTORY_DB_RECENT_WORKSPACE_ROWS: usize = MAX_HISTORY_ENTRIES * 4;
const MAX_JUMP_LIST_ENTRIES: usize = 64;
const MAX_JUMP_LIST_REMOVED_ENTRIES: usize = MAX_JUMP_LIST_ENTRIES;
const MAX_HISTORY_DELETION_IDS: usize = MAX_HISTORY_ENTRIES;
const MAX_HISTORY_ENTRY_PATHS: usize = 32;

pub fn init(fs: Arc<dyn Fs>, cx: &mut App) {
    let manager = cx.new(|_| HistoryManager::new());
    HistoryManager::set_global(manager.clone(), cx);
    HistoryManager::init(manager, fs, cx);
}

pub struct HistoryManager {
    /// The history of workspaces that have been opened in the past, in reverse order.
    /// The most recent workspace is at the end of the vector.
    history: Vec<HistoryManagerEntry>,
}

#[derive(Debug)]
pub struct HistoryManagerEntry {
    pub id: WorkspaceId,
    pub path: SmallVec<[PathBuf; 2]>,
}

struct GlobalHistoryManager(Entity<HistoryManager>);

impl Global for GlobalHistoryManager {}

impl HistoryManager {
    fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    fn init(this: Entity<HistoryManager>, fs: Arc<dyn Fs>, cx: &App) {
        let db = WorkspaceDb::global(cx);
        cx.spawn(async move |cx| {
            let mut recent_folders = db
                .recent_project_workspaces_limited(
                    fs.as_ref(),
                    MAX_HISTORY_DB_RECENT_WORKSPACE_ROWS,
                )
                .await
                .unwrap_or_default()
                .into_iter()
                .filter_map(|workspace| {
                    if matches!(workspace.location, SerializedWorkspaceLocation::Local) {
                        Some(HistoryManagerEntry::new(
                            workspace.workspace_id,
                            &workspace.paths,
                        ))
                    } else {
                        None
                    }
                })
                .take(MAX_HISTORY_ENTRIES)
                .collect::<Vec<_>>();
            recent_folders.reverse();
            this.update(cx, |this, cx| {
                this.history = recent_folders;
                this.update_jump_list(cx);
            })
        })
        .detach();
    }

    pub fn global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalHistoryManager>()
            .map(|model| model.0.clone())
    }

    fn set_global(history_manager: Entity<Self>, cx: &mut App) {
        cx.set_global(GlobalHistoryManager(history_manager));
    }

    pub fn update_history(
        &mut self,
        id: WorkspaceId,
        entry: HistoryManagerEntry,
        cx: &mut Context<'_, HistoryManager>,
    ) {
        if let Some(pos) = self.history.iter().position(|e| e.id == id) {
            self.history.remove(pos);
        }
        self.history.push(entry);
        if self.history.len() > MAX_HISTORY_ENTRIES {
            let overflow = self.history.len() - MAX_HISTORY_ENTRIES;
            self.history.drain(..overflow);
        }
        self.update_jump_list(cx);
    }

    pub fn delete_history(&mut self, id: WorkspaceId, cx: &mut Context<'_, HistoryManager>) {
        let Some(pos) = self.history.iter().position(|e| e.id == id) else {
            return;
        };
        self.history.remove(pos);
        self.update_jump_list(cx);
    }

    fn update_jump_list(&mut self, cx: &mut Context<'_, HistoryManager>) {
        let menus = vec![MenuItem::action("New Window", NewWindow)];
        let entries = self
            .history
            .iter()
            .rev()
            .take(MAX_JUMP_LIST_ENTRIES)
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        let user_removed = cx.update_jump_list(menus, entries);
        let db = WorkspaceDb::global(cx);
        cx.spawn(async move |this, cx| {
            let user_removed = user_removed.await;
            if user_removed.is_empty() {
                return;
            }
            if user_removed.len() > MAX_JUMP_LIST_REMOVED_ENTRIES {
                Err::<(), _>(anyhow::anyhow!(
                    "refusing to process oversized jump-list removal payload ({} entries; max {})",
                    user_removed.len(),
                    MAX_JUMP_LIST_REMOVED_ENTRIES
                ))
                .log_err();
                return;
            }

            let mut deleted_ids =
                Vec::with_capacity(user_removed.len().min(MAX_HISTORY_DELETION_IDS));
            if let Ok(()) = this.update(cx, |this, _| {
                for idx in (0..this.history.len()).rev() {
                    if let Some(entry) = this.history.get(idx)
                        && user_removed.contains(&entry.path)
                    {
                        if deleted_ids.len() >= MAX_HISTORY_DELETION_IDS {
                            Err::<(), _>(anyhow::anyhow!(
                                "workspace history deletion id list reached cap (max {})",
                                MAX_HISTORY_DELETION_IDS
                            ))
                            .log_err();
                            break;
                        }
                        deleted_ids.push(entry.id);
                        this.history.remove(idx);
                    }
                }
            }) {
                for id in deleted_ids.iter() {
                    db.delete_workspace_by_id(*id).await.log_err();
                }
            }
        })
        .detach();
    }
}

impl HistoryManagerEntry {
    pub fn new(id: WorkspaceId, paths: &PathList) -> Self {
        let capped_path_count = paths.paths().len().min(MAX_HISTORY_ENTRY_PATHS);
        let mut path = SmallVec::new();
        if paths.paths().len() > MAX_HISTORY_ENTRY_PATHS {
            Err::<(), _>(anyhow::anyhow!(
                "workspace history entry path list is too large ({} paths; max {})",
                paths.paths().len(),
                MAX_HISTORY_ENTRY_PATHS
            ))
            .log_err();
        }

        for original_index in 0..capped_path_count {
            if let Some(source_path) = paths
                .order()
                .iter()
                .zip(paths.paths())
                .find_map(|(order, source_path)| (*order == original_index).then_some(source_path))
            {
                path.push(source_path.compact());
            }
        }

        Self { id, path }
    }
}
