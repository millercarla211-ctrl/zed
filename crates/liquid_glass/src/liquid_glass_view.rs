use std::{cell::RefCell, rc::Rc, sync::Arc};

use anyhow::Result;
use gpui::{
    App, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable, Half, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Task, WeakEntity, Window, canvas, fill, point, px, size,
};
use project::Project;
use ui::{
    Color, ContextMenu, Divider, DropdownMenu, DropdownStyle, IconName, IconPosition, Label,
    SwitchField, ToggleState, prelude::*,
};
use workspace::{Item, ItemId, SerializableItem, Workspace, WorkspaceId, item::ItemEvent};

use crate::{
    backgrounds::{BackgroundAsset, load_backgrounds},
    ui_state::{GLASS_VARIANTS, UiState},
};

pub struct LiquidGlassView {
    backgrounds: Arc<[BackgroundAsset]>,
    focus_handle: FocusHandle,
    preview_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    slider_bounds: Rc<RefCell<Vec<(SliderKind, Bounds<Pixels>)>>>,
    active_slider: Option<SliderKind>,
    state: UiState,
    use_preview_center: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliderKind {
    Power,
    Width,
    Height,
    ChromaticAberration,
    AberrationSamples,
    BlurIterations,
    BlurRadius,
    BlurDownscale,
    Noise,
    FPower,
    A,
    B,
    C,
    D,
    GlowWeight,
    GlowBias,
    GlowEdge0,
    GlowEdge1,
}

#[derive(Clone, Copy)]
struct SliderSpec {
    kind: SliderKind,
    label: &'static str,
    min: f32,
    max: f32,
    precision: usize,
}

const SHAPE_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        kind: SliderKind::Power,
        label: "Power",
        min: 1.001,
        max: 6.0,
        precision: 3,
    },
    SliderSpec {
        kind: SliderKind::Width,
        label: "Width",
        min: 0.0,
        max: 10.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::Height,
        label: "Height",
        min: 0.0,
        max: 10.0,
        precision: 2,
    },
];

const ABERRATION_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        kind: SliderKind::ChromaticAberration,
        label: "Aberration",
        min: 0.0,
        max: 0.02,
        precision: 4,
    },
    SliderSpec {
        kind: SliderKind::AberrationSamples,
        label: "Quality",
        min: 1.0,
        max: 8.0,
        precision: 0,
    },
];

const BLUR_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        kind: SliderKind::BlurIterations,
        label: "Blur Iters",
        min: 0.0,
        max: 10.0,
        precision: 0,
    },
    SliderSpec {
        kind: SliderKind::BlurRadius,
        label: "Blur Radius",
        min: 0.0,
        max: 10.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::BlurDownscale,
        label: "Blur Downscale",
        min: 0.1,
        max: 1.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::Noise,
        label: "Noise",
        min: 0.0,
        max: 0.3,
        precision: 3,
    },
];

const REFRACTION_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        kind: SliderKind::FPower,
        label: "f(x) Power",
        min: -1.5,
        max: 6.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::A,
        label: "a",
        min: 0.0,
        max: 5.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::B,
        label: "b",
        min: 0.0,
        max: 6.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::C,
        label: "c",
        min: 0.0,
        max: 6.0,
        precision: 2,
    },
    SliderSpec {
        kind: SliderKind::D,
        label: "d",
        min: 0.0,
        max: 10.0,
        precision: 2,
    },
];

const GLOW_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        kind: SliderKind::GlowWeight,
        label: "Glow Weight",
        min: -1.0,
        max: 1.0,
        precision: 3,
    },
    SliderSpec {
        kind: SliderKind::GlowBias,
        label: "Glow Bias",
        min: -1.0,
        max: 1.0,
        precision: 3,
    },
    SliderSpec {
        kind: SliderKind::GlowEdge0,
        label: "Glow Edge0",
        min: -1.0,
        max: 1.0,
        precision: 3,
    },
    SliderSpec {
        kind: SliderKind::GlowEdge1,
        label: "Glow Edge1",
        min: -1.0,
        max: 1.0,
        precision: 3,
    },
];

