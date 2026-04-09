# Windows Web Preview Implementation Report

This repository’s detailed Windows implementation report currently lives at:

- [hexed/WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md](/F:/dx/hexed/WINDOWS_WEB_PREVIEW_IMPLEMENTATION_REPORT.md)

## Current Policy

- The Windows web preview is the completed reference implementation.
- The Windows rendering, input, focus, and z-index path is frozen unless a confirmed Windows regression is reported.
- macOS and Linux work must stay in their own backend crates and must not route through the Windows implementation.

## Why This Root File Exists

Several repo documents, including the current web preview status report and AGENT guidance, refer to a root-level Windows implementation report. This file preserves that entry point while the canonical detailed report remains under `hexed/`.

## Short Summary

The Windows implementation achieves:

- native WebView2 rendering
- GPUI chrome above the page
- working click, hover, wheel, and keyboard interaction
- stable URL-bar focus handoff
- solved z-index/airspace behavior for the inline editor preview

For the full architecture and freeze guidance, use the linked detailed report above.
