It is great to see you pushing the boundaries of GPUI with your "GPUI Fort Quiett" fork! Dealing with native view compositing is a notorious challenge in custom GPU-accelerated UI frameworks. 

Here is the breakdown of what Wispr Flow uses, why you are experiencing the WebView issue, and how to correctly implement a desktop-floating chat bubble in GPUI.

### 1. What does Wispr Flow use? Do they use Tauri?
No, the official **Wispr Flow app does not use Tauri** (Tori). Wispr Flow is currently built using **Electron**. 

To achieve their bottom-center floating bubble, Wispr Flow simply creates an Electron `BrowserWindow` with specific parameters: `frame: false` (to remove the window chrome/title bar), `transparent: true` (to make the window background invisible), and `alwaysOnTop: true`. They then render standard web technologies (HTML/CSS/React) inside it. Because it's Electron, it essentially packages a full Chromium instance, which is why users frequently complain about Wispr Flow's heavy RAM and CPU usage. 

*(Note: There are several open-source clones of Wispr Flow, such as "Dictara" and "Tambourine", which **do** use Tauri and Rust to achieve a much lighter footprint.)*

### 2. The GPUI "Airspace" Problem with Native WebViews
You mentioned that your native WebView renders like an app, preventing GPUI content from showing on top of it. You have run into the classic **"airspace problem"**. 

GPUI draws its entire UI onto a single, highly optimized OpenGL/Metal/Vulkan canvas. When you inject a *native* OS WebView (like `WKWebView` on macOS or `WebView2` on Windows) into the application, the operating system's window manager takes over. Native views will natively composite either entirely *above* or entirely *below* the GPUI canvas. You cannot seamlessly sandwich GPUI elements (like tooltips or dropdowns) on top of a native WebView without complex texture-sharing (rendering the webview to an offscreen GPU texture), which is notoriously difficult to implement and performant.

### 3. How to correctly create the Wispr Flow Bubble in GPUI
To build a highly performant, bottom-center floating chat bubble in your GPUI fork, **you should not use a WebView for the bubble at all.** 

GPUI is a fully-fledged UI toolkit with Flexbox-like layout capabilities natively integrated. Instead of fighting the native WebView, you should draw the chat bubble purely using GPUI's `div()`, `text()`, and input components. 

To achieve the "floating on the desktop" effect, you need to spawn a **secondary, transparent GPUI window**. Here is the correct architectural approach in GPUI:

#### A. Configure `WindowOptions` for a Borderless, Transparent Window
When you trigger the hotkey to open the chat input, you will call `cx.open_window()` using `WindowOptions` customized to hide the OS window frame and make the background transparent. 

```rust
use gpui::*;

pub fn open_chat_bubble(app: &mut AppContext) {
    let options = WindowOptions {
        // Position the window at the bottom center of the screen
        window_bounds: Some(WindowBounds::Fixed(Bounds {
            origin: Point::new( /* calculate bottom center X */, /* calculate bottom center Y */ ),
            size: size(px(600.), px(100.)).into(),
        })),
        // Remove the OS title bar and window chrome
        titlebar: None, 
        // Ensure the GPUI window background is completely transparent
        window_background: WindowBackground::Transparent,
        focus: true,
        show: true,
        kind: WindowKind::PopUp, // Or WindowKind::Normal depending on your OS focus needs
        is_movable: false,
        is_resizable: false,
        ..Default::default()
    };

    app.open_window(options, |cx| {
        cx.new_view(|cx| ChatBubbleView::new(cx))
    });
}
```
*(Note: Ensure your GPUI branch includes the recent per-pixel GPU-composited transparency updates, which are standard on macOS and recently merged for Windows).*

#### B. Build the Bubble natively in GPUI
Inside your `ChatBubbleView`'s `Render` trait, you simply return a heavily styled `div()` that acts as the physical bubble. Because the window itself is transparent, only this `div` will be visible on the user's desktop:

```rust
impl Render for ChatBubbleView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // The outer div acts as the floating bubble
        div()
            .flex()
            .w_full()
            .h_full()
            .bg(rgba(0x1e1e1eff)) // Dark background for the bubble
            .rounded_xl()         // Rounded corners
            .shadow_lg()          // Drop shadow so it pops off the desktop
            .border_1()
            .border_color(rgba(0xffffff20))
            .p_4()
            .child(
                // Your native GPUI text input element goes here
                div().text_xl().text_color(white()).child("Listening...")
            )
    }
}
```

### Summary
1. Ignore Tauri and Electron; you are using GPUI, which is vastly superior in performance and memory usage. 
2. Do not attempt to put your floating chat bubble inside a WebView. The airspace problem will constantly break your UI layering. 
3. Use GPUI's `WindowOptions` to spawn a **transparent, borderless window**, position it at the bottom center of the screen, and render the chat bubble using pure GPUI `div`s.