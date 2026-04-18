use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};

use anyhow::{Context as _, Result, anyhow};
use gpui_windows::{
    clear_webview_passthrough_target, create_webview_composition_visual,
    register_webview_passthrough_target, remove_webview_composition_visual,
    set_webview_composition_visual_offset, update_webview_passthrough_cursor,
    update_webview_passthrough_focus,
};
use webview2_com::{
    AddScriptToExecuteOnDocumentCreatedCompletedHandler, ClearBrowsingDataCompletedHandler,
    CoreWebView2EnvironmentOptions, CreateCoreWebView2CompositionControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, CursorChangedEventHandler,
    DocumentTitleChangedEventHandler, ExecuteScriptCompletedHandler, FocusChangedEventHandler,
    NavigationCompletedEventHandler, WebMessageReceivedEventHandler, take_pwstr, wait_with_pump,
};
use windows::{
    Win32::{
        Foundation::{E_POINTER, HWND, RECT},
        Globalization::{
            GetUserDefaultUILanguage, LCIDToLocaleName, LOCALE_ALLOW_NEUTRAL_NAMES, MAX_LOCALE_NAME,
        },
        UI::{Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::HCURSOR},
    },
    core::{HSTRING, IUnknown, Interface, PCWSTR, PWSTR},
};

use webview2_com::Microsoft::Web::WebView2::Win32::*;

use crate::web_preview_view::{BrowserEvent, WEB_PREVIEW_BRIDGE_SCRIPT, push_browser_event};

const IPC_SHIM_SCRIPT: &str = r#"Object.defineProperty(window, 'ipc', { value: Object.freeze({ postMessage: s => window.chrome.webview.postMessage(s) }) });"#;
const DEFAULT_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";
const HOST_INPUT_BRIDGE_SCRIPT: &str = r#"
(() => {
  if (window.__zedHostInput) return;

  const resolveTarget = () => {
    const explicit = window.__zedHostInputTarget;
    if (explicit && explicit.isConnected) return explicit;
    if (document.activeElement && document.activeElement !== document.body) {
      return document.activeElement;
    }
    return null;
  };

  const fireInput = (element, data, inputType) => {
    element.dispatchEvent(new InputEvent('input', {
      bubbles: true,
      data,
      inputType,
    }));
  };

  const insertIntoEditable = (text) => {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return false;
    selection.deleteFromDocument();
    const range = selection.getRangeAt(0);
    const node = document.createTextNode(text);
    range.insertNode(node);
    range.setStartAfter(node);
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
    return true;
  };

  window.__zedHostInput = {
    setTarget(element) {
      window.__zedHostInputTarget = element;
    },
    insertText(text) {
      const element = resolveTarget();
      if (!element) return false;

      if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
        const start = element.selectionStart ?? element.value.length;
        const end = element.selectionEnd ?? start;
        element.setRangeText(text, start, end, 'end');
        fireInput(element, text, 'insertText');
        return true;
      }

      if (element instanceof HTMLElement && element.isContentEditable) {
        const inserted = insertIntoEditable(text);
        if (inserted) fireInput(element, text, 'insertText');
        return inserted;
      }

      return false;
    },
    keyDown(key) {
      const element = resolveTarget();
      if (!element) return false;

      if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
        const value = element.value;
        let start = element.selectionStart ?? value.length;
        let end = element.selectionEnd ?? start;

        if (key === 'Backspace') {
          if (start === end) start = Math.max(0, start - 1);
          element.setRangeText('', start, end, 'end');
          fireInput(element, null, 'deleteContentBackward');
          return true;
        }

        if (key === 'Delete') {
          if (start === end) end = Math.min(value.length, end + 1);
          element.setRangeText('', start, end, 'end');
          fireInput(element, null, 'deleteContentForward');
          return true;
        }

        if (key === 'Enter') {
          if (element instanceof HTMLTextAreaElement) {
            element.setRangeText('\n', start, end, 'end');
            fireInput(element, '\n', 'insertLineBreak');
            return true;
          }

          element.form?.requestSubmit?.();
          return true;
        }
      }

      if (element instanceof HTMLElement && element.isContentEditable) {
        if (key === 'Backspace') {
          document.execCommand('delete');
          fireInput(element, null, 'deleteContentBackward');
          return true;
        }

        if (key === 'Delete') {
          document.execCommand('forwardDelete');
          fireInput(element, null, 'deleteContentForward');
          return true;
        }

        if (key === 'Enter') {
          const inserted = insertIntoEditable('\n');
          if (inserted) fireInput(element, '\n', 'insertLineBreak');
          return inserted;
        }
      }

      return false;
    },
  };
})();
"#;

