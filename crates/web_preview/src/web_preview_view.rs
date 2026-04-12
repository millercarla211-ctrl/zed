use agent_client_protocol as acp;
use agent_ui::AgentPanel;
use anyhow::{Context as _, Result, anyhow};
use base64::Engine as _;
use editor::Editor;
use gpui::{
    Action, App, AppContext as _, Bounds, ClipboardItem, Context, Corner, Entity, EventEmitter,
    FocusHandle, Focusable, Image as GpuiImage, Pixels, Render, SharedString, Subscription, Task,
    WeakEntity, Window, canvas,
};
#[cfg(target_os = "windows")]
use gpui::{AsyncApp, EntityId, ImageFormat as GpuiImageFormat};
use menu::Confirm;
use paths::data_dir;
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(target_os = "windows")]
use std::{io::Cursor, num::NonZeroIsize};
use ui::{
    Color, ContextMenu, ContextMenuEntry, IconButton, IconName, IconSize, Label, LabelSize,
    PopoverMenu, Tooltip, prelude::*,
};
use workspace::item::{
    Item, ItemEvent, PaneTabBarControls, TabContentParams, WorkspaceScreenKind,
};
use workspace::notifications::NotificationId;
use workspace::{NewWebPreview, Pane, Toast, Workspace, WorkspaceId};

#[cfg(target_os = "windows")]
use crate::windows_visual_webview::WindowsVisualWebView;
use crate::{OpenPreview, OpenPreviewToTheSide};

#[cfg(target_os = "windows")]
use gpui_windows::window_has_focused_webview;
#[cfg(target_os = "windows")]
use image::{ImageFormat as ExternalImageFormat, RgbaImage, imageops, imageops::FilterType};
#[cfg(target_os = "windows")]
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, ClientToScreen,
        CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
        GetDIBits, HBITMAP, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
    },
};
#[cfg(target_os = "macos")]
use wry::{
    PageLoadEvent, Rect as WryRect, WebContext, WebView, WebViewBuilder,
    dpi::{LogicalPosition, LogicalSize, Position, Size},
};

const DEFAULT_WEB_PREVIEW_URL: &str = "https://www.google.com/";
const GOOGLE_SEARCH_URL: &str = "https://www.google.com/search";
const BOOKMARKS_FILE_NAME: &str = "bookmarks.json";

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreviewWorkspaceContext {
    workspace_id: Option<WorkspaceId>,
    root_path: Option<PathBuf>,
    root_name: SharedString,
    preview_key: SharedString,
    profile_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DetectedExtension {
    browser: SharedString,
    id: SharedString,
    name: SharedString,
    path: PathBuf,
    icon_path: Option<PathBuf>,
    supports_chromium_loading: bool,
}

#[derive(Clone, Debug)]
enum PreviewLoadState {
    Ready,
    Error(SharedString),
}

#[derive(Clone, Debug)]
pub(crate) enum BrowserEvent {
    UrlChanged(String),
    TitleChanged(String),
    IpcMessage(String),
    MountFailed(String),
}

#[derive(Clone, Debug)]
enum BrowserAgentPayload {
    InspectElement {
        url: String,
        title: Option<String>,
        selector: Option<String>,
        tag: Option<String>,
        id: Option<String>,
        classes: Vec<String>,
        text: Option<String>,
        href: Option<String>,
        src: Option<String>,
        rect: Option<BrowserRect>,
        css: Option<String>,
        html: Option<String>,
    },
}

#[derive(Clone, Copy, Debug)]
struct BrowserRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(target_os = "windows")]
struct NativeWebPreview {
    webview: WindowsVisualWebView,
}

#[cfg(target_os = "macos")]
struct NativeWebPreview {
    _context: Box<WebContext>,
    webview: WebView,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct RawParentWindow {
    hwnd: NonZeroIsize,
}

#[cfg(target_os = "windows")]
impl RawParentWindow {
    fn from_window(window: &Window) -> Result<Self> {
        let handle = HasWindowHandle::window_handle(window)?;
        let RawWindowHandle::Win32(raw_handle) = handle.as_raw() else {
            return Err(anyhow!("Unsupported window handle for web preview"));
        };
        Ok(Self {
            hwnd: raw_handle.hwnd,
        })
    }

    fn as_hwnd(&self) -> HWND {
        HWND(self.hwnd.get() as *mut _)
    }
}

#[cfg(target_os = "windows")]
impl HasWindowHandle for RawParentWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let handle = Win32WindowHandle::new(self.hwnd);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct NativePreviewMountRequest {
    entity_id: EntityId,
    parent_window: RawParentWindow,
    profile_dir: PathBuf,
    initial_url: String,
    zoom_factor: f64,
    scale_factor: f32,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
}

pub struct WebPreviewView {
    workspace: WeakEntity<Workspace>,
    workspace_context: PreviewWorkspaceContext,
    focus_handle: FocusHandle,
    url_editor: Entity<Editor>,
    page_title: Option<SharedString>,
    active_url: SharedString,
    bookmarks: Vec<String>,
    detected_extensions: Vec<DetectedExtension>,
    extensions_scanned: bool,
    load_state: PreviewLoadState,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    #[cfg(target_os = "macos")]
    last_applied_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    native_mount_requested: Rc<Cell<bool>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    deferred_ipc_messages: Vec<String>,
    ipc_flush_scheduled: bool,
    event_pump_task: Option<Task<()>>,
    zoom_factor: f64,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
    _subscriptions: Vec<Subscription>,
}

impl WebPreviewView {
    pub fn register(workspace: &mut Workspace, _window: &mut Window, _cx: &mut Context<Workspace>) {
        workspace.register_action(move |workspace, _: &NewWebPreview, window, cx| {
            Self::open_new_in_active_pane(workspace, window, cx);
        });

        workspace.register_action(move |workspace, _: &OpenPreview, window, cx| {
            Self::open_in_active_pane(workspace, window, cx);
        });

        workspace.register_action(move |workspace, _: &OpenPreviewToTheSide, window, cx| {
            Self::open_in_side_pane(workspace, window, cx);
        });
    }

    fn open_in_active_pane(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        workspace.active_pane().update(cx, |pane, cx| {
            if let Some(existing_view_idx) = Self::find_existing_preview_item_idx(pane, &view, cx) {
                pane.activate_item(existing_view_idx, true, true, window, cx);
            } else {
                pane.add_item(Box::new(view.clone()), true, true, None, window, cx);
            }
        });
        cx.notify();
    }

    fn open_new_in_active_pane(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        workspace.active_pane().update(cx, |pane, cx| {
            pane.add_item(Box::new(view.clone()), true, true, None, window, cx);
        });
        cx.notify();
    }

    fn open_in_side_pane(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        let pane = workspace
            .find_pane_in_direction(workspace::SplitDirection::Right, cx)
            .unwrap_or_else(|| {
                workspace.split_pane(
                    workspace.active_pane().clone(),
                    workspace::SplitDirection::Right,
                    window,
                    cx,
                )
            });
        pane.update(cx, |pane, cx| {
            if let Some(existing_view_idx) = Self::find_existing_preview_item_idx(pane, &view, cx) {
                pane.activate_item(existing_view_idx, true, true, window, cx);
            } else {
                pane.add_item(Box::new(view.clone()), false, false, None, window, cx);
            }
        });
        cx.notify();
    }

