use std::path::{Path, PathBuf};

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use collections::{HashMap, HashSet};
use db::{
    sqlez::{
        bindable::Column, domain::Domain, statement::Statement,
        thread_safe_connection::ThreadSafeConnection,
    },
    sqlez_macros::sql,
};
use futures::{FutureExt, future::Shared};
use gpui::{AppContext as _, Entity, Global, Task};
use remote::{RemoteConnectionOptions, same_remote_connection_identity};
use ui::{App, Context, SharedString};
use util::{ResultExt as _, truncate_to_byte_limit};
use workspace::PathList;

use crate::{TerminalId, thread_metadata_store::WorktreePaths};

pub fn init(cx: &mut App) {
    TerminalThreadMetadataStore::init_global(cx);
}

struct GlobalTerminalThreadMetadataStore(Entity<TerminalThreadMetadataStore>);
impl Global for GlobalTerminalThreadMetadataStore {}

#[cfg(any(test, feature = "test-support"))]
pub struct TestTerminalMetadataDbName(pub String);
#[cfg(any(test, feature = "test-support"))]
impl Global for TestTerminalMetadataDbName {}

#[cfg(any(test, feature = "test-support"))]
impl TestTerminalMetadataDbName {
    pub fn global(cx: &App) -> String {
        cx.try_global::<Self>()
            .map(|global| global.0.clone())
            .unwrap_or_else(|| {
                let thread = std::thread::current();
                let test_name = thread.name().unwrap_or("unknown_test");
                format!("TERMINAL_THREAD_METADATA_DB_{}", test_name)
            })
    }
}

const MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES: usize = 64 * 1024;
const MAX_TERMINAL_THREAD_METADATA_DB_ROWS: usize = 10_000;
const MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS: usize = 2_048;
const MAX_TERMINAL_THREAD_METADATA_STRING_BYTES: usize = 16 * 1024;
const MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES: usize = 512;
const MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES: usize = 256 * 1024;

fn terminal_thread_metadata_db_list_limit() -> i64 {
    MAX_TERMINAL_THREAD_METADATA_DB_ROWS
        .saturating_add(1)
        .min(i64::MAX as usize) as i64
}

fn bounded_terminal_metadata_rows(
    mut rows: Vec<TerminalThreadMetadata>,
) -> Vec<TerminalThreadMetadata> {
    if rows.len() > MAX_TERMINAL_THREAD_METADATA_DB_ROWS {
        let original_len = rows.len();
        rows.truncate(MAX_TERMINAL_THREAD_METADATA_DB_ROWS);
        log::warn!(
            "terminal thread metadata database load capped at {} rows; skipped at least {} older rows",
            MAX_TERMINAL_THREAD_METADATA_DB_ROWS,
            original_len.saturating_sub(MAX_TERMINAL_THREAD_METADATA_DB_ROWS)
        );
    }

    rows
}

fn bounded_terminal_metadata_text(
    terminal_id: TerminalId,
    context: &str,
    field_name: &str,
    text: &str,
) -> Option<String> {
    if text.len() <= MAX_TERMINAL_THREAD_METADATA_STRING_BYTES {
        return None;
    }

    let truncated =
        truncate_to_byte_limit(text, MAX_TERMINAL_THREAD_METADATA_STRING_BYTES).to_string();
    log::warn!(
        "terminal thread metadata {context} {field_name} truncated for terminal {} ({} bytes; max {} bytes)",
        terminal_id.to_key_string(),
        text.len(),
        MAX_TERMINAL_THREAD_METADATA_STRING_BYTES
    );
    Some(truncated)
}

fn bounded_terminal_metadata_shared_string(
    terminal_id: TerminalId,
    context: &str,
    field_name: &str,
    text: SharedString,
) -> SharedString {
    bounded_terminal_metadata_text(terminal_id, context, field_name, text.as_ref())
        .map(SharedString::from)
        .unwrap_or(text)
}

