use std::time::Duration;

use gpui::{
    AnyElement, App, Bounds, Context, IntoElement, MouseButton, Pixels, Point, SharedString,
    canvas, div, relative,
};
use ui::{Window, prelude::*};

use crate::{ItemHandle, Workspace, WorkspaceScreenKind};

const EDGE_HITBOX: f32 = 10.;
const DWELL_MS: u64 = 180;
const IDLE_REVEAL_FRACTION: f32 = 0.08;
const KEYBOARD_REVEAL_FRACTION: f32 = 0.28;
const MAX_REVEAL_FRACTION: f32 = 0.42;
const COMMIT_FRACTION: f32 = 0.22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScreenCarouselEdge {
    Left,
    Right,
}

#[derive(Default)]
pub(super) struct ScreenCarouselState {
    pub(super) bounds: Option<Bounds<Pixels>>,
    pub(super) edge: Option<ScreenCarouselEdge>,
    pub(super) reveal_fraction: f32,
    pub(super) dragging: bool,
    pub(super) drag_moved: bool,
    pub(super) hover_generation: u64,
}

impl Workspace {
    fn active_screen_kind(&self, cx: &App) -> WorkspaceScreenKind {
        self.active_item(cx)
            .map(|item| item.screen_kind(cx))
            .unwrap_or(WorkspaceScreenKind::Editor)
    }

    fn adjacent_screen_kind(
        &self,
        edge: ScreenCarouselEdge,
        cx: &App,
    ) -> Option<WorkspaceScreenKind> {
        match (self.active_screen_kind(cx), edge) {
            (WorkspaceScreenKind::Editor, ScreenCarouselEdge::Left) => {
                Some(WorkspaceScreenKind::Terminal)
            }
            (WorkspaceScreenKind::Editor, ScreenCarouselEdge::Right) => {
                Some(WorkspaceScreenKind::Browser)
            }
            (WorkspaceScreenKind::Browser, ScreenCarouselEdge::Left) => {
                Some(WorkspaceScreenKind::Editor)
            }
            (WorkspaceScreenKind::Browser, ScreenCarouselEdge::Right) => {
                Some(WorkspaceScreenKind::Terminal)
            }
            (WorkspaceScreenKind::Terminal, ScreenCarouselEdge::Left) => {
                Some(WorkspaceScreenKind::Browser)
            }
            (WorkspaceScreenKind::Terminal, ScreenCarouselEdge::Right) => {
                Some(WorkspaceScreenKind::Editor)
            }
            (WorkspaceScreenKind::LiquidGlass | WorkspaceScreenKind::Other, _) => None,
        }
    }

    fn screen_item_for_kind(
        &self,
        kind: WorkspaceScreenKind,
        cx: &App,
    ) -> Option<Box<dyn ItemHandle>> {
        let screen_host_pane = self.screen_host_pane();
        screen_host_pane
            .read(cx)
            .items()
            .find_map(|item| (item.screen_kind(cx) == kind).then(|| item.boxed_clone()))
    }

