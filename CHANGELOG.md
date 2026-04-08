# Changelog

All notable changes to this Codex fork will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Fixed
- Web preview toolbar action icons now stay muted at rest and only switch to the primary accent during hover and press states.
- Web preview URL editing no longer forces focus away after navigation and avoids overwriting in-progress input while the page reports URL updates.
- Web preview screenshots now copy the captured image to the clipboard and insert image plus URL attachments into the AI composer.
- Dragging a Web Preview tab into the agent panel now inserts the current page URL as a chat attachment.
- Web preview element selection now sends selected-element context into the AI composer with DOM details and a captured element snapshot on supported platforms.
- Removed the persistent top-bar notice above the web preview and made the extensions action available even before the first extension scan.
- GPUI workspace toasts now render from the top-right instead of the bottom edge, and web preview actions use the normal toast path again.
- Web preview screenshot and element-selector actions now guard against action-path panics and shrink oversized AI image attachments before insertion.
- Windows web preview now mounts into a separate underlay host window, and the GPUI preview body stops painting an opaque editor background over the browser region.
- Windows no longer forces `GPUI_DISABLE_DIRECT_COMPOSITION=1` at startup, which is required for alpha-based hole-punch composition work.
- Workspace items can now opt into a transparent workspace background, and web preview uses that path so the underlay browser surface is no longer blocked by the workspace root fill.
- Windows web preview wheel input now routes through the focused/root native webview window instead of the deepest child hit target, which restores mouse-wheel scrolling in the inline preview.
- GPUI now yields cursor ownership while the pointer is over the web preview passthrough body so the native webview cursor no longer flickers against GPUI over video regions.
- Windows web preview mouse-move relay now also sends native `WM_SETCURSOR`, so browser hover and cursor updates can work through the GPUI overlay path.
- Windows web preview body no longer registers GPUI mouse listeners, so the native underlay hole can keep ownership of hover and wheel input instead of fighting the relay path.

---

For upstream Zed changes, see the [official releases page](https://github.com/zed-industries/zed/releases).