impl LiquidGlassView {
    pub fn register(workspace: &mut Workspace, _window: &mut Window, _cx: &mut Context<Workspace>) {
        workspace.register_action(
            move |workspace, _: &workspace::NewLiquidGlass, window, cx| {
                let view = cx.new(|cx| Self::new(cx));
                workspace.add_item_to_active_pane(Box::new(view), None, true, window, cx);
            },
        );
    }

    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            backgrounds: load_backgrounds(cx),
            focus_handle: cx.focus_handle(),
            preview_bounds: Rc::new(RefCell::new(None)),
            slider_bounds: Rc::new(RefCell::new(Vec::new())),
            active_slider: None,
            state: UiState::default(),
            use_preview_center: true,
        }
    }

    fn set_background(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.backgrounds.len() {
            self.state.current_bg = index;
            cx.notify();
        }
    }

    fn set_glass_variant(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < GLASS_VARIANTS.len() {
            self.state.glass_variant = index;
            self.state.apply_variant_size();
            cx.notify();
        }
    }

    fn set_preview_position_from_window(
        &mut self,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_bounds) = self.preview_bounds.borrow().clone() else {
            return;
        };

        let x = (position.x - preview_bounds.origin.x)
            .as_f32()
            .clamp(0.0, preview_bounds.size.width.as_f32());
        let y = (position.y - preview_bounds.origin.y)
            .as_f32()
            .clamp(0.0, preview_bounds.size.height.as_f32());
        self.state.position = [x, y];
        self.use_preview_center = false;
        cx.notify();
    }

    fn set_slider_value(&mut self, kind: SliderKind, value: f32, cx: &mut Context<Self>) {
        match kind {
            SliderKind::Power => self.state.power_factor = value.clamp(1.001, 6.0),
            SliderKind::Width => self.state.width = value.clamp(0.0, 10.0),
            SliderKind::Height => self.state.height = value.clamp(0.0, 10.0),
            SliderKind::ChromaticAberration => {
                self.state.chromatic_aberration = value.clamp(0.0, 0.02)
            }
            SliderKind::AberrationSamples => {
                self.state.aberration_samples = value.round().clamp(1.0, 8.0) as u32
            }
            SliderKind::BlurIterations => {
                self.state.blur_iterations = value.round().clamp(0.0, 10.0) as u32
            }
            SliderKind::BlurRadius => self.state.blur_radius = value.clamp(0.0, 10.0),
            SliderKind::BlurDownscale => self.state.blur_downscale = value.clamp(0.1, 1.0),
            SliderKind::Noise => self.state.noise = value.clamp(0.0, 0.3),
            SliderKind::FPower => self.state.f_power = value.clamp(-1.5, 6.0),
            SliderKind::A => self.state.a = value.clamp(0.0, 5.0),
            SliderKind::B => self.state.b = value.clamp(0.0, 6.0),
            SliderKind::C => self.state.c = value.clamp(0.0, 6.0),
            SliderKind::D => self.state.d = value.clamp(0.0, 10.0),
            SliderKind::GlowWeight => self.state.glow_weight = value.clamp(-1.0, 1.0),
            SliderKind::GlowBias => self.state.glow_bias = value.clamp(-1.0, 1.0),
            SliderKind::GlowEdge0 => self.state.glow_edge0 = value.clamp(-1.0, 1.0),
            SliderKind::GlowEdge1 => self.state.glow_edge1 = value.clamp(-1.0, 1.0),
        }

        cx.notify();
    }

    fn slider_value(&self, kind: SliderKind) -> f32 {
        match kind {
            SliderKind::Power => self.state.power_factor,
            SliderKind::Width => self.state.width,
            SliderKind::Height => self.state.height,
            SliderKind::ChromaticAberration => self.state.chromatic_aberration,
            SliderKind::AberrationSamples => self.state.aberration_samples as f32,
            SliderKind::BlurIterations => self.state.blur_iterations as f32,
            SliderKind::BlurRadius => self.state.blur_radius,
            SliderKind::BlurDownscale => self.state.blur_downscale,
            SliderKind::Noise => self.state.noise,
            SliderKind::FPower => self.state.f_power,
            SliderKind::A => self.state.a,
            SliderKind::B => self.state.b,
            SliderKind::C => self.state.c,
            SliderKind::D => self.state.d,
            SliderKind::GlowWeight => self.state.glow_weight,
            SliderKind::GlowBias => self.state.glow_bias,
            SliderKind::GlowEdge0 => self.state.glow_edge0,
            SliderKind::GlowEdge1 => self.state.glow_edge1,
        }
    }

    fn slider_value_from_position(&self, kind: SliderKind, position: Point<Pixels>) -> Option<f32> {
        let bounds = self
            .slider_bounds
            .borrow()
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map(|(_, bounds)| bounds.clone())?;
        let normalized = ((position.x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0);
        let spec = kind.spec();
        Some(spec.min + (spec.max - spec.min) * normalized)
    }

    fn begin_slider_drag(
        &mut self,
        kind: SliderKind,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(value) = self.slider_value_from_position(kind, position) {
            self.active_slider = Some(kind);
            self.set_slider_value(kind, value, cx);
            window.focus(&self.focus_handle, cx);
        }
    }

    fn on_control_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(active_slider) = self.active_slider
            && let Some(value) = self.slider_value_from_position(active_slider, event.position)
        {
            self.set_slider_value(active_slider, value, cx);
        }
    }

    fn end_slider_drag(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.active_slider = None;
    }

    fn on_preview_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.state.mouse_control {
            return;
        }
        self.set_preview_position_from_window(event.position, cx);
    }

    fn render_dropdowns(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();

        let background_menu = {
            let background_names = self
                .backgrounds
                .iter()
                .map(|background| background.name.clone())
                .collect::<Vec<_>>();
            let current_bg = self.state.current_bg;
            ContextMenu::build(window, cx, move |mut menu, _, _| {
                for (index, name) in background_names.iter().enumerate() {
                    let entity = entity.clone();
                    menu = menu.toggleable_entry(
                        name.clone(),
                        index == current_bg,
                        IconPosition::Start,
                        None,
                        move |window, cx| {
                            let _ = window;
                            entity.update(cx, |this, cx| this.set_background(index, cx));
                        },
                    );
                }
                menu
            })
        };

        let entity = cx.entity().clone();
        let glass_menu = {
            let current_variant = self.state.glass_variant;
            ContextMenu::build(window, cx, move |mut menu, _, _| {
                for (index, name) in GLASS_VARIANTS.iter().enumerate() {
                    let entity = entity.clone();
                    menu = menu.toggleable_entry(
                        (*name).to_string(),
                        index == current_variant,
                        IconPosition::Start,
                        None,
                        move |window, cx| {
                            let _ = window;
                            entity.update(cx, |this, cx| this.set_glass_variant(index, cx));
                        },
                    );
                }
                menu
            })
        };

        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new("Background")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        DropdownMenu::new(
                            "liquid-glass-background",
                            self.backgrounds[self.state.current_bg].name.clone(),
                            background_menu,
                        )
                        .style(DropdownStyle::Outlined)
                        .full_width(true),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new("Glass Style")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        DropdownMenu::new(
                            "liquid-glass-style",
                            GLASS_VARIANTS[self.state.glass_variant],
                            glass_menu,
                        )
                        .style(DropdownStyle::Outlined)
                        .full_width(true),
                    ),
            )
            .child(SwitchField::new(
                "liquid-glass-mouse-control",
                Some("Move with mouse"),
                Some("Use the preview cursor to position the glass element.".into()),
                if self.state.mouse_control {
                    ToggleState::Selected
                } else {
                    ToggleState::Unselected
                },
                {
                    let entity = cx.entity().clone();
                    move |state, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.state.mouse_control = matches!(state, ToggleState::Selected);
                            cx.notify();
                        });
                    }
                },
            ))
    }

    fn render_slider_group(
        &self,
        title: &'static str,
        specs: &[SliderSpec],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let sliders = specs
            .iter()
            .copied()
            .map(|spec| self.render_slider(spec, cx))
            .collect::<Vec<_>>();

        v_flex()
            .gap_2()
            .child(Label::new(title).size(LabelSize::Small).color(Color::Muted))
            .children(sliders)
    }

    fn render_slider(&self, spec: SliderSpec, cx: &mut Context<Self>) -> AnyElement {
        let slider_bounds = self.slider_bounds.clone();
        let normalized = (self.slider_value(spec.kind) - spec.min) / (spec.max - spec.min);
        let normalized = normalized.clamp(0.0, 1.0);
        let value_text = spec.format(self.slider_value(spec.kind));
        let colors = cx.theme().colors();
        let slider_surface = colors
            .surface_background
            .blend(colors.element_active.opacity(0.55));
        let slider_border = colors
            .border_variant
            .blend(colors.text_accent.opacity(0.18));
        let track_color = colors
            .border_variant
            .blend(colors.element_background.opacity(0.35));
        let progress_color = colors.text_accent;
        let progress_glow = colors.text_accent.opacity(0.22);
        let knob_color = colors.elevated_surface_background;
        let knob_border = colors.text_accent;

        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new(spec.label).size(LabelSize::Small))
                    .child(
                        Label::new(value_text)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .child(
                div()
                    .id(format!("liquid-glass-slider-{}", spec.kind.id()))
                    .relative()
                    .h(px(28.0))
                    .w_full()
                    .rounded_md()
                    .bg(slider_surface)
                    .border_1()
                    .border_color(slider_border)
                    .cursor_pointer()
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                slider_bounds.borrow_mut().push((spec.kind, bounds));
                            },
                            move |bounds, _, window, _cx| {
                                let track_bounds = Bounds::new(
                                    point(bounds.origin.x + px(8.0), bounds.origin.y + px(10.0)),
                                    size(bounds.size.width - px(16.0), px(8.0)),
                                );
                                let fill_bounds = Bounds::new(
                                    track_bounds.origin,
                                    size(
                                        track_bounds.size.width * normalized,
                                        track_bounds.size.height,
                                    ),
                                );
                                let knob_center_x =
                                    track_bounds.origin.x + track_bounds.size.width * normalized;
                                let knob_bounds = Bounds::new(
                                    point(knob_center_x - px(7.0), bounds.origin.y + px(4.0)),
                                    size(px(14.0), px(20.0)),
                                );

                                let mut track = fill(track_bounds, track_color);
                                track.corner_radii = px(999.0).into();
                                window.paint_quad(track);

                                let glow_bounds = Bounds::new(
                                    point(fill_bounds.origin.x, fill_bounds.origin.y - px(2.0)),
                                    size(fill_bounds.size.width, fill_bounds.size.height + px(4.0)),
                                );
                                let mut glow = fill(glow_bounds, progress_glow);
                                glow.corner_radii = px(999.0).into();
                                window.paint_quad(glow);

                                let mut progress = fill(fill_bounds, progress_color);
                                progress.corner_radii = px(999.0).into();
                                window.paint_quad(progress);

                                let mut knob = fill(knob_bounds, knob_color);
                                knob.corner_radii = px(999.0).into();
                                knob.border_widths = px(1.0).into();
                                knob.border_color = knob_border;
                                window.paint_quad(knob);
                            },
                        )
                        .size_full(),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                            this.begin_slider_drag(spec.kind, event.position, window, cx);
                        }),
                    ),
            )
            .into_any_element()
    }

    fn render_preview(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let preview_bounds = self.preview_bounds.clone();
        let background = self.backgrounds[self.state.current_bg].clone();
        let state = self.state.clone();
        let use_preview_center = self.use_preview_center;

        div()
            .id("liquid-glass-preview")
            .relative()
            .flex_1()
            .min_h(px(420.0))
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().colors().border)
            .overflow_hidden()
            .bg(cx.theme().colors().editor_background)
            .child(
                canvas(
                    move |bounds, _, _| {
                        *preview_bounds.borrow_mut() = Some(bounds);
                        bounds
                    },
                    move |bounds, _, window, _cx| {
                        let center = if use_preview_center {
                            bounds.center()
                        } else {
                            point(
                                bounds.origin.x + px(state.position[0]),
                                bounds.origin.y + px(state.position[1]),
                            )
                        };
                        let glass_size =
                            size(px(state.glass_width_px()), px(state.glass_height_px()));
                        let glass_bounds = Bounds::new(
                            point(
                                center.x - glass_size.width.half(),
                                center.y - glass_size.height.half(),
                            ),
                            glass_size,
                        );

                        let _ = window.paint_liquid_glass(
                            bounds,
                            background.image,
                            gpui::LiquidGlassParams {
                                glass_bounds,
                                power_factor: state.power_factor,
                                a: state.a,
                                b: state.b,
                                c: state.c,
                                d: state.d,
                                f_power: state.f_power,
                                noise: state.noise,
                                glow_weight: state.glow_weight,
                                glow_edge0: state.glow_edge0,
                                glow_edge1: state.glow_edge1,
                                glow_bias: state.glow_bias,
                                chromatic_aberration: state.chromatic_aberration,
                                aberration_samples: state.aberration_samples,
                                blur_radius: state.blur_radius,
                                blur_iterations: state.blur_iterations,
                                blur_downscale: state.blur_downscale,
                            },
                            0,
                        );
                    },
                )
                .size_full(),
            )
            .on_mouse_move(cx.listener(Self::on_preview_mouse_move))
    }
}