    fn open_or_create(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let workspace_context = Self::workspace_context(workspace, cx);
        let weak_workspace = workspace.weak_handle();

        cx.new(|cx| {
            let current_url = DEFAULT_WEB_PREVIEW_URL.to_string();
            let url_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search Google or enter a URL", window, cx);
                editor.set_text(current_url.as_str(), window, cx);
                editor
            });

            let browser_events = Arc::new(Mutex::new(Vec::new()));
            let mut this = Self {
                workspace: weak_workspace.clone(),
                workspace_context: workspace_context.clone(),
                focus_handle: cx.focus_handle(),
                url_editor,
                page_title: None,
                active_url: current_url.into(),
                bookmarks: load_bookmarks(&workspace_context.profile_dir).unwrap_or_default(),
                detected_extensions: Vec::new(),
                extensions_scanned: false,
                load_state: PreviewLoadState::Ready,
                host_bounds: Rc::new(RefCell::new(None)),
                #[cfg(target_os = "macos")]
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                event_pump_task: None,
                zoom_factor: 1.0,
                #[cfg(any(target_os = "windows", target_os = "macos"))]
                native_preview: Rc::new(RefCell::new(None)),
                _subscriptions: vec![],
            };
            this.start_event_pump(window, cx);
            this
        })
    }

    fn start_event_pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = (window, cx);
        self.event_pump_task = None;
    }

    fn find_existing_preview_item_idx(
        pane: &Pane,
        view: &Entity<WebPreviewView>,
        cx: &App,
    ) -> Option<usize> {
        let preview_key = view.read(cx).workspace_context.preview_key.clone();
        pane.items_of_type::<WebPreviewView>()
            .find(|candidate| candidate.read(cx).workspace_context.preview_key == preview_key)
            .and_then(|candidate| pane.index_for_item(&candidate))
    }

    fn workspace_context(workspace: &Workspace, cx: &App) -> PreviewWorkspaceContext {
        let project = workspace.project().read(cx);
        let root = project.visible_worktrees(cx).next();
        let root_path = root
            .as_ref()
            .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf());
        let root_name = root
            .as_ref()
            .map(|worktree| worktree.read(cx).root_name().as_unix_str().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Workspace".to_string());

        let preview_slug = root_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| slugify(&name.to_string_lossy()))
            .filter(|slug| !slug.is_empty())
            .or_else(|| {
                workspace
                    .database_id()
                    .map(|id| format!("workspace-{id:?}"))
            })
            .unwrap_or_else(|| "workspace".to_string());

        PreviewWorkspaceContext {
            workspace_id: workspace.database_id(),
            root_path,
            root_name: root_name.into(),
            preview_key: preview_slug.clone().into(),
            profile_dir: data_dir()
                .join("web_preview")
                .join("profiles")
                .join(preview_slug),
        }
    }

    fn current_url_text(&self, cx: &App) -> String {
        self.url_editor.read(cx).text(cx).trim().to_string()
    }

    fn current_tab_title(&self) -> SharedString {
        self.page_title
            .clone()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| display_title_from_url(&self.active_url).into())
    }

    fn bookmarks_path(&self) -> PathBuf {
        self.workspace_context.profile_dir.join(BOOKMARKS_FILE_NAME)
    }

    fn is_active_url_bookmarked(&self) -> bool {
        self.bookmarks
            .iter()
            .any(|bookmark| bookmark == self.active_url.as_ref())
    }

    fn persist_bookmarks(&self) -> Result<()> {
        fs::create_dir_all(&self.workspace_context.profile_dir).with_context(|| {
            format!(
                "Failed to prepare {}",
                self.workspace_context.profile_dir.display()
            )
        })?;
        let data = serde_json::to_vec_pretty(&self.bookmarks)
            .with_context(|| "Failed to serialize web preview bookmarks")?;
        fs::write(self.bookmarks_path(), data)
            .with_context(|| "Failed to save web preview bookmarks")?;
        Ok(())
    }

    fn confirm_navigation(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to_input(window, cx);
        let preview_focus = self.focus_handle(cx);
        window.focus(&preview_focus, cx);
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    #[cfg(target_os = "windows")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview
                .webview
                .evaluate_script("window.__zedHostInput?.setTarget?.(null);");
            let _ = preview.webview.focus_parent();
        }
    }

    #[cfg(target_os = "macos")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.webview.focus_parent();
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn release_native_preview_focus(&self) {}

    fn activate_url_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus_handle = self.url_editor.focus_handle(cx);
        #[cfg(target_os = "windows")]
        let should_select_all = RawParentWindow::from_window(window)
            .map(|parent_window| window_has_focused_webview(parent_window.as_hwnd()))
            .unwrap_or_else(|_| !focus_handle.is_focused(window));
        #[cfg(not(target_os = "windows"))]
        let should_select_all = !focus_handle.is_focused(window);

        self.release_native_preview_focus();
        let preview_focus_handle = self.focus_handle(cx);
        window.focus(&preview_focus_handle, cx);
        cx.defer_in(window, move |this, window, cx| {
            window.focus(&focus_handle, cx);
            if should_select_all {
                this.url_editor.update(cx, |editor, cx| {
                    editor.select_all(&editor::actions::SelectAll, window, cx);
                });
            }
        });
    }

    fn navigate_to_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(url) = normalized_url(&self.current_url_text(cx)) else {
            self.load_state = PreviewLoadState::Error("Enter a valid URL or search query.".into());
            cx.notify();
            return;
        };

        self.active_url = url.to_string().into();
        self.page_title = None;
        if let Err(error) = self.load_url(url.as_str(), window, cx) {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.load_state = PreviewLoadState::Ready;
        }
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    fn reload_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.reload_webview(window, cx) {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
            cx.notify();
        }
    }

    fn toggle_bookmark_for_active_url(&mut self, cx: &mut Context<Self>) {
        let url = self.active_url.to_string();
        if url.trim().is_empty() {
            return;
        }

        let message =
            if let Some(index) = self.bookmarks.iter().position(|bookmark| bookmark == &url) {
                self.bookmarks.remove(index);
                "Removed bookmark"
            } else {
                self.bookmarks.push(url);
                self.bookmarks.sort();
                self.bookmarks.dedup();
                "Bookmarked page"
            };

        if let Err(error) = self.persist_bookmarks() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast(message, cx);
        }
        cx.notify();
    }

    fn go_back_in_history(&mut self, cx: &mut Context<Self>) {
        let _ = self.evaluate_script("history.back();");
        cx.notify();
    }

    fn go_forward_in_history(&mut self, cx: &mut Context<Self>) {
        let _ = self.evaluate_script("history.forward();");
        cx.notify();
    }

    fn zoom_in_step(&mut self, cx: &mut Context<Self>) {
        self.zoom_factor = (self.zoom_factor + 0.1).min(3.0);
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn zoom_out_step(&mut self, cx: &mut Context<Self>) {
        self.zoom_factor = (self.zoom_factor - 0.1).max(0.25);
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn open_devtools(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let opened = {
                let borrow = self.native_preview.borrow();
                if let Some(preview) = borrow.as_ref() {
                    preview.webview.open_devtools();
                    true
                } else {
                    false
                }
            };
            if opened {
                self.show_toast("Opened DevTools", cx);
            }
        }
    }

    fn inspect_element(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let script = "window.__zedWebPreview && window.__zedWebPreview.inspectNextElement();";
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(script))) {
            Ok(Ok(())) => {
                self.show_toast("Click an element in the page to send it to the agent.", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Element selector is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Element selector crashed before it could start", cx);
            }
        }
    }

    fn clear_cache(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.clear_all_browsing_data() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast("Cleared browser cache", cx);
        }
        cx.notify();
    }

    fn open_extension_location(
        &mut self,
        extension_name: SharedString,
        extension_path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let url = if extension_path.is_dir() {
            url::Url::from_directory_path(&extension_path)
        } else {
            url::Url::from_file_path(&extension_path)
        };

        match url {
            Ok(url) => {
                cx.open_url(url.as_str());
                self.show_toast(format!("Opened {} extension folder", extension_name), cx);
            }
            Err(()) => {
                self.load_state = PreviewLoadState::Error(
                    format!("Failed to open extension path {}", extension_path.display()).into(),
                );
            }
        }
    }

    fn take_screenshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match catch_unwind(AssertUnwindSafe(|| {
            self.capture_screenshot_payload(None, window)
        })) {
            Ok(Ok((_path, image, blocks))) => {
                cx.write_to_clipboard(ClipboardItem::new_image(&image));
                self.append_content_blocks_to_agent_panel(blocks, window, cx);
                self.show_toast(
                    "Captured web preview screenshot to clipboard and AI input",
                    cx,
                );
            }
            Ok(Err(error)) => {
                self.report_action_error("Web preview screenshot failed", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Web preview screenshot crashed", cx);
            }
        }
        cx.notify();
    }

    fn report_action_error(&mut self, prefix: &str, error: anyhow::Error, cx: &mut Context<Self>) {
        let message = format!("{prefix}: {error}");
        self.load_state = PreviewLoadState::Error(message.clone().into());
        self.show_toast(message, cx);
    }

    fn report_action_panic(&mut self, message: &str, cx: &mut Context<Self>) {
        let message = message.to_string();
        self.load_state = PreviewLoadState::Error(message.clone().into());
        self.show_toast(message, cx);
    }

    fn apply_browser_events(
        &mut self,
        events: Vec<BrowserEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut tab_updated = false;

        for event in events {
            match event {
                BrowserEvent::UrlChanged(url) => {
                    let previous_url = self.active_url.to_string();
                    self.active_url = url.clone().into();
                    let editor_focus = self.url_editor.focus_handle(cx);
                    let editor_text = self.current_url_text(cx);
                    let should_sync_editor = !editor_focus.is_focused(window)
                        || editor_text.is_empty()
                        || editor_text == previous_url;

                    if should_sync_editor {
                        self.url_editor.update(cx, |editor, cx| {
                            editor.set_text(url.as_str(), window, cx);
                        });
                    }
                }
                BrowserEvent::TitleChanged(title) => {
                    self.page_title = Some(title.into());
                    tab_updated = true;
                }
                BrowserEvent::IpcMessage(message) => {
                    self.deferred_ipc_messages.push(message);
                }
                BrowserEvent::MountFailed(error) => {
                    self.native_mount_requested.set(false);
                    self.load_state = PreviewLoadState::Error(error.into());
                }
            }
        }

        self.flush_deferred_ipc(window, cx);

        if tab_updated {
            cx.emit(ItemEvent::UpdateTab);
        }
        cx.notify();
    }

    fn flush_deferred_ipc(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.deferred_ipc_messages.is_empty() || self.ipc_flush_scheduled {
            return;
        }

        self.ipc_flush_scheduled = true;
        cx.on_next_frame(window, |this, window, cx| {
            this.ipc_flush_scheduled = false;
            let pending = std::mem::take(&mut this.deferred_ipc_messages);
            for message in pending {
                if let Err(error) = this.handle_ipc_message(&message, window, cx) {
                    this.load_state = PreviewLoadState::Error(error.to_string().into());
                }
            }
            cx.notify();
        });
    }

    fn handle_ipc_message(
        &mut self,
        message: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let payload: Value =
            serde_json::from_str(message).with_context(|| "Invalid Web Preview bridge payload")?;
        let kind = payload
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Browser bridge payload is missing kind"))?;

        match kind {
            "inspect-element" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let raw_rect = payload.get("rect").and_then(parse_browser_rect);
                    let scale = payload
                        .get("scale")
                        .and_then(Value::as_f64)
                        .filter(|value| *value > 0.0)
                        .unwrap_or(1.0);
                    let data = BrowserAgentPayload::InspectElement {
                        url: payload
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        title: payload
                            .get("title")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        selector: payload
                            .get("selector")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        tag: payload
                            .get("tag")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        id: payload
                            .get("id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        classes: payload
                            .get("classes")
                            .and_then(Value::as_array)
                            .map(|classes| {
                                classes
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(ToOwned::to_owned)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        text: payload
                            .get("text")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        href: payload
                            .get("href")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        src: payload
                            .get("src")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        rect: raw_rect,
                        css: payload
                            .get("css")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        html: payload
                            .get("html")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    };
                    let screenshot_png = raw_rect
                        .map(|rect| BrowserRect {
                            x: rect.x * scale,
                            y: rect.y * scale,
                            width: rect.width * scale,
                            height: rect.height * scale,
                        })
                        .map(|crop| self.capture_screenshot_png_bytes(Some(crop), window))
                        .transpose()
                        .ok()
                        .flatten()
                        .and_then(|png_bytes| self.prepare_agent_png_bytes(png_bytes).ok());
                    self.append_content_blocks_to_agent_panel(
                        self.inspect_element_agent_blocks(&data, screenshot_png.as_deref()),
                        window,
                        cx,
                    );
                    Ok(())
                })) {
                    Ok(Ok(())) => {
                        self.show_toast("Sent inspected element to the agent panel", cx);
                    }
                    Ok(Err(error)) => {
                        self.report_action_error("Element selector failed to send data", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Element selector crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "capture-area" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let scale = payload
                        .get("scale")
                        .and_then(Value::as_f64)
                        .filter(|value| *value > 0.0)
                        .unwrap_or(1.0);
                    let rect = payload
                        .get("rect")
                        .and_then(parse_browser_rect)
                        .ok_or_else(|| anyhow!("Capture area payload is missing a rectangle"))?;
                    let crop = BrowserRect {
                        x: rect.x * scale,
                        y: rect.y * scale,
                        width: rect.width * scale,
                        height: rect.height * scale,
                    };
                    let (_path, image, blocks) =
                        self.capture_screenshot_payload(Some(crop), window)?;
                    cx.write_to_clipboard(ClipboardItem::new_image(&image));
                    self.append_content_blocks_to_agent_panel(blocks, window, cx);
                    Ok(())
                })) {
                    Ok(Ok(())) => {
                        self.show_toast(
                            "Captured selected web preview area to clipboard and AI input",
                            cx,
                        );
                    }
                    Ok(Err(error)) => {
                        self.report_action_error("Selected-area screenshot failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Selected-area screenshot crashed while processing",
                            cx,
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn append_content_blocks_to_agent_panel(
        &mut self,
        blocks: Vec<acp::ContentBlock>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if blocks.is_empty() {
            return;
        }

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let workspace_for_retry = workspace.downgrade();
        window.defer(cx, move |window, cx| {
            let Some(workspace) = workspace_for_retry.upgrade() else {
                return;
            };

            let mut needs_retry = false;
            let retry_blocks = blocks.clone();
            workspace.update(cx, |workspace, cx| {
                if workspace.panel::<AgentPanel>(cx).is_none() {
                    workspace.open_panel::<AgentPanel>(window, cx);
                }

                let Some(panel) = workspace
                    .focus_panel::<AgentPanel>(window, cx)
                    .or_else(|| workspace.panel::<AgentPanel>(cx))
                else {
                    needs_retry = true;
                    return;
                };

                panel.update(cx, |panel, cx| {
                    panel.insert_content_blocks(blocks, window, cx);
                });
            });

            if needs_retry {
                let workspace_for_second_retry = workspace.downgrade();
                window.defer(cx, move |window, cx| {
                    let Some(workspace) = workspace_for_second_retry.upgrade() else {
                        return;
                    };

                    workspace.update(cx, |workspace, cx| {
                        let Some(panel) = workspace
                            .focus_panel::<AgentPanel>(window, cx)
                            .or_else(|| workspace.panel::<AgentPanel>(cx))
                        else {
                            return;
                        };

                        panel.update(cx, |panel, cx| {
                            panel.insert_content_blocks(retry_blocks, window, cx);
                        });
                    });
                });
            }
        });
    }

    fn inspect_element_agent_blocks(
        &self,
        payload: &BrowserAgentPayload,
        screenshot_png_bytes: Option<&[u8]>,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        let mut has_context_attachment = false;

        if let Some(png_bytes) = screenshot_png_bytes {
            blocks.push(acp::ContentBlock::Image(acp::ImageContent::new(
                base64::engine::general_purpose::STANDARD.encode(png_bytes),
                "image/png",
            )));
            has_context_attachment = true;
        }

        if let Some(url_block) = self.inspect_element_url_attachment_block(payload) {
            if has_context_attachment {
                blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n")));
            }
            blocks.push(url_block);
            has_context_attachment = true;
        }

        if has_context_attachment {
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(
            format_agent_note(payload),
        )));
        blocks
    }

    fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let message = message.into();
        let workspace = workspace.downgrade();
        cx.defer(move |cx| {
            let Some(workspace) = workspace.upgrade() else {
                return;
            };

            let _ = workspace.update(cx, |workspace, cx| {
                workspace.show_toast(
                    Toast::new(
                        NotificationId::named("web-preview-toast".into()),
                        message.to_string(),
                    )
                    .autohide(),
                    cx,
                );
            });
        });
    }

    fn ensure_extensions_scanned(&mut self, cx: &mut Context<Self>) {
        if self.extensions_scanned {
            return;
        }

        self.detected_extensions = scan_local_extensions().unwrap_or_default();
        self.extensions_scanned = true;
        cx.notify();
    }

    fn render_tab_bar_add_menu(&self) -> impl IntoElement {
        IconButton::new("web-preview-tab-bar-add-trigger", IconName::Plus)
            .icon_size(IconSize::Small)
            .tooltip(Tooltip::text("New Web Preview"))
            .on_click(|_, window, cx| {
                window.dispatch_action(NewWebPreview.boxed_clone(), cx);
            })
    }

    fn render_tab_bar_extensions_menu(&self, entity: Entity<Self>) -> impl IntoElement {
        PopoverMenu::new("web-preview-tab-bar-extensions-menu")
            .trigger_with_tooltip(
                IconButton::new("web-preview-tab-bar-extensions-trigger", IconName::Blocks)
                    .icon_size(IconSize::Small),
                Tooltip::text("Extensions"),
            )
            .anchor(Corner::TopRight)
            .menu(move |window, cx| {
                let detected_extensions = entity.update(cx, |this, cx| {
                    this.ensure_extensions_scanned(cx);
                    this.detected_extensions.clone()
                });

                Some(ContextMenu::build(window, cx, {
                    let entity = entity.clone();
                    move |menu, _, _| {
                        if detected_extensions.is_empty() {
                            menu.item(
                                ContextMenuEntry::new("No local browser extensions detected")
                                    .icon(IconName::Blocks)
                                    .disabled(true),
                            )
                        } else {
                            let mut menu = menu;
                            for extension in detected_extensions.iter().cloned() {
                                let label = format!("{} ({})", extension.name, extension.browser);
                                let entity = entity.clone();

                                menu = menu.item(
                                    ContextMenuEntry::new(label).icon(IconName::Blocks).handler(
                                        move |window, cx| {
                                            let _ = entity.update(cx, |this, cx| {
                                                this.open_extension_location(
                                                    extension.name.clone(),
                                                    extension.path.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        },
                                    ),
                                );
                            }
                            menu
                        }
                    }
                }))
            })
    }

    fn render_tab_bar_more_menu(&self, entity: Entity<Self>) -> impl IntoElement {
        PopoverMenu::new("web-preview-tab-bar-more-menu")
            .trigger_with_tooltip(
                IconButton::new("web-preview-tab-bar-more-trigger", IconName::Ellipsis)
                    .icon_size(IconSize::Small),
                Tooltip::text("More"),
            )
            .anchor(Corner::TopRight)
            .menu(move |window, cx| {
                let (_is_bookmarked, bookmark_icon, bookmark_label) =
                    entity.read_with(cx, |this, _| {
                        let is_bookmarked = this.is_active_url_bookmarked();
                        let bookmark_icon = if is_bookmarked {
                            IconName::StarFilled
                        } else {
                            IconName::Star
                        };
                        let bookmark_label = if is_bookmarked {
                            "Remove Bookmark"
                        } else {
                            "Bookmark Page"
                        };

                        (is_bookmarked, bookmark_icon, bookmark_label)
                    });

                Some(ContextMenu::build(window, cx, {
                    let entity = entity.clone();
                    move |menu, _, _| {
                        menu.item(
                            ContextMenuEntry::new(bookmark_label)
                                .icon(bookmark_icon)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.toggle_bookmark_for_active_url(cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Take Screenshot")
                                .icon(IconName::Screen)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.take_screenshot(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Inspect Element")
                                .icon(IconName::Code)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.inspect_element(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Open DevTools")
                                .icon(IconName::Terminal)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.open_devtools(window, cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Zoom In")
                                .icon(IconName::Plus)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.zoom_in_step(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Zoom Out")
                                .icon(IconName::Dash)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.zoom_out_step(cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Clear Cache")
                                .icon(IconName::Trash)
                                .handler(move |window, cx| {
                                    let _ = entity.update(cx, |this, cx| {
                                        this.clear_cache(window, cx);
                                    });
                                }),
                        )
                    }
                }))
            })
    }

    fn render_tab_bar_start_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(
                IconButton::new("web-preview-tab-bar-back", IconName::ArrowLeft)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Back"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.go_back_in_history(cx);
                    })),
            )
            .child(
                IconButton::new("web-preview-tab-bar-forward", IconName::ArrowRight)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Forward"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.go_forward_in_history(cx);
                    })),
            )
            .child(
                IconButton::new("web-preview-tab-bar-reload", IconName::RotateCw)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Reload"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.reload_page(window, cx);
                    })),
            )
            .into_any_element()
    }

    fn render_tab_bar_end_controls(
        &self,
        entity: Entity<Self>,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(self.render_tab_bar_add_menu())
            .child(self.render_tab_bar_extensions_menu(entity.clone()))
            .child(self.render_tab_bar_more_menu(entity))
            .into_any_element()
    }

    #[cfg(target_os = "windows")]
    fn ensure_native_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.native_preview.borrow().is_some() || self.native_mount_requested.get() {
            return;
        }

        let Ok(parent_window) = RawParentWindow::from_window(window) else {
            self.load_state = PreviewLoadState::Error(
                "The native web preview could not access the editor window handle.".into(),
            );
            return;
        };

        self.native_mount_requested.set(true);
        eprintln!("[web-preview] schedule mount");
        let request = NativePreviewMountRequest {
            entity_id: cx.entity().entity_id(),
            parent_window,
            profile_dir: self.workspace_context.profile_dir.clone(),
            initial_url: self.active_url.to_string(),
            zoom_factor: self.zoom_factor,
            scale_factor: window.scale_factor(),
            host_bounds: self.host_bounds.clone(),
            browser_events: self.browser_events.clone(),
            native_preview: self.native_preview.clone(),
        };

        self.event_pump_task = Some(cx.spawn(move |_this, cx: &mut AsyncApp| {
            mount_native_preview(request.clone());
            cx.update(|app| app.notify(request.entity_id));
            async {}
        }));
    }

    #[cfg(target_os = "macos")]
    fn ensure_native_preview(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if self.native_preview.borrow().is_some() || self.native_mount_requested.get() {
            return;
        }

        self.native_mount_requested.set(true);
        let result = create_native_preview_for_macos_window(
            window,
            self.workspace_context.profile_dir.clone(),
            self.active_url.to_string(),
            self.zoom_factor,
            self.host_bounds.clone(),
            self.browser_events.clone(),
            self.native_preview.clone(),
        );
        self.native_mount_requested.set(false);

        if let Err(error) = result {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn ensure_native_preview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.load_state = PreviewLoadState::Error(
            "Native web preview underlay support is wired for Windows and macOS. Linux still needs GTK/X11 integration or a separate Wayland embedding path.".into(),
        );
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn load_url(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.ensure_native_preview(window, cx);
        let mut borrow = self.native_preview.borrow_mut();
        let preview = borrow
            .as_mut()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.load_url(url)?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn load_url(
        &mut self,
        _url: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn reload_webview(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.ensure_native_preview(window, cx);
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.reload()?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn reload_webview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn evaluate_script(&self, script: &str) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.evaluate_script(script)?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn evaluate_script(&self, _script: &str) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn apply_zoom(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.zoom(self.zoom_factor)?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn apply_zoom(&self) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.clear_all_browsing_data()?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(target_os = "windows")]
    fn capture_and_store_screenshot(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<PathBuf> {
        let bounds = *self
            .host_bounds
            .borrow()
            .as_ref()
            .ok_or_else(|| anyhow!("The web preview is not visible yet"))?;
        let rect = screen_rect_for_bounds(window, bounds)?;
        let mut image = capture_screen_rect(rect)?;
        if let Some(crop) = crop {
            image = crop_image(image, crop)?;
        }
        let screenshots_dir = self.workspace_context.profile_dir.join("screenshots");
        fs::create_dir_all(&screenshots_dir)
            .with_context(|| "Failed to create the screenshots directory")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let path = screenshots_dir.join(format!("web-preview-{timestamp}.png"));
        image
            .save(&path)
            .with_context(|| format!("Failed to save screenshot to {}", path.display()))?;
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_and_store_screenshot(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<PathBuf> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    fn screenshot_agent_blocks(&self, png_bytes: &[u8]) -> Vec<acp::ContentBlock> {
        let mut blocks = vec![acp::ContentBlock::Image(acp::ImageContent::new(
            base64::engine::general_purpose::STANDARD.encode(png_bytes),
            "image/png",
        ))];

        if let Some(url_block) = self.current_url_attachment_block() {
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n")));
            blocks.push(url_block);
        }

        blocks
    }

    fn current_url_attachment_block(&self) -> Option<acp::ContentBlock> {
        self.url_attachment_block(self.active_url.trim())
    }

    fn inspect_element_url_attachment_block(
        &self,
        payload: &BrowserAgentPayload,
    ) -> Option<acp::ContentBlock> {
        match payload {
            BrowserAgentPayload::InspectElement { url, .. } => self.url_attachment_block(url),
        }
    }

    fn url_attachment_block(&self, url: &str) -> Option<acp::ContentBlock> {
        let url = url.trim();
        let parsed = url::Url::parse(url).ok()?;
        match parsed.scheme() {
            "http" | "https" => Some(acp::ContentBlock::ResourceLink(acp::ResourceLink::new(
                url.to_string(),
                url.to_string(),
            ))),
            _ => None,
        }
    }

    #[cfg(target_os = "windows")]
    fn prepare_agent_png_bytes(&self, png_bytes: Vec<u8>) -> Result<Vec<u8>> {
        const MAX_AGENT_IMAGE_BYTES: usize = 2 * 1024 * 1024;
        const MAX_AGENT_IMAGE_EDGE: u32 = 1600;

        let Ok(image) = image::load_from_memory(&png_bytes) else {
            return Ok(png_bytes);
        };

        if png_bytes.len() <= MAX_AGENT_IMAGE_BYTES
            && image.width() <= MAX_AGENT_IMAGE_EDGE
            && image.height() <= MAX_AGENT_IMAGE_EDGE
        {
            return Ok(png_bytes);
        }

        let resized = image.resize(
            MAX_AGENT_IMAGE_EDGE,
            MAX_AGENT_IMAGE_EDGE,
            FilterType::Lanczos3,
        );
        let mut cursor = Cursor::new(Vec::new());
        resized
            .write_to(&mut cursor, ExternalImageFormat::Png)
            .with_context(|| "Failed to encode the AI screenshot attachment")?;
        Ok(cursor.into_inner())
    }

    #[cfg(not(target_os = "windows"))]
    fn prepare_agent_png_bytes(&self, png_bytes: Vec<u8>) -> Result<Vec<u8>> {
        Ok(png_bytes)
    }

    #[cfg(target_os = "windows")]
    fn capture_screenshot_png_bytes(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<Vec<u8>> {
        let path = self.capture_and_store_screenshot(crop, window)?;
        fs::read(&path)
            .with_context(|| format!("Failed to read screenshot bytes from {}", path.display()))
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_screenshot_png_bytes(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<Vec<u8>> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    #[cfg(target_os = "windows")]
    fn capture_screenshot_payload(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<(PathBuf, GpuiImage, Vec<acp::ContentBlock>)> {
        let path = self.capture_and_store_screenshot(crop, window)?;
        let png_bytes = fs::read(&path)
            .with_context(|| format!("Failed to read screenshot from {}", path.display()))?;
        let agent_png_bytes = self.prepare_agent_png_bytes(png_bytes.clone())?;
        let image = GpuiImage::from_bytes(GpuiImageFormat::Png, png_bytes);
        let blocks = self.screenshot_agent_blocks(&agent_png_bytes);
        Ok((path, image, blocks))
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_screenshot_payload(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<(PathBuf, GpuiImage, Vec<acp::ContentBlock>)> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    fn render_webview_body(&self, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let host_bounds = self.host_bounds.clone();
            #[cfg(target_os = "macos")]
            let last_applied_bounds = self.last_applied_bounds.clone();
            let native_preview = self.native_preview.clone();

            let canvas = canvas(
                move |bounds, window, _cx| {
                    *host_bounds.borrow_mut() = Some(bounds);
                    #[cfg(target_os = "windows")]
                    let preview_ready = native_preview.borrow().is_some();
                    if let Some(preview) = native_preview.borrow_mut().as_mut() {
                        // Always keep webview visible
                        let _ = preview.webview.set_visible(true);

                        #[cfg(target_os = "windows")]
                        {
                            let _ = preview.webview.set_bounds(
                                client_rect_for_bounds(bounds, window.scale_factor()),
                                window.scale_factor(),
                            );
                        }

                        #[cfg(target_os = "macos")]
                        {
                            let should_update_bounds =
                                last_applied_bounds.borrow().as_ref().copied() != Some(bounds);
                            if should_update_bounds {
                                let _ = set_webview_bounds(&preview.webview, bounds);
                                *last_applied_bounds.borrow_mut() = Some(bounds);
                            }
                        }
                    }
                    #[cfg(target_os = "windows")]
                    if preview_ready {
                        let passthrough_hitbox =
                            window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
                        window.insert_mouse_passthrough_region(&passthrough_hitbox);
                    }
                    bounds
                },
                |_bounds, _state, _window, _cx| {},
            )
            .size_full();

            #[cfg(target_os = "windows")]
            {
                let _ = cx;
                return canvas.into_any_element();
            }

            #[cfg(not(target_os = "windows"))]
            {
                let _ = cx;
                return canvas.into_any_element();
            }
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    Label::new("Web Preview native embedding is wired for Windows and macOS. Linux still needs compositor-specific host integration.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element();
        }
    }
}

impl EventEmitter<ItemEvent> for WebPreviewView {}

impl Focusable for WebPreviewView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for WebPreviewView {
    type Event = ItemEvent;

    fn tab_content(&self, params: TabContentParams, window: &Window, cx: &App) -> AnyElement {
        let editor_focused = params.selected && self.url_editor.focus_handle(cx).is_focused(window);

        if editor_focused {
            return div()
                .w(px(240.))
                .min_w_0()
                .child(self.url_editor.clone())
                .into_any_element();
        }

        Label::new(self.current_tab_title())
            .color(params.text_color())
            .into_any_element()
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        self.current_tab_title()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<ui::Icon> {
        Some(ui::Icon::new(IconName::Public))
    }

    fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
        (!self.active_url.trim().is_empty()).then(|| self.active_url.clone())
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Web Preview Opened")
    }

    fn screen_kind(&self) -> WorkspaceScreenKind {
        WorkspaceScreenKind::Browser
    }

    fn requires_transparent_workspace_background() -> bool {
        true
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn pane_tab_bar_controls(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PaneTabBarControls> {
        let entity = cx.entity();

        Some(PaneTabBarControls::new(
            Some(self.render_tab_bar_start_controls(cx)),
            Some(self.render_tab_bar_end_controls(entity, cx)),
        ))
    }

    fn on_tab_click(
        &mut self,
        params: TabContentParams,
        event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !params.selected || event.click_count() != 1 {
            return false;
        }

        let editor_focus = self.url_editor.focus_handle(cx);
        if !editor_focus.is_focused(window) {
            self.activate_url_editor(window, cx);
        }
        true
    }

    fn on_tab_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.url_editor.focus_handle(cx).is_focused(window) {
            return false;
        }

        self.confirm_navigation(&Confirm, window, cx);
        true
    }

    fn can_split(&self) -> bool {
        true
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        let workspace = self.workspace.clone();
        let workspace_context = self.workspace_context.clone();
        let current_url = self.current_url_text(cx);
        let detected_extensions = self.detected_extensions.clone();
        let bookmarks = self.bookmarks.clone();

        Task::ready(Some(cx.new(|cx| {
            let url_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search Google or enter a URL", window, cx);
                editor.set_text(current_url.as_str(), window, cx);
                editor
            });

            let browser_events = Arc::new(Mutex::new(Vec::new()));
            let mut this = Self {
                workspace,
                workspace_context,
                focus_handle: cx.focus_handle(),
                url_editor,
                page_title: None,
                active_url: current_url.clone().into(),
                bookmarks,
                detected_extensions,
                extensions_scanned: self.extensions_scanned,
                load_state: PreviewLoadState::Ready,
                host_bounds: Rc::new(RefCell::new(None)),
                #[cfg(target_os = "macos")]
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                event_pump_task: None,
                zoom_factor: 1.0,
                #[cfg(any(target_os = "windows", target_os = "macos"))]
                native_preview: Rc::new(RefCell::new(None)),
                _subscriptions: vec![],
            };
            this.start_event_pump(window, cx);
            this
        })))
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }

    fn added_to_workspace(
        &mut self,
        _workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._subscriptions.push(cx.on_focus_out(
            &self.focus_handle(cx),
            window,
            |this, _, _window, _cx| {
                this.release_native_preview_focus();
            },
        ));

        let url_editor_focus = self.url_editor.focus_handle(cx);
        self._subscriptions
            .push(cx.on_focus(&url_editor_focus, window, |_, _, cx| {
                cx.emit(ItemEvent::UpdateTab);
                cx.notify();
            }));
        self._subscriptions
            .push(cx.on_focus_out(&url_editor_focus, window, |_, _, _, cx| {
                cx.emit(ItemEvent::UpdateTab);
                cx.notify();
            }));

        let focus_handle = self.focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            focus_handle.focus(window, cx);
        });
    }

    fn deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // Hide webview when tab is deactivated
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        if let Some(preview) = self.native_preview.borrow_mut().as_mut() {
            let _ = preview.webview.set_visible(false);
        }
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        _window.set_background_appearance(gpui::WindowBackgroundAppearance::Opaque);
    }

    fn workspace_deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
    }
}

impl Render for WebPreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending_events = {
            let mut events = self
                .browser_events
                .lock()
                .expect("browser event queue lock poisoned");
            std::mem::take(&mut *events)
        };
        if !pending_events.is_empty() {
            self.apply_browser_events(pending_events, window, cx);
        }

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            window.set_background_appearance(gpui::WindowBackgroundAppearance::Transparent);
            self.ensure_native_preview(window, cx);
        }

        let body = self.render_webview_body(cx);
        let error_message = match &self.load_state {
            PreviewLoadState::Ready => None,
            PreviewLoadState::Error(error) => Some(error.clone()),
        };
        #[cfg(target_os = "windows")]
        let preview_surface_background = gpui::transparent_black().alpha(1.0 / 255.0);
        #[cfg(not(target_os = "windows"))]
        let preview_surface_background = gpui::transparent_black();

        div()
            .id("web-preview")
            .key_context("WebPreview")
            .on_action(cx.listener(Self::confirm_navigation))
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .overflow_hidden()
            .child(
                v_flex().size_full().child(
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .overflow_hidden()
                        // Keep the Windows top GPUI window hit-testable over the underlay webview
                        // without visibly obscuring the page.
                        .bg(preview_surface_background)
                        .child(body)
                        .when_some(error_message, |this, error| {
                            this.child(
                                div()
                                    .absolute()
                                    .top_3()
                                    .left_3()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(Color::Error.color(cx).alpha(0.14))
                                    .child(
                                        Label::new(error)
                                            .size(LabelSize::Small)
                                            .color(Color::Error),
                                    ),
                            )
                        }),
                ),
            )
    }
}

pub(crate) fn push_browser_event(event_queue: &Arc<Mutex<Vec<BrowserEvent>>>, event: BrowserEvent) {
    let mut queue = event_queue
        .lock()
        .expect("browser event queue lock poisoned");
    queue.push(event);
}

#[cfg(target_os = "windows")]
fn mount_native_preview(request: NativePreviewMountRequest) {
    eprintln!("[web-preview] deferred mount start");
    let result = catch_unwind(AssertUnwindSafe(|| {
        create_native_preview_for_request(&request)
    }));

    match result {
        Ok(Ok(())) => eprintln!("[web-preview] deferred mount ok"),
        Ok(Err(error)) => push_browser_event(
            &request.browser_events,
            BrowserEvent::MountFailed(error.to_string()),
        ),
        Err(_) => push_browser_event(
            &request.browser_events,
            BrowserEvent::MountFailed(
                "The native web preview crashed while mounting the embedded browser.".to_string(),
            ),
        ),
    }
}

#[cfg(target_os = "windows")]
fn create_native_preview_for_request(request: &NativePreviewMountRequest) -> Result<()> {
    if request.native_preview.borrow().is_some() {
        return Ok(());
    }

    eprintln!("[web-preview] create request prepare");
    fs::create_dir_all(&request.profile_dir)
        .with_context(|| "Failed to prepare the Web Preview profile directory")?;

    let url = normalized_url(&request.initial_url)?;
    let main_window = request.parent_window.as_hwnd();
    let initial_bounds = request
        .host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(|bounds| client_rect_for_bounds(bounds, request.scale_factor))
        .unwrap_or(RECT {
            left: 0,
            top: 0,
            right: 32,
            bottom: 32,
        });

    eprintln!("[web-preview] create request build");
    let webview = WindowsVisualWebView::new(
        main_window,
        request.profile_dir.clone(),
        url.as_str(),
        request.zoom_factor,
        request.scale_factor,
        initial_bounds,
        request.browser_events.clone(),
    )
    .with_context(|| "Failed to build the embedded web preview")?;

    *request.native_preview.borrow_mut() = Some(NativeWebPreview { webview });
    eprintln!("[web-preview] create request done");

    Ok(())
}

#[cfg(target_os = "macos")]
fn create_native_preview_for_macos_window(
    window: &Window,
    profile_dir: PathBuf,
    initial_url: String,
    zoom_factor: f64,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
) -> Result<()> {
    if native_preview.borrow().is_some() {
        return Ok(());
    }

    fs::create_dir_all(&profile_dir)
        .with_context(|| "Failed to prepare the Web Preview profile directory")?;

    let url = normalized_url(&initial_url)?;
    let event_queue = browser_events.clone();
    let mut web_context = Box::new(WebContext::new(Some(profile_dir)));

    let initial_bounds = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(bounds_to_wry_rect)
        .unwrap_or_else(|| WryRect {
            position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
            size: Size::Logical(LogicalSize::new(32.0, 32.0)),
        });

    let webview = WebViewBuilder::new_with_web_context(web_context.as_mut())
        .with_bounds(initial_bounds)
        .with_url(url.as_str())
        .with_focused(false)
        .with_clipboard(true)
        .with_hotkeys_zoom(false)
        .with_back_forward_navigation_gestures(true)
        .with_default_context_menus(false)
        .with_devtools(true)
        .with_visible(true)
        .with_initialization_script(WEB_PREVIEW_BRIDGE_SCRIPT)
        .with_document_title_changed_handler({
            let event_queue = event_queue.clone();
            move |title| push_browser_event(&event_queue, BrowserEvent::TitleChanged(title))
        })
        .with_on_page_load_handler({
            let event_queue = event_queue.clone();
            move |event, url| {
                if matches!(event, PageLoadEvent::Finished) {
                    push_browser_event(&event_queue, BrowserEvent::UrlChanged(url));
                }
            }
        })
        .with_ipc_handler(move |request| {
            push_browser_event(
                &event_queue,
                BrowserEvent::IpcMessage(request.body().to_string()),
            );
        })
        .build_as_child(window)
        .with_context(|| "Failed to build the embedded web preview")?;

    webview.zoom(zoom_factor)?;

    if let Some(bounds) = *host_bounds.borrow() {
        let _ = set_webview_bounds(&webview, bounds);
    }

    *native_preview.borrow_mut() = Some(NativeWebPreview {
        _context: web_context,
        webview,
    });

    Ok(())
}

fn parse_browser_rect(value: &Value) -> Option<BrowserRect> {
    Some(BrowserRect {
        x: value.get("x")?.as_f64()?,
        y: value.get("y")?.as_f64()?,
        width: value.get("width")?.as_f64()?,
        height: value.get("height")?.as_f64()?,
    })
}

fn format_agent_note(payload: &BrowserAgentPayload) -> String {
    match payload {
        BrowserAgentPayload::InspectElement {
            url,
            title,
            selector,
            tag,
            id,
            classes,
            text,
            href,
            src,
            rect,
            css,
            html,
        } => {
            let mut message = format!("Web Preview inspected element.\nURL: {url}");
            if let Some(title) = title.as_deref().filter(|title| !title.is_empty()) {
                message.push_str(&format!("\nPage title: {title}"));
            }
            if let Some(selector) = selector.as_deref().filter(|selector| !selector.is_empty()) {
                message.push_str(&format!("\nSelector: {selector}"));
            }
            if let Some(tag) = tag.as_deref().filter(|tag| !tag.is_empty()) {
                message.push_str(&format!("\nTag: {tag}"));
            }
            if let Some(id) = id.as_deref().filter(|id| !id.is_empty()) {
                message.push_str(&format!("\nID: {id}"));
            }
            if !classes.is_empty() {
                message.push_str(&format!("\nClasses: {}", classes.join(" ")));
            }
            if let Some(text) = text.as_deref().filter(|text| !text.is_empty()) {
                message.push_str(&format!("\nText: {text}"));
            }
            if let Some(href) = href.as_deref().filter(|href| !href.is_empty()) {
                message.push_str(&format!("\nHref: {href}"));
            }
            if let Some(src) = src.as_deref().filter(|src| !src.is_empty()) {
                message.push_str(&format!("\nSource: {src}"));
            }
            if let Some(rect) = rect.as_ref() {
                message.push_str(&format!(
                    "\nRect: x={:.1}, y={:.1}, width={:.1}, height={:.1}",
                    rect.x, rect.y, rect.width, rect.height
                ));
            }
            if let Some(css) = css.as_deref().filter(|css| !css.is_empty()) {
                message.push_str("\nCSS snapshot:\n```css\n");
                message.push_str(css);
                message.push_str("\n```");
            }
            if let Some(html) = html.as_deref().filter(|html| !html.is_empty()) {
                message.push_str("\nHTML snippet:\n```html\n");
                message.push_str(html);
                message.push_str("\n```");
            }
            message
        }
    }
}

fn display_title_from_url(url: &str) -> String {
    if url.eq_ignore_ascii_case("about:blank") {
        return "Preview".to_string();
    }

    url::Url::parse(url)
        .ok()
        .and_then(|url| {
            url.host_str()
                .map(|host| host.to_string())
                .or_else(|| Some(url.path().to_string()))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Preview".to_string())
}

fn normalized_url(raw: &str) -> Result<url::Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return url::Url::parse(DEFAULT_WEB_PREVIEW_URL).map_err(Into::into);
    }

    if let Ok(url) = url::Url::parse(trimmed) {
        return Ok(url);
    }

    if trimmed.contains('.') && !trimmed.contains(' ') {
        return url::Url::parse(&format!("https://{trimmed}"))
            .or_else(|_| url::Url::parse(&format!("http://{trimmed}")))
            .map_err(Into::into);
    }

    let mut url = url::Url::parse(GOOGLE_SEARCH_URL)?;
    url.query_pairs_mut().append_pair("q", trimmed);
    Ok(url)
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn load_bookmarks(profile_dir: &Path) -> Result<Vec<String>> {
    let path = profile_dir.join(BOOKMARKS_FILE_NAME);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("Failed to parse {}", path.display()))
}

fn scan_local_extensions() -> Result<Vec<DetectedExtension>> {
    let mut extensions = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let local_app_data = PathBuf::from(local_app_data);
            let sources = [
                (
                    "Chrome",
                    local_app_data
                        .join("Google")
                        .join("Chrome")
                        .join("User Data")
                        .join("Default")
                        .join("Extensions"),
                    true,
                ),
                (
                    "Edge",
                    local_app_data
                        .join("Microsoft")
                        .join("Edge")
                        .join("User Data")
                        .join("Default")
                        .join("Extensions"),
                    true,
                ),
                (
                    "Brave",
                    local_app_data
                        .join("BraveSoftware")
                        .join("Brave-Browser")
                        .join("User Data")
                        .join("Default")
                        .join("Extensions"),
                    true,
                ),
                (
                    "Firefox",
                    local_app_data
                        .join("Mozilla")
                        .join("Firefox")
                        .join("Profiles"),
                    false,
                ),
            ];

            for (browser, path, chromium_compatible) in sources {
                if browser == "Firefox" {
                    scan_firefox_extensions(browser, &path, &mut extensions)?;
                    continue;
                }

                scan_chromium_extensions(browser, &path, chromium_compatible, &mut extensions)?;
            }
        }
    }

    extensions.sort_by(|left, right| {
        left.browser
            .cmp(&right.browser)
            .then_with(|| left.name.cmp(&right.name))
    });
    extensions.dedup_by(|left, right| left.browser == right.browser && left.path == right.path);
    Ok(extensions)
}

#[cfg(target_os = "windows")]
fn scan_chromium_extensions(
    browser: &str,
    root: &Path,
    chromium_compatible: bool,
    extensions: &mut Vec<DetectedExtension>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    for extension_dir in fs::read_dir(root)? {
        let extension_dir = extension_dir?;
        let extension_path = extension_dir.path();
        if !extension_path.is_dir() {
            continue;
        }

        let Some(extension_id) = extension_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let mut versions = fs::read_dir(&extension_path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        versions.sort();

        let Some(version_dir) = versions.pop() else {
            continue;
        };
        let manifest_path = version_dir.join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }

        let manifest: Value = serde_json::from_slice(
            &fs::read(&manifest_path)
                .with_context(|| format!("Failed to read {}", manifest_path.display()))?,
        )
        .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;

        let name = manifest
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .unwrap_or(extension_id);

        let icon_path = manifest
            .get("icons")
            .and_then(Value::as_object)
            .and_then(|icons| {
                icons
                    .iter()
                    .filter_map(|(size, path)| Some((size.parse::<u32>().ok()?, path.as_str()?)))
                    .max_by_key(|(size, _)| *size)
                    .map(|(_, path)| version_dir.join(path))
            })
            .filter(|path| path.is_file());

        extensions.push(DetectedExtension {
            browser: browser.to_string().into(),
            id: extension_id.to_string().into(),
            name: name.to_string().into(),
            path: version_dir,
            icon_path,
            supports_chromium_loading: chromium_compatible,
        });
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn scan_firefox_extensions(
    browser: &str,
    root: &Path,
    extensions: &mut Vec<DetectedExtension>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    for profile_dir in fs::read_dir(root)? {
        let profile_dir = profile_dir?;
        let extensions_dir = profile_dir.path().join("extensions");
        if !extensions_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&extensions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() && !path.is_dir() {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Firefox Extension");
            extensions.push(DetectedExtension {
                browser: browser.to_string().into(),
                id: slugify(name).into(),
                name: name.to_string().into(),
                path,
                icon_path: None,
                supports_chromium_loading: false,
            });
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn set_webview_bounds(webview: &WebView, bounds: Bounds<Pixels>) -> Result<()> {
    let rect = bounds_to_wry_rect(bounds);
    webview.set_bounds(rect)?;
    // Webview stays visible - no hiding
    Ok(())
}

#[cfg(target_os = "macos")]
fn bounds_to_wry_rect(bounds: Bounds<Pixels>) -> WryRect {
    WryRect {
        position: Position::Logical(LogicalPosition::new(
            f64::from(bounds.origin.x),
            f64::from(bounds.origin.y),
        )),
        size: Size::Logical(LogicalSize::new(
            f64::from(bounds.size.width.max(Pixels::from(1.0))),
            f64::from(bounds.size.height.max(Pixels::from(1.0))),
        )),
    }
}

#[cfg(target_os = "windows")]
fn client_rect_for_bounds(bounds: Bounds<Pixels>, scale_factor: f32) -> RECT {
    RECT {
        left: (bounds.origin.x.as_f32() * scale_factor).floor() as i32,
        top: (bounds.origin.y.as_f32() * scale_factor).floor() as i32,
        right: ((bounds.origin.x.as_f32() + bounds.size.width.as_f32()) * scale_factor).ceil()
            as i32,
        bottom: ((bounds.origin.y.as_f32() + bounds.size.height.as_f32()) * scale_factor).ceil()
            as i32,
    }
}

#[cfg(target_os = "windows")]
fn screen_rect_for_hwnd(hwnd: HWND, scale: f32, bounds: Bounds<Pixels>) -> Result<RECT> {
    let mut client_origin = POINT { x: 0, y: 0 };
    unsafe {
        if !ClientToScreen(hwnd, &mut client_origin).as_bool() {
            return Err(anyhow!(
                "Failed to translate the web preview bounds to screen coordinates"
            ));
        }
    }

    let left = client_origin.x + (f32::from(bounds.origin.x) * scale).round() as i32;
    let top = client_origin.y + (f32::from(bounds.origin.y) * scale).round() as i32;
    let width = (f32::from(bounds.size.width) * scale).round().max(1.0) as i32;
    let height = (f32::from(bounds.size.height) * scale).round().max(1.0) as i32;

    Ok(RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
}

#[cfg(target_os = "windows")]
fn screen_rect_for_bounds(window: &Window, bounds: Bounds<Pixels>) -> Result<RECT> {
    let handle = raw_window_handle::HasWindowHandle::window_handle(window)?;
    let RawWindowHandle::Win32(raw_handle) = handle.as_raw() else {
        return Err(anyhow!(
            "Unsupported window handle for web preview screenshots"
        ));
    };
    screen_rect_for_hwnd(
        HWND(raw_handle.hwnd.get() as *mut _),
        window.scale_factor(),
        bounds,
    )
}

#[cfg(target_os = "windows")]
fn capture_screen_rect(rect: RECT) -> Result<RgbaImage> {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err(anyhow!("Screenshot bounds are empty"));
    }

    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return Err(anyhow!("Failed to acquire the screen device context"));
        }

        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        if memory_dc.0.is_null() {
            ReleaseDC(None, screen_dc);
            return Err(anyhow!("Failed to create a compatible device context"));
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.0.is_null() {
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err(anyhow!("Failed to create a compatible bitmap"));
        }

        let previous = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
        let blit_result = BitBlt(
            memory_dc,
            0,
            0,
            width,
            height,
            Some(screen_dc),
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        );
        let _ = SelectObject(memory_dc, previous);

        if blit_result.is_err() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err(anyhow!(
                "Failed to copy the web preview surface into a bitmap"
            ));
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bytes = vec![0u8; (width as usize) * (height as usize) * 4];
        let rows = GetDIBits(
            memory_dc,
            HBITMAP(bitmap.0),
            0,
            height as u32,
            Some(bytes.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(None, screen_dc);

        if rows == 0 {
            return Err(anyhow!("Failed to read the captured bitmap data"));
        }

        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        RgbaImage::from_raw(width as u32, height as u32, bytes)
            .ok_or_else(|| anyhow!("Failed to construct the screenshot image buffer"))
    }
}

#[cfg(target_os = "windows")]
fn crop_image(image: RgbaImage, rect: BrowserRect) -> Result<RgbaImage> {
    let image_width = image.width();
    let image_height = image.height();
    let x = rect.x.max(0.0).round() as u32;
    let y = rect.y.max(0.0).round() as u32;
    let width = rect.width.max(1.0).round() as u32;
    let height = rect.height.max(1.0).round() as u32;

    if x >= image_width || y >= image_height {
        return Err(anyhow!(
            "The selected capture area is outside the visible page"
        ));
    }

    let width = width.min(image_width.saturating_sub(x));
    let height = height.min(image_height.saturating_sub(y));
    Ok(imageops::crop_imm(&image, x, y, width, height).to_image())
}

pub(crate) const WEB_PREVIEW_BRIDGE_SCRIPT: &str = r#"
(() => {
  if (window.__zedWebPreview) return;

  const post = (payload) => {
    try {
      window.ipc.postMessage(JSON.stringify(payload));
    } catch (_error) {}
  };

  const limitText = (value, max) => {
    if (!value) return null;
    const text = String(value).trim();
    return text ? text.slice(0, max) : null;
  };

  const cssSelector = (element) => {
    if (!(element instanceof Element)) return null;
    if (element.id) return `#${element.id}`;
    const parts = [];
    let node = element;
    while (node && node.nodeType === Node.ELEMENT_NODE && parts.length < 6) {
      let selector = node.tagName.toLowerCase();
      if (node.classList.length) {
        selector += Array.from(node.classList).slice(0, 3).map(cls => `.${cls}`).join("");
      }
      const parent = node.parentElement;
      if (parent) {
        const siblings = Array.from(parent.children).filter(child => child.tagName === node.tagName);
        if (siblings.length > 1) {
          selector += `:nth-of-type(${siblings.indexOf(node) + 1})`;
        }
      }
      parts.unshift(selector);
      node = parent;
    }
    return parts.join(" > ");
  };

  const styleSnapshot = (element) => {
    if (!(element instanceof Element)) return null;
    const computed = window.getComputedStyle(element);
    const properties = [
      "display",
      "position",
      "z-index",
      "width",
      "height",
      "min-width",
      "min-height",
      "max-width",
      "max-height",
      "margin",
      "padding",
      "gap",
      "align-items",
      "justify-content",
      "flex",
      "flex-direction",
      "grid-template-columns",
      "grid-template-rows",
      "color",
      "background-color",
      "font-family",
      "font-size",
      "font-weight",
      "line-height",
      "text-align",
      "border",
      "border-radius",
      "box-shadow",
      "opacity",
      "overflow",
      "transform"
    ];

    const lines = properties
      .map((property) => {
        const value = computed.getPropertyValue(property)?.trim();
        return value ? `${property}: ${value};` : null;
      })
      .filter(Boolean);

    return lines.length ? lines.join("\n") : null;
  };

  const createOverlay = () => {
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "2147483647";
    overlay.style.cursor = "crosshair";
    overlay.style.pointerEvents = "none";
    overlay.style.background = "rgba(0, 0, 0, 0.02)";
    return overlay;
  };

  window.__zedWebPreview = {
    inspectNextElement() {
      if (window.__zedWebPreview.__cleanup) {
        window.__zedWebPreview.__cleanup();
      }

      const overlay = createOverlay();
      const highlight = document.createElement("div");
      highlight.style.position = "fixed";
      highlight.style.border = "2px solid #4ea1ff";
      highlight.style.background = "rgba(78, 161, 255, 0.14)";
      highlight.style.pointerEvents = "none";
      highlight.style.borderRadius = "8px";
      overlay.appendChild(highlight);
      document.documentElement.appendChild(overlay);

      const move = (event) => {
        const element = event.target instanceof Element ? event.target : null;
        if (!element) return;
        const rect = element.getBoundingClientRect();
        highlight.style.left = `${rect.left}px`;
        highlight.style.top = `${rect.top}px`;
        highlight.style.width = `${rect.width}px`;
        highlight.style.height = `${rect.height}px`;
      };

      const cleanup = () => {
        document.removeEventListener("mousemove", move, true);
        document.removeEventListener("click", click, true);
        document.removeEventListener("keydown", keydown, true);
        overlay.remove();
        window.__zedWebPreview.__cleanup = null;
      };

      const click = (event) => {
        event.preventDefault();
        event.stopPropagation();
        const element = event.target instanceof Element ? event.target : null;
        if (!element) {
          cleanup();
          return;
        }

        const rect = element.getBoundingClientRect();
        cleanup();
        post({
          kind: "inspect-element",
          url: window.location.href,
          title: document.title,
          scale: window.devicePixelRatio || 1,
          selector: cssSelector(element),
          tag: element.tagName.toLowerCase(),
          id: element.id || null,
          classes: Array.from(element.classList || []),
          text: limitText(element.innerText || element.textContent, 2000),
          href: element.getAttribute("href"),
          src: element.getAttribute("src"),
          css: styleSnapshot(element),
          html: limitText(element.outerHTML, 4000),
          rect: {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height
          }
        });
      };

      const keydown = (event) => {
        if (event.key === "Escape") cleanup();
      };

      document.addEventListener("mousemove", move, true);
      document.addEventListener("click", click, true);
      document.addEventListener("keydown", keydown, true);
      window.__zedWebPreview.__cleanup = cleanup;
    },

    captureAreaScreenshot() {
      if (window.__zedWebPreview.__cleanup) {
        window.__zedWebPreview.__cleanup();
      }

      const overlay = createOverlay();
      overlay.style.pointerEvents = "auto";
      const selection = document.createElement("div");
      selection.style.position = "fixed";
      selection.style.border = "2px solid #4ea1ff";
      selection.style.background = "rgba(78, 161, 255, 0.18)";
      selection.style.pointerEvents = "none";
      selection.style.borderRadius = "10px";
      overlay.appendChild(selection);
      document.documentElement.appendChild(overlay);

      let start = null;

      const updateSelection = (x, y) => {
        if (!start) return;
        const left = Math.min(start.x, x);
        const top = Math.min(start.y, y);
        const width = Math.abs(start.x - x);
        const height = Math.abs(start.y - y);
        selection.style.left = `${left}px`;
        selection.style.top = `${top}px`;
        selection.style.width = `${width}px`;
        selection.style.height = `${height}px`;
      };

      const cleanup = () => {
        document.removeEventListener("mousedown", mouseDown, true);
        document.removeEventListener("mousemove", mouseMove, true);
        document.removeEventListener("mouseup", mouseUp, true);
        document.removeEventListener("keydown", keydown, true);
        overlay.remove();
        window.__zedWebPreview.__cleanup = null;
      };

      const mouseDown = (event) => {
        event.preventDefault();
        event.stopPropagation();
        start = { x: event.clientX, y: event.clientY };
        updateSelection(event.clientX, event.clientY);
      };

      const mouseMove = (event) => {
        if (!start) return;
        event.preventDefault();
        updateSelection(event.clientX, event.clientY);
      };

      const mouseUp = (event) => {
        if (!start) return;
        event.preventDefault();
        const left = Math.min(start.x, event.clientX);
        const top = Math.min(start.y, event.clientY);
        const width = Math.max(1, Math.abs(start.x - event.clientX));
        const height = Math.max(1, Math.abs(start.y - event.clientY));
        cleanup();
        post({
          kind: "capture-area",
          url: window.location.href,
          title: document.title,
          scale: window.devicePixelRatio || 1,
          rect: { x: left, y: top, width, height }
        });
      };

      const keydown = (event) => {
        if (event.key === "Escape") cleanup();
      };

      document.addEventListener("mousedown", mouseDown, true);
      document.addEventListener("mousemove", mouseMove, true);
      document.addEventListener("mouseup", mouseUp, true);
      document.addEventListener("keydown", keydown, true);
      window.__zedWebPreview.__cleanup = cleanup;
    }
  };
})();
"#;

#[cfg(test)]
mod tests {
    use super::{display_title_from_url, normalized_url, slugify};

    #[test]
    fn slugify_strips_extra_separators() {
        assert_eq!(slugify("Friday Web App"), "friday-web-app");
        assert_eq!(slugify("___Preview___"), "preview");
    }

    #[test]
    fn normalize_search_queries_to_google() {
        let url = normalized_url("react devtools extension").unwrap();
        assert_eq!(url.host_str(), Some("www.google.com"));
        assert_eq!(url.path(), "/search");
    }

    #[test]
    fn normalize_missing_scheme_urls() {
        let url = normalized_url("localhost:5173").unwrap();
        assert_eq!(url.host_str(), Some("localhost"));
    }

    #[test]
    fn title_uses_host_name() {
        assert_eq!(
            display_title_from_url("https://www.google.com/"),
            "www.google.com"
        );
    }

    #[test]
    fn title_uses_preview_for_blank_page() {
        assert_eq!(display_title_from_url("about:blank"), "Preview");
    }
}