pub(crate) struct WindowsVisualWebView {
    main_window: HWND,
    controller: ICoreWebView2Controller,
    composition_controller: ICoreWebView2CompositionController,
    webview: ICoreWebView2,
    visual: windows::Win32::Graphics::DirectComposition::IDCompositionVisual,
    cursor_changed_token: i64,
    got_focus_token: i64,
    lost_focus_token: i64,
    last_bounds: Option<RECT>,
    visible: bool,
}

impl WindowsVisualWebView {
    pub(crate) fn new(
        main_window: HWND,
        profile_dir: PathBuf,
        initial_url: &str,
        zoom_factor: f64,
        scale_factor: f32,
        bounds: RECT,
        browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    ) -> Result<Self> {
        Self::new_internal(
            main_window,
            profile_dir,
            initial_url,
            zoom_factor,
            scale_factor,
            bounds,
            browser_events,
            true,
        )
    }

    pub(crate) fn new_hidden(
        main_window: HWND,
        profile_dir: PathBuf,
        initial_url: &str,
        zoom_factor: f64,
        scale_factor: f32,
        bounds: RECT,
        browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    ) -> Result<Self> {
        Self::new_internal(
            main_window,
            profile_dir,
            initial_url,
            zoom_factor,
            scale_factor,
            bounds,
            browser_events,
            false,
        )
    }

    fn new_internal(
        main_window: HWND,
        profile_dir: PathBuf,
        initial_url: &str,
        zoom_factor: f64,
        scale_factor: f32,
        bounds: RECT,
        browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
        initially_visible: bool,
    ) -> Result<Self> {
        let environment = create_environment(&profile_dir)?;
        let composition_controller = create_composition_controller(main_window, &environment)?;
        let controller: ICoreWebView2Controller = composition_controller.cast()?;
        let webview = unsafe { controller.CoreWebView2()? };
        let visual = create_webview_composition_visual(main_window)
            .with_context(|| "Failed to create the DirectComposition visual for the web preview")?;
        let visual_unknown: IUnknown = visual.cast()?;

        unsafe {
            composition_controller.SetRootVisualTarget(&visual_unknown)?;
            controller.SetIsVisible(initially_visible)?;
        }

        configure_webview_settings(&webview)?;
        attach_event_handlers(&webview, browser_events)?;
        let init_script = combined_init_script();
        add_init_script(&webview, init_script.as_str())?;

        unsafe {
            controller.SetZoomFactor(zoom_factor)?;
        }

        let mut this = Self {
            main_window,
            controller,
            composition_controller,
            webview,
            visual,
            cursor_changed_token: 0,
            got_focus_token: 0,
            lost_focus_token: 0,
            last_bounds: None,
            visible: initially_visible,
        };
        this.register_cursor_handler()?;
        this.register_focus_handlers()?;
        this.sync_cursor();
        this.set_bounds(bounds, scale_factor)?;
        this.load_url(initial_url)?;
        Ok(this)
    }

    pub(crate) fn load_url(&self, url: &str) -> Result<()> {
        let url = HSTRING::from(url);
        unsafe { self.webview.Navigate(&url) }.map_err(Into::into)
    }