impl SliderKind {
    fn id(self) -> &'static str {
        match self {
            SliderKind::Power => "power",
            SliderKind::Width => "width",
            SliderKind::Height => "height",
            SliderKind::ChromaticAberration => "chromatic-aberration",
            SliderKind::AberrationSamples => "aberration-samples",
            SliderKind::BlurIterations => "blur-iterations",
            SliderKind::BlurRadius => "blur-radius",
            SliderKind::BlurDownscale => "blur-downscale",
            SliderKind::Noise => "noise",
            SliderKind::FPower => "f-power",
            SliderKind::A => "param-a",
            SliderKind::B => "param-b",
            SliderKind::C => "param-c",
            SliderKind::D => "param-d",
            SliderKind::GlowWeight => "glow-weight",
            SliderKind::GlowBias => "glow-bias",
            SliderKind::GlowEdge0 => "glow-edge0",
            SliderKind::GlowEdge1 => "glow-edge1",
        }
    }

    fn spec(self) -> SliderSpec {
        SHAPE_SLIDERS
            .iter()
            .chain(ABERRATION_SLIDERS)
            .chain(BLUR_SLIDERS)
            .chain(REFRACTION_SLIDERS)
            .chain(GLOW_SLIDERS)
            .copied()
            .find(|spec| spec.kind == self)
            .expect("all slider kinds have specs")
    }
}

