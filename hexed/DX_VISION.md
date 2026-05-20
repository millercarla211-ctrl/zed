# DX Vision: Fast Native Editor With Agentic Superpowers

## Product Thesis

DX should feel like the code editor developers already trust, with the missing modern surfaces built directly into the native workflow: local models, remote agents, web preview, terminal, media preview, shadcn/ui assets, icon and font tooling, and fast workspace navigation. The advantage is not to become an Electron agent shell. The advantage is to keep Zed-class performance and add the product features that make coding, previewing, designing, and shipping happen in one place.

## Differentiators

- Native performance first: keep the editor, terminal, previews, and agent surfaces on fast GPUI/Rust paths.
- Local model advantage: direct llama.cpp integration should stay first-class, model-aware, and benchmarked against Ollama and LM Studio.
- Real workspace surfaces: Browser, Terminal, Editor, media, icons, fonts, and shadcn/ui should be usable screens and panels, not mock sections.
- Agent freedom: keep Codex, Claude Code, ACP agents, remote models, and local models available instead of locking the product to one provider.
- Designer-developer bridge: keep assets, preview, shadcn/ui, and media workflows close to the code without slowing ordinary typing.

## Screen Dock Carousel

The screen dock should become the fast way to move between the main working modes:

- Editor is the default full-width coding screen.
- Browser and Terminal are adjacent full-width screens in the same screen loop.
- Moving the cursor to the left or right edge and pausing briefly reveals a resize affordance.
- Dragging inward peeks the adjacent screen with a smooth carousel feel.
- The screen loop wraps: dragging left from Editor reveals Terminal, and dragging right from Terminal reveals Editor.
- Clicking the visible adjacent screen sliver activates that screen and restores it to full width.
- Keyboard shortcuts should support both direct screen activation and temporary peeking.

The implementation must stay wired to real `WorkspaceScreenKind` items and the existing screen dock. It must not create decorative duplicate screens or fake preview panels.

## Rainbow Cursor Policy

The rainbow cursor is optional and should stay behind a performance gate. The first implementation priority is the screen dock carousel. A rainbow cursor should only ship if it can be implemented as an opt-in, editor-local, GPU-cheap cursor-layer effect with no measurable typing or frame-time regression. If it requires repainting broad editor surfaces every frame, it should remain out of product builds.

## Launch Bar

- No dummy UI: every visible control should activate a real screen, panel, command, or workflow.
- No typing lag: ordinary editor repaint, cursor blink, and text input must not pay for preview or dock metadata work.
- One coherent git history: upstream syncs, feature commits, and verification notes should stay separated.
- Verification should respect this repo's scale: source review, whitespace checks, and one final `just run` when the batch is ready.