fn bounded_terminal_metadata_working_directory(
    terminal_id: TerminalId,
    context: &str,
    working_directory: Option<PathBuf>,
) -> Option<PathBuf> {
    let working_directory = working_directory?;
    let path_len = working_directory.to_string_lossy().len();
    if path_len <= MAX_TERMINAL_THREAD_METADATA_STRING_BYTES {
        return Some(working_directory);
    }

    log::warn!(
        "terminal thread metadata {context} working_directory skipped for terminal {} ({} bytes; max {} bytes)",
        terminal_id.to_key_string(),
        path_len,
        MAX_TERMINAL_THREAD_METADATA_STRING_BYTES
    );
    None
}

fn bounded_terminal_metadata_worktree_paths(
    terminal_id: TerminalId,
    context: &str,
    worktree_paths: WorktreePaths,
) -> WorktreePaths {
    let pair_count = worktree_paths.ordered_pairs().count();
    if pair_count == 0 {
        return worktree_paths;
    }

    let mut main_paths = Vec::new();
    let mut folder_paths = Vec::new();
    let mut total_path_bytes = 0usize;
    let mut skipped_pairs = 0usize;
    let mut capped = false;

    for (index, (main_path, folder_path)) in worktree_paths.ordered_pairs().enumerate() {
        if index >= MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES {
            skipped_pairs += pair_count.saturating_sub(index);
            capped = true;
            break;
        }

        let main_path_text = main_path.to_string_lossy();
        let folder_path_text = folder_path.to_string_lossy();
        if main_path_text.len() > MAX_TERMINAL_THREAD_METADATA_STRING_BYTES
            || folder_path_text.len() > MAX_TERMINAL_THREAD_METADATA_STRING_BYTES
        {
            skipped_pairs += 1;
            capped = true;
            continue;
        }

        let pair_bytes = main_path_text
            .len()
            .saturating_add(folder_path_text.len())
            .saturating_add(2);
        if total_path_bytes.saturating_add(pair_bytes)
            > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        {
            skipped_pairs += pair_count.saturating_sub(index);
            capped = true;
            break;
        }

        total_path_bytes = total_path_bytes.saturating_add(pair_bytes);
        main_paths.push(main_path.clone());
        folder_paths.push(folder_path.clone());
    }

    if !capped {
        return worktree_paths;
    }

    log::warn!(
        "terminal thread metadata {context} path list capped for terminal {} (kept {} of {} pairs; skipped {}; max {} pairs, {} bytes)",
        terminal_id.to_key_string(),
        folder_paths.len(),
        pair_count,
        skipped_pairs,
        MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES,
        MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
    );

    WorktreePaths::from_path_lists(PathList::new(&main_paths), PathList::new(&folder_paths))
        .unwrap_or_else(|error| {
            log::warn!(
                "terminal thread metadata {context} path list skipped for terminal {} after capping: {error}",
                terminal_id.to_key_string()
            );
            WorktreePaths::default()
        })
}

fn bounded_terminal_metadata(
    mut metadata: TerminalThreadMetadata,
    context: &str,
) -> TerminalThreadMetadata {
    let terminal_id = metadata.terminal_id;
    metadata.title =
        bounded_terminal_metadata_shared_string(terminal_id, context, "title", metadata.title);
    metadata.custom_title = metadata.custom_title.map(|custom_title| {
        bounded_terminal_metadata_shared_string(terminal_id, context, "custom_title", custom_title)
    });
    metadata.working_directory = bounded_terminal_metadata_working_directory(
        terminal_id,
        context,
        metadata.working_directory,
    );
    metadata.worktree_paths =
        bounded_terminal_metadata_worktree_paths(terminal_id, context, metadata.worktree_paths);
    metadata
}

fn serialized_path_list_entry_count(paths: &str) -> usize {
    if paths.is_empty() {
        0
    } else {
        paths.bytes().filter(|byte| *byte == b'\n').count() + 1
    }
}