impl SliderSpec {
    fn format(self, value: f32) -> SharedString {
        if self.precision == 0 {
            format!("{value:.0}").into()
        } else {
            format!("{value:.precision$}", precision = self.precision).into()
        }
    }
}

impl EventEmitter<ItemEvent> for LiquidGlassView {}

impl Focusable for LiquidGlassView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for LiquidGlassView {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "Liquid Glass".into()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<ui::Icon> {
        Some(ui::Icon::new(IconName::Image))
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Liquid Glass Opened")
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn can_split(&self) -> bool {
        true
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        let backgrounds = self.backgrounds.clone();
        let preview_bounds = Rc::new(RefCell::new(None));
        let slider_bounds = Rc::new(RefCell::new(Vec::new()));
        let state = self.state.clone();
        let use_preview_center = self.use_preview_center;

        Task::ready(Some(cx.new(|cx| Self {
            backgrounds,
            focus_handle: cx.focus_handle(),
            preview_bounds,
            slider_bounds,
            active_slider: None,
            state,
            use_preview_center,
        })))
    }
}

impl SerializableItem for LiquidGlassView {
    fn serialized_item_kind() -> &'static str {
        "LiquidGlass"
    }

    fn cleanup(
        _workspace_id: WorkspaceId,
        _alive_items: Vec<ItemId>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn deserialize(
        _project: Entity<Project>,
        _workspace: WeakEntity<Workspace>,
        _workspace_id: WorkspaceId,
        _item_id: ItemId,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Entity<Self>>> {
        Task::ready(Ok(cx.new(Self::new)))
    }

    fn serialize(
        &mut self,
        _workspace: &mut Workspace,
        _item_id: ItemId,
        _closing: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Task<Result<()>>> {
        None
    }

    fn should_serialize(&self, _event: &Self::Event) -> bool {
        false
    }
}

impl Render for LiquidGlassView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.slider_bounds.borrow_mut().clear();

        let background_credit = self.backgrounds[self.state.current_bg].credit.clone();

        h_flex()
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .on_mouse_move(cx.listener(Self::on_control_mouse_move))
            .capture_any_mouse_up(cx.listener(Self::end_slider_drag))
            .child(
                v_flex()
                    .id("liquid-glass-controls")
                    .w(px(360.0))
                    .h_full()
                    .p_4()
                    .gap_4()
                    .border_r_1()
                    .border_color(cx.theme().colors().border)
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Headline::new("Liquid Glass").size(HeadlineSize::Small))
                            .child(
                                Label::new(
                                    "Native GPUI GPU element with platform renderer support and real editor controls.",
                                )
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                            ),
                    )
                    .child(self.render_dropdowns(window, cx))
                    .child(Divider::horizontal())
                    .child(self.render_slider_group("Shape", SHAPE_SLIDERS, cx))
                    .child(Divider::horizontal())
                    .child(self.render_slider_group(
                        "Chromatic Aberration",
                        ABERRATION_SLIDERS,
                        cx,
                    ))
                    .child(Divider::horizontal())
                    .child(self.render_slider_group("Blur & Noise", BLUR_SLIDERS, cx))
                    .child(Divider::horizontal())
                    .child(self.render_slider_group("Refraction", REFRACTION_SLIDERS, cx))
                    .child(Divider::horizontal())
                    .child(self.render_slider_group("Glow", GLOW_SLIDERS, cx))
                    .children(background_credit.into_iter().map(|credit| {
                        Label::new(credit)
                            .size(LabelSize::Small)
                            .color(Color::Muted)
                            .into_any_element()
                    })),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .p_4()
                    .child(self.render_preview(cx)),
            )
    }
}
