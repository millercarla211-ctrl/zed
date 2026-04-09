# Hole Punching

This repository’s earlier detailed hole-punching notes currently live at:

- [hexed/HOLE_PUNCHING.md](/F:/dx/hexed/HOLE_PUNCHING.md)

## Current Summary

The required architecture is:

1. keep the native browser surface behind GPUI
2. mark the web preview body as a transparent/passthrough region in GPUI
3. let GPUI remain the top composited UI layer for editor chrome, menus, and overlays
4. keep platform-specific hosting and hit-testing logic isolated by OS

## Platform State

- Windows: completed and frozen
- macOS: underlay-safe native path in progress
- Linux X11: child-host path plus GPUI passthrough path in progress
- Linux Wayland: dedicated compositor-specific host path in progress

This root file restores the expected documentation entry point while the historical detailed notes remain under `hexed/`.
