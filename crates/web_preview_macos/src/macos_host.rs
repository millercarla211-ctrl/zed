use anyhow::{Context as _, Result, anyhow};
use cocoa::{
    appkit::{NSBackingStoreBuffered, NSView, NSWindow, NSWindowStyleMask},
    base::{NO, YES, id, nil},
    foundation::{NSPoint, NSRect, NSSize},
};
use gpui::{Bounds, Pixels, Window};
use image::RgbaImage;
use objc::{
    class, msg_send, sel,
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
use objc2_app_kit::{NSPNGFileType, NSWindow as Objc2NSWindow, NSWindowOrderingMode};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::Cell, slice, sync::OnceLock};

pub(crate) struct MacPreviewHost {
    gpui_view: Cell<id>,
    parent_window: Cell<id>,
    host_window: id,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MacHostTarget {
    pub(crate) gpui_view: usize,
    pub(crate) parent_window: usize,
}

impl MacPreviewHost {
    pub(crate) fn new(window: &Window, initial_bounds: Option<Bounds<Pixels>>) -> Result<Self> {
        let (gpui_view, parent_window) = gpui_parent_handles(window)?;
        let initial_frame = initial_bounds
            .map(|bounds| screen_rect_for_preview_bounds(gpui_view, parent_window, bounds))
            .transpose()?
            .unwrap_or_else(default_host_rect);

        let host_window: id = unsafe {
            let allocated: id = msg_send![web_preview_host_window_class(), alloc];
            let initialized: id = msg_send![
                allocated,
                initWithContentRect: initial_frame
                styleMask: NSWindowStyleMask::NSBorderlessWindowMask
                backing: NSBackingStoreBuffered
                defer: NO
            ];
            if initialized.is_null() {
                return Err(anyhow!(
                    "Failed to allocate the macOS web preview host window"
                ));
            }

            let clear_color: id = msg_send![class!(NSColor), clearColor];
            let parent_level: i64 = msg_send![parent_window, level];
            let parent_collection_behavior: u64 = msg_send![parent_window, collectionBehavior];
            let _: () = msg_send![initialized, setReleasedWhenClosed: NO];
            let _: () = msg_send![initialized, setOpaque: NO];
            let _: () = msg_send![initialized, setBackgroundColor: clear_color];
            let _: () = msg_send![initialized, setHasShadow: NO];
            let _: () = msg_send![initialized, setMovable: NO];
            let _: () = msg_send![initialized, setAcceptsMouseMovedEvents: YES];
            let _: () = msg_send![initialized, setLevel: parent_level];
            let _: () = msg_send![
                initialized,
                setCollectionBehavior: parent_collection_behavior
            ];
            let _: () = msg_send![
                parent_window,
                addChildWindow: initialized
                ordered: NSWindowOrderingMode::Below
            ];
            let _: () = msg_send![initialized, orderOut: nil];
            initialized
        };

        Ok(Self {
            gpui_view: Cell::new(gpui_view),
            parent_window: Cell::new(parent_window),
            host_window,
        })
    }

    pub(crate) fn ns_window_ptr(&self) -> *mut Objc2NSWindow {
        self.host_window.cast()
    }

    pub(crate) fn set_bounds(&self, bounds: Bounds<Pixels>) -> Result<()> {
        let frame =
            screen_rect_for_preview_bounds(self.gpui_view.get(), self.parent_window.get(), bounds)?;
        unsafe {
            let _: () = msg_send![self.host_window, setFrame: frame display: NO];
        }
        self.sync_parent_window_state();
        Ok(())
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        unsafe {
            let is_visible: bool = msg_send![self.host_window, isVisible];
            if visible {
                self.sync_parent_window_state();
            } else if is_visible {
                let _: () = msg_send![self.host_window, orderOut: nil];
            }
        }
    }

    pub(crate) fn focus_gpui_view(&self) {
        unsafe {
            let _: () = msg_send![self.parent_window.get(), makeFirstResponder: self.gpui_view.get()];
        }
    }

    pub(crate) fn sync_target(&self, window: &Window) -> Result<MacHostTarget> {
        let (gpui_view, parent_window) = gpui_parent_handles(window)?;
        let current_gpui_view = self.gpui_view.get();
        let current_parent_window = self.parent_window.get();
        if gpui_view != current_gpui_view || parent_window != current_parent_window {
            unsafe {
                if current_parent_window != nil {
                    let _: () =
                        msg_send![current_parent_window, removeChildWindow: self.host_window];
                }
                let _: () = msg_send![
                    parent_window,
                    addChildWindow: self.host_window
                    ordered: NSWindowOrderingMode::Below
                ];
            }
            self.gpui_view.set(gpui_view);
            self.parent_window.set(parent_window);
        }
        self.sync_parent_window_state();
        Ok(MacHostTarget {
            gpui_view: gpui_view as usize,
            parent_window: parent_window as usize,
        })
    }

    fn sync_parent_window_state(&self) {
        unsafe {
            let parent_window = self.parent_window.get();
            let parent_level: i64 = msg_send![parent_window, level];
            let parent_collection_behavior: u64 = msg_send![parent_window, collectionBehavior];
            let parent_window_number: i64 = msg_send![parent_window, windowNumber];
            let _: () = msg_send![self.host_window, setLevel: parent_level];
            let _: () = msg_send![
                self.host_window,
                setCollectionBehavior: parent_collection_behavior
            ];
            let _: () = msg_send![
                self.host_window,
                orderWindow: NSWindowOrderingMode::Below
                relativeTo: parent_window_number
            ];
        }
    }