    pub(crate) fn reload(&self) -> Result<()> {
        unsafe { self.webview.Reload() }.map_err(Into::into)
    }

    pub(crate) fn evaluate_script(&self, script: &str) -> Result<()> {
        let script = HSTRING::from(script);
        unsafe {
            self.webview.ExecuteScript(
                &script,
                &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
            )?
        };
        Ok(())
    }

    pub(crate) fn zoom(&self, scale_factor: f64) -> Result<()> {
        unsafe { self.controller.SetZoomFactor(scale_factor) }.map_err(Into::into)
    }

    pub(crate) fn clear_all_browsing_data(&self) -> Result<()> {
        unsafe {
            self.webview
                .cast::<ICoreWebView2_13>()?
                .Profile()?
                .cast::<ICoreWebView2Profile2>()?
                .ClearBrowsingDataAll(&ClearBrowsingDataCompletedHandler::create(Box::new(
                    move |_| Ok(()),
                )))
                .map_err(Into::into)
        }
    }

    pub(crate) fn open_devtools(&self) {
        unsafe {
            let _ = self.webview.OpenDevToolsWindow();
        }
    }

    pub(crate) fn focus_parent(&self) -> Result<()> {
        update_webview_passthrough_focus(self.main_window, false);
        unsafe { SetFocus(Some(self.main_window))? };
        Ok(())
    }

    pub(crate) fn set_visible(&mut self, visible: bool) -> Result<()> {
        self.visible = visible;
        unsafe {
            self.controller.SetIsVisible(visible)?;
        }
        if visible {
            if let Some(bounds) = self.last_bounds {
                register_webview_passthrough_target(
                    self.main_window,
                    self.composition_controller.clone(),
                    bounds,
                );
            }
        } else {
            clear_webview_passthrough_target(self.main_window);
            update_webview_passthrough_cursor(self.main_window, None);
            update_webview_passthrough_focus(self.main_window, false);
        }
        Ok(())
    }

    pub(crate) fn set_bounds(&mut self, bounds: RECT, scale_factor: f32) -> Result<()> {
        let width = (bounds.right - bounds.left).max(1);
        let height = (bounds.bottom - bounds.top).max(1);
        self.last_bounds = Some(bounds);

        set_webview_composition_visual_offset(
            self.main_window,
            &self.visual,
            bounds.left as f32,
            bounds.top as f32,
        )?;

        unsafe {
            self.controller.SetBounds(RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            })?;
            if let Ok(controller3) = self.controller.cast::<ICoreWebView2Controller3>() {
                let _ = controller3.SetRasterizationScale(scale_factor as f64);
            }
            let _ = self.controller.NotifyParentWindowPositionChanged();
        }

        if self.visible {
            register_webview_passthrough_target(
                self.main_window,
                self.composition_controller.clone(),
                bounds,
            );
        }

        Ok(())
    }

    fn register_cursor_handler(&mut self) -> Result<()> {
        let main_window = self.main_window;
        let handler = CursorChangedEventHandler::create(Box::new(move |controller, _| {
            if let Some(controller) = controller {
                let mut cursor = HCURSOR::default();
                if unsafe { controller.Cursor(&mut cursor) }.is_ok() {
                    update_webview_passthrough_cursor(
                        main_window,
                        (!cursor.0.is_null()).then_some(cursor),
                    );
                }
            }
            Ok(())
        }));
        unsafe {
            self.composition_controller
                .add_CursorChanged(&handler, &mut self.cursor_changed_token)?;
        }
        Ok(())
    }

    fn sync_cursor(&self) {
        unsafe {
            let mut cursor = HCURSOR::default();
            if self.composition_controller.Cursor(&mut cursor).is_ok() {
                update_webview_passthrough_cursor(
                    self.main_window,
                    (!cursor.0.is_null()).then_some(cursor),
                );
            }
        }
    }

    fn register_focus_handlers(&mut self) -> Result<()> {
        let main_window = self.main_window;
        let got_focus = FocusChangedEventHandler::create(Box::new(move |_, _| {
            update_webview_passthrough_focus(main_window, true);
            Ok(())
        }));
        let lost_focus = FocusChangedEventHandler::create(Box::new(move |_, _| {
            update_webview_passthrough_focus(main_window, false);
            Ok(())
        }));
        unsafe {
            self.controller
                .add_GotFocus(&got_focus, &mut self.got_focus_token)?;
            self.controller
                .add_LostFocus(&lost_focus, &mut self.lost_focus_token)?;
        }
        Ok(())
    }
}

