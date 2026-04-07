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

---

For upstream Zed changes, see the [official releases page](https://github.com/zed-industries/zed/releases).