    pub(crate) fn capture_image(&self) -> Result<RgbaImage> {
        unsafe {
            let content_view: id = msg_send![self.host_window, contentView];
            if content_view == nil {
                return Err(anyhow!(
                    "The macOS web preview host window has no content view to capture"
                ));
            }

            let bounds: NSRect = msg_send![content_view, bounds];
            if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
                return Err(anyhow!("The macOS web preview host bounds are empty"));
            }

            let bitmap_rep: id =
                msg_send![content_view, bitmapImageRepForCachingDisplayInRect: bounds];
            if bitmap_rep == nil {
                return Err(anyhow!(
                    "Failed to create a macOS bitmap representation for the web preview"
                ));
            }

            let _: () = msg_send![
                content_view,
                cacheDisplayInRect: bounds
                toBitmapImageRep: bitmap_rep
            ];

            let properties: id = msg_send![class!(NSDictionary), dictionary];
            let png_data: id = msg_send![
                bitmap_rep,
                representationUsingType: NSPNGFileType.0 as usize
                properties: properties
            ];
            if png_data == nil {
                return Err(anyhow!(
                    "Failed to encode the macOS web preview snapshot as PNG"
                ));
            }

            let length: usize = msg_send![png_data, length];
            let bytes: *const u8 = msg_send![png_data, bytes];
            if bytes.is_null() || length == 0 {
                return Err(anyhow!(
                    "The macOS web preview snapshot returned no image bytes"
                ));
            }

            let png_bytes = slice::from_raw_parts(bytes, length);
            image::load_from_memory(png_bytes)
                .map(|image| image.to_rgba8())
                .with_context(|| "Failed to decode the macOS web preview snapshot")
        }
    }
}

fn web_preview_host_window_class() -> &'static Class {
    static CLASS: OnceLock<&'static Class> = OnceLock::new();
    CLASS.get_or_init(|| unsafe {
        extern "C" fn can_become_key_window(_: &Object, _: Sel) -> bool {
            true
        }

        extern "C" fn can_become_main_window(_: &Object, _: Sel) -> bool {
            true
        }

        let superclass = class!(NSWindow);
        let mut decl = ClassDecl::new("ZedWebPreviewHostWindow", superclass)
            .expect("failed to create macOS web preview host window subclass");
        decl.add_method(
            sel!(canBecomeKeyWindow),
            can_become_key_window as extern "C" fn(&Object, Sel) -> bool,
        );
        decl.add_method(
            sel!(canBecomeMainWindow),
            can_become_main_window as extern "C" fn(&Object, Sel) -> bool,
        );
        decl.register()
    })
}

impl Drop for MacPreviewHost {
    fn drop(&mut self) {
        unsafe {
            let parent_window = self.parent_window.get();
            if parent_window != nil {
                let _: () = msg_send![parent_window, removeChildWindow: self.host_window];
            }
            let _: () = msg_send![self.host_window, orderOut: nil];
            let _: () = msg_send![self.host_window, close];
        }
    }
}

fn gpui_parent_handles(window: &Window) -> Result<(id, id)> {
    let handle = HasWindowHandle::window_handle(window)?;
    let RawWindowHandle::AppKit(raw_handle) = handle.as_raw() else {
        return Err(anyhow!(
            "Unsupported macOS window handle for native web preview"
        ));
    };

    let gpui_view = raw_handle.ns_view.as_ptr() as id;
    let parent_window: id = unsafe { msg_send![gpui_view, window] };
    if parent_window == nil {
        return Err(anyhow!(
            "The GPUI AppKit view is not attached to an NSWindow yet"
        ));
    }

    Ok((gpui_view, parent_window))
}

pub(crate) fn current_host_target(window: &Window) -> Result<MacHostTarget> {
    let (gpui_view, parent_window) = gpui_parent_handles(window)?;
    Ok(MacHostTarget {
        gpui_view: gpui_view as usize,
        parent_window: parent_window as usize,
    })
}

fn default_host_rect() -> NSRect {
    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(32.0, 32.0))
}

fn screen_rect_for_preview_bounds(
    gpui_view: id,
    parent_window: id,
    bounds: Bounds<Pixels>,
) -> Result<NSRect> {
    let local_rect = view_rect_for_preview_bounds(gpui_view, bounds);
    let rect_in_window = unsafe { NSView::convertRect_toView_(gpui_view, local_rect, nil) };
    let screen_rect: NSRect =
        unsafe { msg_send![parent_window, convertRectToScreen: rect_in_window] };
    Ok(screen_rect)
}

fn view_rect_for_preview_bounds(gpui_view: id, bounds: Bounds<Pixels>) -> NSRect {
    let width = f32::from(bounds.size.width).max(1.0) as f64;
    let height = f32::from(bounds.size.height).max(1.0) as f64;
    let x = f32::from(bounds.origin.x) as f64;
    let y = f32::from(bounds.origin.y) as f64;

    unsafe {
        let is_flipped = NSView::isFlipped(gpui_view) == YES;
        let frame = NSView::frame(gpui_view);
        let origin_y = if is_flipped {
            y
        } else {
            frame.size.height - y - height
        };
        NSRect::new(NSPoint::new(x, origin_y), NSSize::new(width, height))
    }
}