impl Drop for WindowsVisualWebView {
    fn drop(&mut self) {
        clear_webview_passthrough_target(self.main_window);
        update_webview_passthrough_cursor(self.main_window, None);
        unsafe {
            if self.cursor_changed_token != 0 {
                let _ = self
                    .composition_controller
                    .remove_CursorChanged(self.cursor_changed_token);
            }
            if self.got_focus_token != 0 {
                let _ = self.controller.remove_GotFocus(self.got_focus_token);
            }
            if self.lost_focus_token != 0 {
                let _ = self.controller.remove_LostFocus(self.lost_focus_token);
            }
            let _ = self.controller.Close();
        }
        let _ = remove_webview_composition_visual(self.main_window, &self.visual);
    }
}

fn create_environment(profile_dir: &PathBuf) -> Result<ICoreWebView2Environment> {
    let data_directory = HSTRING::from(profile_dir.to_string_lossy().as_ref());
    let options = CoreWebView2EnvironmentOptions::default();
    let (tx, rx) = mpsc::channel();

    unsafe {
        options.set_additional_browser_arguments(DEFAULT_BROWSER_ARGS.to_string());
        options.set_are_browser_extensions_enabled(false);

        let lcid = GetUserDefaultUILanguage();
        let mut locale_name = [0; MAX_LOCALE_NAME as usize];
        LCIDToLocaleName(
            lcid as u32,
            Some(&mut locale_name),
            LOCALE_ALLOW_NEUTRAL_NAMES,
        );
        options.set_language(String::from_utf16_lossy(&locale_name));
        options.set_scroll_bar_style(COREWEBVIEW2_SCROLLBAR_STYLE_DEFAULT);

        CreateCoreWebView2EnvironmentWithOptions(
            PCWSTR::null(),
            &data_directory,
            &ICoreWebView2EnvironmentOptions::from(options),
            &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
                move |error_code, environment| {
                    error_code?;
                    let _ =
                        tx.send(environment.ok_or_else(|| windows::core::Error::from(E_POINTER)));
                    Ok(())
                },
            )),
        )?;
    }

    wait_with_pump(rx)?
        .map_err(anyhow::Error::from)
        .with_context(|| "Failed to create the WebView2 environment")
}

fn create_composition_controller(
    main_window: HWND,
    environment: &ICoreWebView2Environment,
) -> Result<ICoreWebView2CompositionController> {
    let (tx, rx) = mpsc::channel();
    let environment10 = environment.cast::<ICoreWebView2Environment10>();
    let handler = CreateCoreWebView2CompositionControllerCompletedHandler::create(Box::new(
        move |error_code, controller| {
            error_code?;
            let _ = tx.send(controller.ok_or_else(|| windows::core::Error::from(E_POINTER)));
            Ok(())
        },
    ));

    unsafe {
        if let Ok(environment10) = environment10 {
            let options = environment10.CreateCoreWebView2ControllerOptions()?;
            options.SetIsInPrivateModeEnabled(false)?;
            if let Ok(options3) = options.cast::<ICoreWebView2ControllerOptions3>() {
                options3.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                    R: 0,
                    G: 0,
                    B: 0,
                    A: 0,
                })?;
            }
            environment10.CreateCoreWebView2CompositionControllerWithOptions(
                main_window,
                &options,
                &handler,
            )?;
        } else {
            environment
                .cast::<ICoreWebView2Environment3>()
                .with_context(
                    || "The installed WebView2 runtime does not support composition hosting",
                )?
                .CreateCoreWebView2CompositionController(main_window, &handler)?;
        }
    }

    wait_with_pump(rx)?
        .map_err(anyhow::Error::from)
        .with_context(|| "Failed to create the WebView2 composition controller")
}