fn deserialize_bounded_terminal_path_list(
    terminal_id: &str,
    field_name: &str,
    paths: Option<String>,
    order: Option<String>,
) -> PathList {
    let Some(paths) = paths else {
        return PathList::default();
    };
    let order = order.unwrap_or_default();
    let entry_count = serialized_path_list_entry_count(&paths);

    if paths.len() > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        || order.len() > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        || entry_count > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES
    {
        log::warn!(
            "terminal thread metadata database load {field_name} skipped for terminal {terminal_id} ({} entries, {} path bytes, {} order bytes; max {} entries, {} bytes)",
            entry_count,
            paths.len(),
            order.len(),
            MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES,
            MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        );
        return PathList::default();
    }

    PathList::deserialize(&util::path_list::SerializedPathList { paths, order })
}

fn serialize_bounded_terminal_path_list(
    terminal_id: TerminalId,
    field_name: &str,
    path_list: &PathList,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    if path_list.is_empty() {
        return Ok((None, None));
    }

    let serialized = path_list.serialize();
    let entry_count = serialized_path_list_entry_count(&serialized.paths);
    if serialized.paths.len() > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        || serialized.order.len() > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        || entry_count > MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES
    {
        anyhow::bail!(
            "serialize terminal thread metadata {field_name} for terminal {}: path list is too large ({} entries, {} path bytes, {} order bytes; max {} entries, {} bytes)",
            terminal_id.to_key_string(),
            entry_count,
            serialized.paths.len(),
            serialized.order.len(),
            MAX_TERMINAL_THREAD_METADATA_PATH_LIST_ENTRIES,
            MAX_TERMINAL_THREAD_METADATA_PATH_LIST_BYTES
        );
    }

    Ok((Some(serialized.paths), Some(serialized.order)))
}

