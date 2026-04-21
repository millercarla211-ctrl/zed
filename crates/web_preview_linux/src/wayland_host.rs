use anyhow::{Result, anyhow};
use gdkwayland::WaylandWindow;
use gpui::{Bounds, Pixels, point, px, size};
use gtk::{Fixed, Inhibit, Window, WindowType, cairo, glib::object::Cast, prelude::*};
use image::{RgbaImage, imageops};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub(crate) struct WaylandPreviewHost {
    window: Window,
    fixed: Fixed,
    exported_parent_handle: Rc<RefCell<String>>,
    webview_bounds: Rc<RefCell<Bounds<Pixels>>>,
}

impl WaylandPreviewHost {
    pub(crate) fn new(
        exported_parent_handle: &str,
        initial_window_bounds: Option<Bounds<Pixels>>,
        initial_webview_bounds: Option<Bounds<Pixels>>,
    ) -> Result<Self> {
        let window = Window::new(WindowType::Toplevel);
        window.set_decorated(false);
        window.set_deletable(false);
        window.set_resizable(false);
        window.set_destroy_with_parent(true);
        window.set_keep_below(true);
        window.set_accept_focus(true);
        window.set_focus_on_map(false);
        window.set_can_focus(true);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_app_paintable(true);
        if let Some(screen) = window.screen() {
            if let Some(visual) = screen.rgba_visual() {
                window.set_visual(Some(&visual));
            }
        }
        install_transparent_background(&window);

        let fixed = Fixed::new();
        fixed.set_app_paintable(true);
        fixed.set_can_focus(false);
        install_transparent_background(&fixed);
        window.add(&fixed);

        apply_window_bounds(&window, &fixed, initial_window_bounds);
        window.realize();

        let gdk_window = window.window().ok_or_else(|| {
            anyhow!("The GTK Wayland host window did not expose a native GDK window")
        })?;
        let gdk_window = gdk_window.downcast::<WaylandWindow>().map_err(|_| {
            anyhow!("The GTK host window is not running on the Wayland GDK backend")
        })?;

        if !gdk_window.set_transient_for_exported(exported_parent_handle) {
            return Err(anyhow!(
                "Failed to attach the GTK Wayland host window to the exported GPUI parent handle"
            ));
        }

        let webview_bounds = initial_webview_bounds.unwrap_or_else(|| {
            Bounds::new(point(Pixels::ZERO, Pixels::ZERO), size(px(32.0), px(32.0)))
        });

        Ok(Self {
            window,
            fixed,
            exported_parent_handle: Rc::new(RefCell::new(exported_parent_handle.to_string())),
            webview_bounds: Rc::new(RefCell::new(webview_bounds)),
        })
    }

    pub(crate) fn container(&self) -> &Fixed {
        &self.fixed
    }

    pub(crate) fn set_layout(&self, window_bounds: Bounds<Pixels>, webview_bounds: Bounds<Pixels>) {
        apply_window_bounds(&self.window, &self.fixed, Some(window_bounds));
        sync_webview_child_bounds(&self.fixed, webview_bounds);
        *self.webview_bounds.borrow_mut() = webview_bounds;
        self.lower();
    }

    pub(crate) fn set_exported_parent_handle(&self, exported_parent_handle: &str) -> Result<()> {
        if self.exported_parent_handle.borrow().as_str() == exported_parent_handle {
            return Ok(());
        }
        let gdk_window = self.window.window().ok_or_else(|| {
            anyhow!("The GTK Wayland host window did not expose a native GDK window")
        })?;
        let gdk_window = gdk_window.downcast::<WaylandWindow>().map_err(|_| {
            anyhow!("The GTK host window is not running on the Wayland GDK backend")
        })?;

        if gdk_window.set_transient_for_exported(exported_parent_handle) {
            *self.exported_parent_handle.borrow_mut() = exported_parent_handle.to_string();
            self.lower();
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to reattach the GTK Wayland host window to the exported GPUI parent handle"
            ))
        }
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        if visible {
            if !self.window.is_visible() {
                self.window.show_all();
            }
            self.lower();
        } else if self.window.is_visible() {
            self.window.hide();
        }
    }

    pub(crate) fn capture_image(&self) -> Result<RgbaImage> {
        let image = capture_host_window_image(&self.window)?;
        crop_to_webview_bounds(
            image,
            *self.webview_bounds.borrow(),
            self.window.allocated_width(),
            self.window.allocated_height(),
        )
    }

    fn lower(&self) {
        if let Some(gdk_window) = self.window.window() {
            gdk_window.lower();
        }
    }
}