    fn set_screen_carousel_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        if self.screen_carousel.bounds != Some(bounds) {
            self.screen_carousel.bounds = Some(bounds);
            cx.notify();
        }
    }

    fn screen_carousel_edge_at_position(
        &self,
        position: Point<Pixels>,
        cx: &App,
    ) -> Option<ScreenCarouselEdge> {
        let bounds = self.screen_carousel.bounds?;
        if !bounds.contains(&position) || self.zoomed.is_some() {
            return None;
        }

        let x = position.x.as_f32();
        let left = bounds.left().as_f32();
        let right = bounds.right().as_f32();

        if x - left <= EDGE_HITBOX
            && self
                .adjacent_screen_kind(ScreenCarouselEdge::Left, cx)
                .is_some()
        {
            Some(ScreenCarouselEdge::Left)
        } else if right - x <= EDGE_HITBOX
            && self
                .adjacent_screen_kind(ScreenCarouselEdge::Right, cx)
                .is_some()
        {
            Some(ScreenCarouselEdge::Right)
        } else {
            None
        }
    }

    fn arm_screen_carousel_dwell(
        &mut self,
        edge: ScreenCarouselEdge,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.screen_carousel.hover_generation =
            self.screen_carousel.hover_generation.wrapping_add(1);
        self.screen_carousel.edge = Some(edge);
        self.screen_carousel.reveal_fraction = 0.;
        self.screen_carousel.dragging = false;
        self.screen_carousel.drag_moved = false;

        let generation = self.screen_carousel.hover_generation;
        self._screen_carousel_reveal_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(DWELL_MS))
                .await;

            let _ = this.update_in(cx, |workspace, _window, cx| {
                if workspace.screen_carousel.edge == Some(edge)
                    && workspace.screen_carousel.hover_generation == generation
                    && !workspace.screen_carousel.dragging
                {
                    workspace.screen_carousel.reveal_fraction = IDLE_REVEAL_FRACTION;
                    cx.notify();
                }
            });
        }));
    }

    fn clear_screen_carousel_preview(&mut self, cx: &mut Context<Self>) {
        if self.screen_carousel.edge.is_some() || self.screen_carousel.reveal_fraction > 0. {
            self.screen_carousel.hover_generation =
                self.screen_carousel.hover_generation.wrapping_add(1);
            self.screen_carousel.edge = None;
            self.screen_carousel.reveal_fraction = 0.;
            self.screen_carousel.dragging = false;
            self.screen_carousel.drag_moved = false;
            self._screen_carousel_reveal_task = None;
            cx.notify();
        }
    }

    fn update_screen_carousel_pointer(
        &mut self,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(edge) = self.screen_carousel.edge
            && self.screen_carousel.dragging
            && event.dragging()
        {
            if let Some(bounds) = self.screen_carousel.bounds {
                let width = bounds.size.width.as_f32();
                if width <= 0. {
                    return;
                }

                let reveal_px = match edge {
                    ScreenCarouselEdge::Left => event.position.x - bounds.left(),
                    ScreenCarouselEdge::Right => bounds.right() - event.position.x,
                };
                let reveal_fraction =
                    (reveal_px.as_f32() / width).clamp(IDLE_REVEAL_FRACTION, MAX_REVEAL_FRACTION);
                if (self.screen_carousel.reveal_fraction - reveal_fraction).abs() > 0.005 {
                    self.screen_carousel.drag_moved = true;
                    self.screen_carousel.reveal_fraction = reveal_fraction;
                    cx.notify();
                }
            }
            return;
        }

        let edge = self.screen_carousel_edge_at_position(event.position, cx);
        if edge != self.screen_carousel.edge {
            if let Some(edge) = edge {
                self.arm_screen_carousel_dwell(edge, window, cx);
            } else {
                self.clear_screen_carousel_preview(cx);
            }
        }
    }

    fn begin_screen_carousel_drag(&mut self, edge: ScreenCarouselEdge, cx: &mut Context<Self>) {
        self.screen_carousel.hover_generation =
            self.screen_carousel.hover_generation.wrapping_add(1);
        self.screen_carousel.edge = Some(edge);
        self.screen_carousel.dragging = true;
        self.screen_carousel.drag_moved = false;
        self.screen_carousel.reveal_fraction = self
            .screen_carousel
            .reveal_fraction
            .max(IDLE_REVEAL_FRACTION);
        self._screen_carousel_reveal_task = None;
        cx.notify();
    }

    fn finish_screen_carousel_drag(
        &mut self,
        edge: ScreenCarouselEdge,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let should_commit = self.screen_carousel.edge == Some(edge)
            && (!self.screen_carousel.drag_moved
                || self.screen_carousel.reveal_fraction >= COMMIT_FRACTION);

        if should_commit {
            self.activate_adjacent_screen(edge, window, cx);
        } else {
            self.clear_screen_carousel_preview(cx);
        }
    }

    fn activate_adjacent_screen(
        &mut self,
        edge: ScreenCarouselEdge,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(kind) = self.adjacent_screen_kind(edge, cx) {
            self.activate_screen_kind(kind, window, cx);
        }
        self.clear_screen_carousel_preview(cx);
    }

    pub(super) fn activate_previous_screen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_adjacent_screen(ScreenCarouselEdge::Left, window, cx);
    }

    pub(super) fn activate_next_screen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_adjacent_screen(ScreenCarouselEdge::Right, window, cx);
    }

    fn peek_adjacent_screen(&mut self, edge: ScreenCarouselEdge, cx: &mut Context<Self>) {
        if self.adjacent_screen_kind(edge, cx).is_none() {
            return;
        }

        self.screen_carousel.hover_generation =
            self.screen_carousel.hover_generation.wrapping_add(1);
        self.screen_carousel.edge = Some(edge);
        self.screen_carousel.reveal_fraction = KEYBOARD_REVEAL_FRACTION;
        self.screen_carousel.dragging = false;
        self.screen_carousel.drag_moved = false;
        self._screen_carousel_reveal_task = None;
        cx.notify();
    }

    pub(super) fn peek_previous_screen(&mut self, cx: &mut Context<Self>) {
        self.peek_adjacent_screen(ScreenCarouselEdge::Left, cx);
    }

    pub(super) fn peek_next_screen(&mut self, cx: &mut Context<Self>) {
        self.peek_adjacent_screen(ScreenCarouselEdge::Right, cx);
    }

    fn screen_kind_label(kind: WorkspaceScreenKind) -> &'static str {
        match kind {
            WorkspaceScreenKind::Editor => "Editor",
            WorkspaceScreenKind::Browser => "Browser",
            WorkspaceScreenKind::Terminal => "Terminal",
            WorkspaceScreenKind::LiquidGlass => "Glass",
            WorkspaceScreenKind::Other => "Screen",
        }
    }

    fn screen_kind_icon(kind: WorkspaceScreenKind) -> IconName {
        match kind {
            WorkspaceScreenKind::Editor => IconName::Code,
            WorkspaceScreenKind::Browser => IconName::Public,
            WorkspaceScreenKind::Terminal => IconName::Terminal,
            WorkspaceScreenKind::LiquidGlass => IconName::Sparkle,
            WorkspaceScreenKind::Other => IconName::Circle,
        }
    }

    fn render_screen_carousel_preview(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let edge = self.screen_carousel.edge?;
        let target_kind = self.adjacent_screen_kind(edge, cx)?;
        let reveal_fraction = self
            .screen_carousel
            .reveal_fraction
            .clamp(0., MAX_REVEAL_FRACTION);

        if reveal_fraction <= 0. {
            return None;
        }

        let item = self.screen_item_for_kind(target_kind, cx);
        let title = item
            .as_ref()
            .map(|item| item.tab_content_text(0, cx))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| SharedString::from(Self::screen_kind_label(target_kind)));

        let preview_body = if let Some(item) = item {
            div()
                .size_full()
                .overflow_hidden()
                .child(item.to_any_view())
                .into_any_element()
        } else {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(Self::screen_kind_icon(target_kind)).size(IconSize::Small))
                        .child(Label::new(format!(
                            "Create {} Screen",
                            Self::screen_kind_label(target_kind)
                        ))),
                )
                .into_any_element()
        };

        let border_color = cx.theme().colors().border_variant;
        let background = cx.theme().colors().elevated_surface_background;
        let edge_id = match edge {
            ScreenCarouselEdge::Left => "screen-carousel-preview-left",
            ScreenCarouselEdge::Right => "screen-carousel-preview-right",
        };

        Some(
            div()
                .id(edge_id)
                .absolute()
                .top_0()
                .bottom_0()
                .w(relative(reveal_fraction))
                .overflow_hidden()
                .occlude()
                .bg(background)
                .border_color(border_color)
                .shadow_lg()
                .cursor_col_resize()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |workspace, _event, _window, cx| {
                        workspace.begin_screen_carousel_drag(edge, cx);
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |workspace, _event, window, cx| {
                        workspace.finish_screen_carousel_drag(edge, window, cx);
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |workspace, _event, window, cx| {
                        workspace.finish_screen_carousel_drag(edge, window, cx);
                        cx.stop_propagation();
                    }),
                )
                .when(edge == ScreenCarouselEdge::Left, |this| {
                    this.left_0().border_r_1()
                })
                .when(edge == ScreenCarouselEdge::Right, |this| {
                    this.right_0().border_l_1()
                })
                .child(
                    v_flex()
                        .size_full()
                        .overflow_hidden()
                        .child(
                            h_flex()
                                .flex_none()
                                .h_8()
                                .px_2()
                                .gap_1()
                                .items_center()
                                .border_b_1()
                                .border_color(border_color)
                                .child(
                                    Icon::new(Self::screen_kind_icon(target_kind))
                                        .size(IconSize::Small)
                                        .color(Color::Muted),
                                )
                                .child(Label::new(title).size(LabelSize::Small).truncate()),
                        )
                        .child(preview_body),
                )
                .into_any_element(),
        )
    }

    pub(super) fn render_screen_carousel_center(
        &self,
        center: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let bounds_recorder = {
            let this = cx.entity();
            canvas(
                move |bounds, _window, cx| {
                    let _ = this.update(cx, |workspace, cx| {
                        workspace.set_screen_carousel_bounds(bounds, cx);
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full()
        };

        div()
            .relative()
            .flex_1()
            .size_full()
            .overflow_hidden()
            .on_mouse_move(
                cx.listener(|workspace, event: &gpui::MouseMoveEvent, window, cx| {
                    workspace.update_screen_carousel_pointer(event, window, cx);
                }),
            )
            .child(center)
            .child(bounds_recorder)
            .children(self.render_screen_carousel_preview(cx))
            .into_any_element()
    }
}