fn deserialize_terminal_thread_remote_connection(
    remote_connection_json: &str,
) -> anyhow::Result<RemoteConnectionOptions> {
    if remote_connection_json.len() > MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES {
        anyhow::bail!(
            "deserialize terminal thread remote connection: remote_connection_json is too large ({} bytes; max {} bytes)",
            remote_connection_json.len(),
            MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES
        );
    }

    serde_json::from_str::<RemoteConnectionOptions>(remote_connection_json)
        .context("deserialize terminal thread remote connection")
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalThreadMetadata {
    pub terminal_id: TerminalId,
    pub title: SharedString,
    pub custom_title: Option<SharedString>,
    pub created_at: DateTime<Utc>,
    pub worktree_paths: WorktreePaths,
    pub remote_connection: Option<RemoteConnectionOptions>,
    pub working_directory: Option<PathBuf>,
}

impl TerminalThreadMetadata {
    pub fn folder_paths(&self) -> &PathList {
        self.worktree_paths.folder_path_list()
    }

    pub fn main_worktree_paths(&self) -> &PathList {
        self.worktree_paths.main_worktree_path_list()
    }
}

pub struct TerminalThreadMetadataStore {
    db: TerminalThreadMetadataDb,
    terminals: HashMap<TerminalId, TerminalThreadMetadata>,
    terminals_by_paths: HashMap<PathList, HashSet<TerminalId>>,
    terminals_by_main_paths: HashMap<PathList, HashSet<TerminalId>>,
    reload_task: Option<Shared<Task<()>>>,
    pending_terminal_ops_tx: async_channel::Sender<DbOperation>,
    _db_operations_task: Task<()>,
}

#[derive(Debug, PartialEq)]
enum DbOperation {
    Upsert(TerminalThreadMetadata),
    Delete(TerminalId),
}

impl DbOperation {
    fn id(&self) -> TerminalId {
        match self {
            DbOperation::Upsert(metadata) => metadata.terminal_id,
            DbOperation::Delete(terminal_id) => *terminal_id,
        }
    }
}

impl TerminalThreadMetadataStore {
    #[cfg(not(any(test, feature = "test-support")))]
    pub fn init_global(cx: &mut App) {
        if cx.has_global::<GlobalTerminalThreadMetadataStore>() {
            return;
        }

        let db = TerminalThreadMetadataDb::global(cx);
        let terminal_store = cx.new(|cx| Self::new(db, cx));
        cx.set_global(GlobalTerminalThreadMetadataStore(terminal_store));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn init_global(cx: &mut App) {
        let db_name = TestTerminalMetadataDbName::global(cx);
        let db = gpui::block_on(db::open_test_db::<TerminalThreadMetadataDb>(&db_name));
        let terminal_store = cx.new(|cx| Self::new(TerminalThreadMetadataDb(db), cx));
        cx.set_global(GlobalTerminalThreadMetadataStore(terminal_store));
    }

    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalTerminalThreadMetadataStore>()
            .map(|store| store.0.clone())
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalTerminalThreadMetadataStore>().0.clone()
    }

    pub fn entry(&self, terminal_id: TerminalId) -> Option<&TerminalThreadMetadata> {
        self.terminals.get(&terminal_id)
    }

    pub fn entries(&self) -> impl Iterator<Item = &TerminalThreadMetadata> + '_ {
        self.terminals.values()
    }

    pub fn reload_task(&self) -> Shared<Task<()>> {
        self.reload_task
            .clone()
            .unwrap_or_else(|| Task::ready(()).shared())
    }

    pub fn entries_for_path<'a>(
        &'a self,
        path_list: &PathList,
        remote_connection: Option<&'a RemoteConnectionOptions>,
    ) -> impl Iterator<Item = &'a TerminalThreadMetadata> + 'a {
        self.terminals_by_paths
            .get(path_list)
            .into_iter()
            .flatten()
            .filter_map(|id| self.terminals.get(id))
            .filter(move |terminal| {
                same_remote_connection_identity(
                    terminal.remote_connection.as_ref(),
                    remote_connection,
                )
            })
    }

    pub fn entries_for_main_worktree_path<'a>(
        &'a self,
        path_list: &PathList,
        remote_connection: Option<&'a RemoteConnectionOptions>,
    ) -> impl Iterator<Item = &'a TerminalThreadMetadata> + 'a {
        self.terminals_by_main_paths
            .get(path_list)
            .into_iter()
            .flatten()
            .filter_map(|id| self.terminals.get(id))
            .filter(move |terminal| {
                same_remote_connection_identity(
                    terminal.remote_connection.as_ref(),
                    remote_connection,
                )
            })
    }

    pub fn path_is_referenced_by_terminal(
        &self,
        terminal_id: Option<TerminalId>,
        path: &Path,
        remote_connection: Option<&RemoteConnectionOptions>,
    ) -> bool {
        self.entries().any(|terminal| {
            Some(terminal.terminal_id) != terminal_id
                && same_remote_connection_identity(
                    terminal.remote_connection.as_ref(),
                    remote_connection,
                )
                && terminal
                    .folder_paths()
                    .paths()
                    .iter()
                    .any(|folder_path| folder_path.as_path() == path)
        })
    }

    pub fn save(&mut self, metadata: TerminalThreadMetadata, cx: &mut Context<Self>) {
        self.save_internal(metadata);
        cx.notify();
    }

    pub fn change_worktree_paths(
        &mut self,
        current_folder_paths: &PathList,
        remote_connection: Option<&RemoteConnectionOptions>,
        mutate: impl Fn(&mut WorktreePaths),
        cx: &mut Context<Self>,
    ) {
        let terminal_ids: Vec<_> = self
            .terminals_by_paths
            .get(current_folder_paths)
            .into_iter()
            .flatten()
            .filter(|id| {
                self.terminals.get(id).is_some_and(|terminal| {
                    same_remote_connection_identity(
                        terminal.remote_connection.as_ref(),
                        remote_connection,
                    )
                })
            })
            .copied()
            .collect();

        if terminal_ids.is_empty() {
            return;
        }

        for terminal_id in terminal_ids {
            if let Some(mut terminal) = self.terminals.get(&terminal_id).cloned() {
                mutate(&mut terminal.worktree_paths);
                self.save_internal(terminal);
            }
        }

        cx.notify();
    }

    fn save_internal(&mut self, metadata: TerminalThreadMetadata) {
        let metadata = bounded_terminal_metadata(metadata, "save");
        if let Some(existing) = self.terminals.get(&metadata.terminal_id) {
            if existing.folder_paths() != metadata.folder_paths()
                && let Some(ids) = self.terminals_by_paths.get_mut(existing.folder_paths())
            {
                ids.remove(&metadata.terminal_id);
            }

            if existing.main_worktree_paths() != metadata.main_worktree_paths()
                && let Some(ids) = self
                    .terminals_by_main_paths
                    .get_mut(existing.main_worktree_paths())
            {
                ids.remove(&metadata.terminal_id);
            }
        }

        self.cache_terminal_metadata(metadata.clone());
        self.queue_db_operation(DbOperation::Upsert(metadata));
    }

    fn cache_terminal_metadata(&mut self, metadata: TerminalThreadMetadata) {
        self.terminals
            .insert(metadata.terminal_id, metadata.clone());

        self.terminals_by_paths
            .entry(metadata.folder_paths().clone())
            .or_default()
            .insert(metadata.terminal_id);

        if !metadata.main_worktree_paths().is_empty() {
            self.terminals_by_main_paths
                .entry(metadata.main_worktree_paths().clone())
                .or_default()
                .insert(metadata.terminal_id);
        }
    }

    pub fn delete(&mut self, terminal_id: TerminalId, cx: &mut Context<Self>) {
        if let Some(terminal) = self.terminals.remove(&terminal_id) {
            if let Some(ids) = self.terminals_by_paths.get_mut(terminal.folder_paths()) {
                ids.remove(&terminal_id);
            }
            if !terminal.main_worktree_paths().is_empty()
                && let Some(ids) = self
                    .terminals_by_main_paths
                    .get_mut(terminal.main_worktree_paths())
            {
                ids.remove(&terminal_id);
            }
        }
        self.queue_db_operation(DbOperation::Delete(terminal_id));
        cx.notify();
    }

    fn new(db: TerminalThreadMetadataDb, cx: &mut Context<Self>) -> Self {
        let (tx, rx) = async_channel::bounded(MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS);
        let _db_operations_task = cx.background_spawn({
            let db = db.clone();
            async move {
                while let Ok(first_update) = rx.recv().await {
                    let updates = Self::drain_pending_terminal_db_operations(first_update, &rx);
                    let updates = Self::dedup_db_operations(updates);
                    for operation in updates {
                        match operation {
                            DbOperation::Upsert(metadata) => {
                                db.save(metadata).await.log_err();
                            }
                            DbOperation::Delete(terminal_id) => {
                                db.delete(terminal_id).await.log_err();
                            }
                        }
                    }
                }
            }
        });

        let mut this = Self {
            db,
            terminals: HashMap::default(),
            terminals_by_paths: HashMap::default(),
            terminals_by_main_paths: HashMap::default(),
            reload_task: None,
            pending_terminal_ops_tx: tx,
            _db_operations_task,
        };
        this.reload(cx);
        this
    }

    fn queue_db_operation(&self, operation: DbOperation) {
        let terminal_id = operation.id();
        match self.pending_terminal_ops_tx.try_send(operation) {
            Ok(()) => {}
            Err(async_channel::TrySendError::Full(operation)) => {
                log::warn!(
                    "terminal thread metadata database operation backpressured for terminal {}: pending queue cap {} reached",
                    terminal_id.to_key_string(),
                    MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS
                );
                if self
                    .pending_terminal_ops_tx
                    .send_blocking(operation)
                    .is_err()
                {
                    log::warn!(
                        "terminal thread metadata database operation skipped for terminal {}: pending queue closed",
                        terminal_id.to_key_string()
                    );
                }
            }
            Err(async_channel::TrySendError::Closed(_)) => {
                log::warn!(
                    "terminal thread metadata database operation skipped for terminal {}: pending queue closed",
                    terminal_id.to_key_string()
                );
            }
        }
    }

    fn drain_pending_terminal_db_operations(
        first_update: DbOperation,
        rx: &async_channel::Receiver<DbOperation>,
    ) -> Vec<DbOperation> {
        let mut updates = vec![first_update];
        while updates.len() < MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS {
            let Ok(update) = rx.try_recv() else {
                break;
            };
            updates.push(update);
        }

        let remaining = rx.len();
        if remaining > 0 {
            log::warn!(
                "terminal thread metadata database operation drain capped at {} operations; {remaining} queued operations deferred",
                MAX_TERMINAL_THREAD_METADATA_PENDING_DB_OPERATIONS
            );
        }

        updates
    }

    fn dedup_db_operations(operations: Vec<DbOperation>) -> Vec<DbOperation> {
        let mut ops = HashMap::default();
        for operation in operations.into_iter().rev() {
            if ops.contains_key(&operation.id()) {
                continue;
            }
            ops.insert(operation.id(), operation);
        }
        ops.into_values().collect()
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let db = self.db.clone();
        self.reload_task = Some(
            cx.spawn(async move |this, cx| {
                let rows = cx
                    .background_spawn(async move {
                        db.list()
                            .context("Failed to fetch terminal thread metadata")
                    })
                    .await
                    .log_err()
                    .unwrap_or_default();

                this.update(cx, |this, cx| {
                    this.terminals.clear();
                    this.terminals_by_paths.clear();
                    this.terminals_by_main_paths.clear();

                    let rows = bounded_terminal_metadata_rows(rows);
                    for row in rows {
                        let row = bounded_terminal_metadata(row, "database load");
                        this.cache_terminal_metadata(row);
                    }

                    cx.notify();
                })
                .ok();
            })
            .shared(),
        );
    }
}