impl Drop for WaylandPreviewHost {
    fn drop(&mut self) {
        self.window.hide();
        self.window.close();
    }
}

fn apply_window_bounds(window: &Window, fixed: &Fixed, bounds: Option<Bounds<Pixels>>) {
    let width = bounds
        .map(|bounds| f32::from(bounds.size.width).round().max(1.0) as i32)
        .unwrap_or(32);
    let height = bounds
        .map(|bounds| f32::from(bounds.size.height).round().max(1.0) as i32)
        .unwrap_or(32);

    fixed.set_size_request(width, height);
    window.resize(width, height);
}

fn sync_webview_child_bounds(fixed: &Fixed, bounds: Bounds<Pixels>) {
    let x = f32::from(bounds.origin.x).round() as i32;
    let y = f32::from(bounds.origin.y).round() as i32;
    let width = f32::from(bounds.size.width).round().max(1.0) as i32;
    let height = f32::from(bounds.size.height).round().max(1.0) as i32;

    for child in fixed.children() {
        fixed.move_(&child, x, y);
        child.set_size_request(width, height);
    }
}

fn install_transparent_background(widget: &impl IsA<gtk::Widget>) {
    widget.connect_draw(|_, cr| {
        cr.set_operator(cairo::Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);
        Inhibit(true)
    });
}

fn capture_host_window_image(window: &Window) -> Result<RgbaImage> {
    let gdk_window = window
        .window()
        .ok_or_else(|| anyhow!("The GTK Wayland host window did not expose a native GDK window"))?;
    let width = window.allocated_width().max(1);
    let height = window.allocated_height().max(1);
    let pixbuf = gdk_window
        .pixbuf(0, 0, width, height)
        .ok_or_else(|| anyhow!("Failed to capture the Linux Wayland web preview host window"))?;
    pixbuf_to_rgba_image(&pixbuf)
}

fn pixbuf_to_rgba_image(pixbuf: &gdk_pixbuf::Pixbuf) -> Result<RgbaImage> {
    let width = pixbuf.width().max(1) as usize;
    let height = pixbuf.height().max(1) as usize;
    let rowstride = pixbuf.rowstride().max(1) as usize;
    let channels = pixbuf.n_channels().max(3) as usize;
    let has_alpha = pixbuf.has_alpha();
    let bytes = pixbuf.read_pixel_bytes();
    let pixels = bytes.as_ref();
    let mut rgba = vec![0u8; width * height * 4];

    for y in 0..height {
        let src_row = y * rowstride;
        let dst_row = y * width * 4;
        for x in 0..width {
            let src = src_row + x * channels;
            let dst = dst_row + x * 4;
            rgba[dst] = pixels[src];
            rgba[dst + 1] = pixels[src + 1];
            rgba[dst + 2] = pixels[src + 2];
            rgba[dst + 3] = if has_alpha && channels >= 4 {
                pixels[src + 3]
            } else {
                255
            };
        }
    }

    RgbaImage::from_vec(width as u32, height as u32, rgba)
        .ok_or_else(|| anyhow!("Failed to construct the Linux Wayland screenshot image buffer"))
}

fn crop_to_webview_bounds(
    image: RgbaImage,
    bounds: Bounds<Pixels>,
    host_width: i32,
    host_height: i32,
) -> Result<RgbaImage> {
    let host_width = host_width.max(1) as f32;
    let host_height = host_height.max(1) as f32;
    let scale_x = image.width() as f32 / host_width;
    let scale_y = image.height() as f32 / host_height;

    let x = (f32::from(bounds.origin.x) * scale_x).round().max(0.0) as u32;
    let y = (f32::from(bounds.origin.y) * scale_y).round().max(0.0) as u32;
    let width = (f32::from(bounds.size.width) * scale_x).round().max(1.0) as u32;
    let height = (f32::from(bounds.size.height) * scale_y).round().max(1.0) as u32;
    let image_width = image.width();
    let image_height = image.height();

    if x >= image_width || y >= image_height {
        return Err(anyhow!(
            "The Linux Wayland web preview bounds are outside the captured host image"
        ));
    }

    let cropped_width = width.min(image_width.saturating_sub(x));
    let cropped_height = height.min(image_height.saturating_sub(y));
    Ok(imageops::crop_imm(&image, x, y, cropped_width, cropped_height).to_image())
}