fn configure_webview_settings(webview: &ICoreWebView2) -> Result<()> {
    unsafe {
        let settings = webview.Settings()?;
        settings.SetIsStatusBarEnabled(false)?;
        settings.SetAreDefaultContextMenusEnabled(false)?;
        settings.SetIsZoomControlEnabled(false)?;
        settings.SetAreDevToolsEnabled(true)?;
        settings.SetIsScriptEnabled(true)?;

        if let Ok(settings5) = settings.cast::<ICoreWebView2Settings5>() {
            settings5.SetIsPinchZoomEnabled(false)?;
        }
        if let Ok(settings6) = settings.cast::<ICoreWebView2Settings6>() {
            settings6.SetIsSwipeNavigationEnabled(true)?;
        }
        if let Ok(settings9) = settings.cast::<ICoreWebView2Settings9>() {
            settings9.SetIsNonClientRegionSupportEnabled(true)?;
        }
    }
    Ok(())
}

fn attach_event_handlers(
    webview: &ICoreWebView2,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
) -> Result<()> {
    unsafe {
        let event_queue = browser_events.clone();
        let mut token = 0;
        webview.add_DocumentTitleChanged(
            &DocumentTitleChangedEventHandler::create(Box::new(move |webview, _| {
                let Some(webview) = webview else {
                    return Ok(());
                };
                let mut title = PWSTR::null();
                webview.DocumentTitle(&mut title)?;
                push_browser_event(&event_queue, BrowserEvent::TitleChanged(take_pwstr(title)));
                Ok(())
            })),
            &mut token,
        )?;

        let event_queue = browser_events.clone();
        webview.add_NavigationCompleted(
            &NavigationCompletedEventHandler::create(Box::new(move |webview, _| {
                let Some(webview) = webview else {
                    return Ok(());
                };
                let mut url = PWSTR::null();
                webview.Source(&mut url)?;
                push_browser_event(&event_queue, BrowserEvent::UrlChanged(take_pwstr(url)));
                Ok(())
            })),
            &mut token,
        )?;

        let event_queue = browser_events;
        webview.add_WebMessageReceived(
            &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else {
                    return Ok(());
                };
                let mut message = PWSTR::null();
                args.TryGetWebMessageAsString(&mut message)?;
                push_browser_event(&event_queue, BrowserEvent::IpcMessage(take_pwstr(message)));
                Ok(())
            })),
            &mut token,
        )?;
    }
    Ok(())
}

fn add_init_script(webview: &ICoreWebView2, script: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel::<()>();
    let webview = webview.clone();
    let script = HSTRING::from(script);

    unsafe {
        webview.AddScriptToExecuteOnDocumentCreated(
            &script,
            &AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(
                move |error_code, _| {
                    error_code?;
                    let _ = tx.send(());
                    Ok(())
                },
            )),
        )?;
    }

    wait_with_pump(rx).map_err(|_| anyhow!("The WebView2 initialization script was cancelled"))?;
    Ok(())
}

fn combined_init_script() -> String {
    format!(
        "{ipc}\n{bridge}\n{host}",
        ipc = IPC_SHIM_SCRIPT,
        bridge = WEB_PREVIEW_BRIDGE_SCRIPT,
        host = HOST_INPUT_BRIDGE_SCRIPT,
    )
}