struct TerminalThreadMetadataDb(ThreadSafeConnection);

impl Domain for TerminalThreadMetadataDb {
    const NAME: &str = stringify!(TerminalThreadMetadataDb);

    const MIGRATIONS: &[&str] = &[sql!(
        CREATE TABLE IF NOT EXISTS sidebar_terminal_threads(
            terminal_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            custom_title TEXT,
            created_at TEXT NOT NULL,
            working_directory TEXT,
            folder_paths TEXT,
            folder_paths_order TEXT,
            main_worktree_paths TEXT,
            main_worktree_paths_order TEXT,
            remote_connection TEXT
        ) STRICT;
    )];
}

db::static_connection!(TerminalThreadMetadataDb, []);

impl TerminalThreadMetadataDb {
    pub fn list(&self) -> anyhow::Result<Vec<TerminalThreadMetadata>> {
        self.select_bound::<i64, TerminalThreadMetadata>(
            "SELECT terminal_id, title, custom_title, created_at, \
            working_directory, folder_paths, folder_paths_order, main_worktree_paths, \
            main_worktree_paths_order, remote_connection \
            FROM sidebar_terminal_threads \
            ORDER BY created_at DESC \
            LIMIT ?1",
        )?(terminal_thread_metadata_db_list_limit())
    }

    pub async fn save(&self, row: TerminalThreadMetadata) -> anyhow::Result<()> {
        let row = bounded_terminal_metadata(row, "database save");
        let terminal_id = row.terminal_id.to_key_string();
        let title = row.title.to_string();
        let custom_title = row.custom_title.as_ref().map(ToString::to_string);
        let created_at = row.created_at.to_rfc3339();
        let working_directory = row
            .working_directory
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let (folder_paths, folder_paths_order) = serialize_bounded_terminal_path_list(
            row.terminal_id,
            "folder_paths",
            row.folder_paths(),
        )?;
        let (main_worktree_paths, main_worktree_paths_order) =
            serialize_bounded_terminal_path_list(
                row.terminal_id,
                "main_worktree_paths",
                row.main_worktree_paths(),
            )?;
        let remote_connection = match row.remote_connection.as_ref() {
            Some(remote_connection) => {
                let json = serde_json::to_string(remote_connection)
                    .context("serialize terminal thread remote connection")?;
                if json.len() > MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES {
                    anyhow::bail!(
                        "serialize terminal thread remote connection: remote_connection_json is too large ({} bytes; max {} bytes)",
                        json.len(),
                        MAX_TERMINAL_THREAD_REMOTE_CONNECTION_JSON_BYTES
                    );
                }
                Some(json)
            }
            None => None,
        };

        self.write(move |conn| {
            let sql = "INSERT INTO sidebar_terminal_threads(terminal_id, title, custom_title, created_at, working_directory, folder_paths, folder_paths_order, main_worktree_paths, main_worktree_paths_order, remote_connection) \
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                       ON CONFLICT(terminal_id) DO UPDATE SET \
                           title = excluded.title, \
                           custom_title = excluded.custom_title, \
                           created_at = excluded.created_at, \
                           working_directory = excluded.working_directory, \
                           folder_paths = excluded.folder_paths, \
                           folder_paths_order = excluded.folder_paths_order, \
                           main_worktree_paths = excluded.main_worktree_paths, \
                           main_worktree_paths_order = excluded.main_worktree_paths_order, \
                           remote_connection = excluded.remote_connection";
            let mut stmt = Statement::prepare(conn, sql)?;
            let mut i = stmt.bind(&terminal_id, 1)?;
            i = stmt.bind(&title, i)?;
            i = stmt.bind(&custom_title, i)?;
            i = stmt.bind(&created_at, i)?;
            i = stmt.bind(&working_directory, i)?;
            i = stmt.bind(&folder_paths, i)?;
            i = stmt.bind(&folder_paths_order, i)?;
            i = stmt.bind(&main_worktree_paths, i)?;
            i = stmt.bind(&main_worktree_paths_order, i)?;
            stmt.bind(&remote_connection, i)?;
            stmt.exec()
        })
        .await
    }

    pub async fn delete(&self, terminal_id: TerminalId) -> anyhow::Result<()> {
        let terminal_id = terminal_id.to_key_string();
        self.write(move |conn| {
            let mut stmt = Statement::prepare(
                conn,
                "DELETE FROM sidebar_terminal_threads WHERE terminal_id = ?",
            )?;
            stmt.bind(&terminal_id, 1)?;
            stmt.exec()
        })
        .await
    }
}

impl Column for TerminalThreadMetadata {
    fn column(statement: &mut Statement, start_index: i32) -> anyhow::Result<(Self, i32)> {
        let (terminal_id, next): (String, i32) = Column::column(statement, start_index)?;
        let (title, next): (String, i32) = Column::column(statement, next)?;
        let (custom_title, next): (Option<String>, i32) = Column::column(statement, next)?;
        let (created_at, next): (String, i32) = Column::column(statement, next)?;
        let (working_directory, next): (Option<String>, i32) = Column::column(statement, next)?;
        let (folder_paths_str, next): (Option<String>, i32) = Column::column(statement, next)?;
        let (folder_paths_order_str, next): (Option<String>, i32) =
            Column::column(statement, next)?;
        let (main_worktree_paths_str, next): (Option<String>, i32) =
            Column::column(statement, next)?;
        let (main_worktree_paths_order_str, next): (Option<String>, i32) =
            Column::column(statement, next)?;
        let (remote_connection_json, next): (Option<String>, i32) =
            Column::column(statement, next)?;

        let folder_paths = deserialize_bounded_terminal_path_list(
            &terminal_id,
            "folder_paths",
            folder_paths_str,
            folder_paths_order_str,
        );

        let main_worktree_paths = deserialize_bounded_terminal_path_list(
            &terminal_id,
            "main_worktree_paths",
            main_worktree_paths_str,
            main_worktree_paths_order_str,
        );

        let remote_connection = remote_connection_json
            .as_deref()
            .map(deserialize_terminal_thread_remote_connection)
            .transpose()?;

        let worktree_paths = WorktreePaths::from_path_lists(main_worktree_paths, folder_paths)
            .unwrap_or_else(|error| {
                log::warn!(
                    "terminal thread metadata database load path list skipped for terminal {terminal_id}: {error}"
                );
                WorktreePaths::default()
            });
        let terminal_id = TerminalId::from_key_string(&terminal_id)?;

        Ok((
            bounded_terminal_metadata(
                TerminalThreadMetadata {
                    terminal_id,
                    title: SharedString::from(title),
                    custom_title: custom_title
                        .filter(|title| !title.trim().is_empty())
                        .map(SharedString::from),
                    created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                    worktree_paths,
                    remote_connection,
                    working_directory: working_directory.map(PathBuf::from),
                },
                "database row",
            ),
            next,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use std::path::Path;

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            TerminalThreadMetadataStore::init_global(cx);
        });
        cx.run_until_parked();
    }

    fn metadata(title: &str, worktree_paths: WorktreePaths) -> TerminalThreadMetadata {
        let now = Utc::now();
        TerminalThreadMetadata {
            terminal_id: TerminalId::new(),
            title: SharedString::from(title.to_string()),
            custom_title: None,
            created_at: now,
            worktree_paths,
            remote_connection: None,
            working_directory: None,
        }
    }

    #[gpui::test]
    async fn test_change_worktree_paths_reindexes_terminal_metadata(cx: &mut TestAppContext) {
        init_test(cx);

        let old_main_paths = PathList::new(&[Path::new("/repo")]);
        let old_folder_paths = PathList::new(&[Path::new("/repo-feature")]);
        let new_main_path = Path::new("/repo");
        let new_folder_path = Path::new("/repo-feature-renamed");
        let new_folder_paths = PathList::new(&[new_folder_path]);
        let metadata = metadata(
            "Dev Server",
            WorktreePaths::from_path_lists(old_main_paths.clone(), old_folder_paths.clone())
                .unwrap(),
        );
        let terminal_id = metadata.terminal_id;

        cx.update(|cx| {
            TerminalThreadMetadataStore::global(cx).update(cx, |store, cx| {
                store.save(metadata, cx);
            });
        });

        cx.update(|cx| {
            TerminalThreadMetadataStore::global(cx).update(cx, |store, cx| {
                store.change_worktree_paths(
                    &old_folder_paths,
                    None,
                    |paths| {
                        paths.add_path(new_main_path, new_folder_path);
                        paths.remove_folder_path(Path::new("/repo-feature"));
                    },
                    cx,
                );
            });
        });

        cx.update(|cx| {
            let store = TerminalThreadMetadataStore::global(cx);
            let store = store.read(cx);
            assert!(
                store
                    .entries_for_path(&old_folder_paths, None)
                    .next()
                    .is_none()
            );
            assert_eq!(
                store
                    .entries_for_path(&new_folder_paths, None)
                    .map(|entry| entry.terminal_id)
                    .collect::<Vec<_>>(),
                vec![terminal_id]
            );
            assert_eq!(
                store
                    .entry(terminal_id)
                    .unwrap()
                    .main_worktree_paths()
                    .paths(),
                old_main_paths.paths()
            );
        });
    }
}
