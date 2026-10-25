//! Single source of truth for every colour, size and spacing token in the launcher.
//!
//! The screens never hard-code a colour or a pixel value; they ask this module.
//! That is what keeps every control on the same scale and makes the look
//! changeable in one place.

use eframe::egui::{self, Color32, FontId, Stroke, TextStyle};

// ---------------------------------------------------------------------------
// Spacing scale
// ---------------------------------------------------------------------------

/// 4 px — hairline gaps between a label and its value.
pub const SPACE_XS: f32 = 4.;
/// 8 px — default gap between related controls.
pub const SPACE_S: f32 = 8.;
/// 14 px — gap between groups inside a card.
pub const SPACE_M: f32 = 14.;
/// 22 px — gap between cards and sections.
pub const SPACE_L: f32 = 22.;
/// 30 px — page margin.
pub const SPACE_XL: f32 = 30.;

// ---------------------------------------------------------------------------
// Geometry — the "one size" contract for controls
// ---------------------------------------------------------------------------

/// Every button and text field in the app is exactly this tall.
pub const CONTROL_H: f32 = 34.;
/// Primary action buttons (Start / Stop) are taller so the eye lands on them.
pub const CONTROL_H_PRIMARY: f32 = 40.;
/// Corner radius shared by cards and controls.
pub const RADIUS: f32 = 9.;
/// Corner radius for small chips and inline tags.
pub const RADIUS_S: f32 = 6.;
/// Card inner padding.
pub const CARD_PAD: f32 = 16.;
/// Standard button width so a stack of buttons lines up.
pub const BUTTON_W: f32 = 150.;

// ---------------------------------------------------------------------------
// Type scale
// ---------------------------------------------------------------------------

pub const FONT_TITLE: f32 = 26.;
pub const FONT_HEADING: f32 = 19.;
pub const FONT_BODY: f32 = 15.;
pub const FONT_SMALL: f32 = 13.;
pub const LINE_BODY: f32 = 21.;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// Window background.
pub const BG: Color32 = Color32::from_rgb(13, 18, 27);
/// Cards and panels sitting on the background.
pub const PANEL: Color32 = Color32::from_rgb(21, 29, 42);
/// Slightly raised panel, used for the active tab and inputs.
pub const PANEL_RAISED: Color32 = Color32::from_rgb(28, 39, 55);
/// Sunken background for text inputs and the console view.
pub const SUNKEN: Color32 = Color32::from_rgb(9, 13, 20);
/// Hairline border around cards.
pub const BORDER: Color32 = Color32::from_rgb(45, 60, 80);
/// Stronger border for the focused or active element.
pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(79, 140, 168);

/// Primary accent — headings, active tab, links.
pub const ACCENT: Color32 = Color32::from_rgb(126, 205, 255);
/// Secondary accent for the "running" state.
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(140, 233, 199);

/// Main body text.
pub const TEXT: Color32 = Color32::from_rgb(233, 239, 247);
/// Secondary text, hints and captions.
pub const TEXT_DIM: Color32 = Color32::from_rgb(158, 174, 196);
/// Muted text for the least important notes.
pub const TEXT_FAINT: Color32 = Color32::from_rgb(120, 136, 158);

// ---------------------------------------------------------------------------
// Semantic status colours
// ---------------------------------------------------------------------------

pub const OK: Color32 = Color32::from_rgb(120, 224, 172);
pub const WARN: Color32 = Color32::from_rgb(245, 201, 112);
pub const DANGER: Color32 = Color32::from_rgb(246, 133, 143);
pub const INFO: Color32 = ACCENT;

// ---------------------------------------------------------------------------
// Widget colours
// ---------------------------------------------------------------------------

pub const BUTTON_BG: Color32 = Color32::from_rgb(38, 52, 72);
pub const BUTTON_BG_HOVER: Color32 = Color32::from_rgb(50, 70, 95);
pub const BUTTON_BG_ACTIVE: Color32 = Color32::from_rgb(57, 84, 112);

pub const PRIMARY_BG: Color32 = Color32::from_rgb(30, 104, 82);
pub const PRIMARY_BG_HOVER: Color32 = Color32::from_rgb(36, 126, 99);
pub const OPEN_BG: Color32 = Color32::from_rgb(32, 92, 142);
pub const OPEN_BG_HOVER: Color32 = Color32::from_rgb(38, 112, 172);
pub const STOP_BG: Color32 = Color32::from_rgb(120, 48, 60);
pub const STOP_BG_HOVER: Color32 = Color32::from_rgb(146, 58, 72);

/// Status pill background.
pub const PILL_BG: Color32 = Color32::from_rgb(17, 24, 35);

// ---------------------------------------------------------------------------
// Status level
// ---------------------------------------------------------------------------

/// How serious the current status line is. Drives its colour and icon.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Idle,
    Busy,
    Ok,
    Warn,
    Error,
}

impl Level {
    pub fn color(self) -> Color32 {
        match self {
            Level::Idle => TEXT_DIM,
            Level::Busy => WARN,
            Level::Ok => OK,
            Level::Warn => WARN,
            Level::Error => DANGER,
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Level::Idle => "·",
            Level::Busy => "…",
            Level::Ok => "✔",
            Level::Warn => "!",
            Level::Error => "✖",
        }
    }

    pub fn chip(self) -> Color32 {
        self.color()
    }
}

// ---------------------------------------------------------------------------
// Style installation
// ---------------------------------------------------------------------------

/// Apply the theme to a context. Called once at startup.
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(FONT_HEADING));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(FONT_BODY));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(FONT_BODY));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(FONT_SMALL));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(FONT_SMALL));

    style.spacing.item_spacing = egui::vec2(SPACE_S, SPACE_S);
    style.spacing.button_padding = egui::vec2(14., 8.);
    style.spacing.interact_size.y = CONTROL_H;
    style.spacing.scroll.bar_width = 9.;
    style.spacing.scroll.floating = false;

    let visuals = &mut style.visuals;
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.window_stroke = Stroke::new(1_f32, BORDER);
    visuals.menu_corner_radius = RADIUS.into();
    visuals.extreme_bg_color = SUNKEN;
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = Color32::from_rgb(28, 96, 118);
    visuals.selection.stroke = Stroke::new(1_f32, ACCENT);

    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1_f32, TEXT_DIM);

    visuals.widgets.inactive.bg_fill = BUTTON_BG;
    visuals.widgets.inactive.weak_bg_fill = BUTTON_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1_f32, TEXT_DIM);

    visuals.widgets.hovered.bg_fill = BUTTON_BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BUTTON_BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1_f32, BORDER_ACTIVE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT);

    visuals.widgets.active.bg_fill = BUTTON_BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BUTTON_BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1_f32, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT);

    visuals.widgets.open.bg_fill = BUTTON_BG_ACTIVE;
    visuals.widgets.open.weak_bg_fill = BUTTON_BG_ACTIVE;
    visuals.widgets.open.bg_stroke = Stroke::new(1_f32, BORDER_ACTIVE);

    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// Reusable drawing helpers
// ---------------------------------------------------------------------------

/// A card: raised panel with a hairline border and consistent padding.
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1_f32, BORDER))
        .corner_radius(RADIUS)
        .inner_margin(CARD_PAD)
}

/// A sunken frame, used for the console and read-only output.
pub fn sunken() -> egui::Frame {
    egui::Frame::new()
        .fill(SUNKEN)
        .stroke(Stroke::new(1_f32, BORDER))
        .corner_radius(RADIUS_S)
        .inner_margin(SPACE_S)
}

/// Full-width card that fills the available width.
pub fn wide_card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    card().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add(ui);
    });
}

/// Section heading used inside a tab.
pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(SPACE_S);
    ui.label(
        egui::RichText::new(title)
            .size(FONT_HEADING)
            .strong()
            .color(ACCENT),
    );
    ui.add_space(SPACE_XS);
}

/// A small caption under a control.
pub fn hint(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(
        egui::RichText::new(text.into())
            .size(FONT_SMALL)
            .color(TEXT_FAINT),
    );
}

/// A warning or notice line in a semantic colour.
pub fn note(ui: &mut egui::Ui, level: Level, text: impl Into<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = SPACE_XS;
        ui.label(
            egui::RichText::new(level.glyph())
                .size(FONT_SMALL)
                .strong()
                .color(level.color()),
        );
        ui.label(
            egui::RichText::new(text.into())
                .size(FONT_SMALL)
                .color(level.color()),
        );
    });
}

/// A labelled row: fixed-width label, then the control, all on the control scale.
pub fn field(ui: &mut egui::Ui, label: &str, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [width, CONTROL_H],
            egui::Label::new(egui::RichText::new(label).size(FONT_BODY).color(TEXT_DIM))
                .halign(egui::Align::LEFT),
        );
        add(ui);
    });
}

/// Standard secondary button, always CONTROL_H tall.
pub fn button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(
        [BUTTON_W, CONTROL_H],
        egui::Button::new(egui::RichText::new(label).size(FONT_BODY)),
    )
}

/// Standard button at an explicit width.
pub fn button_w(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    ui.add_sized(
        [width, CONTROL_H],
        egui::Button::new(egui::RichText::new(label).size(FONT_BODY)),
    )
}

/// Filled action button (Start / Open / Stop), CONTROL_H_PRIMARY tall.
pub fn action(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    bg: Color32,
    hover: Color32,
) -> egui::Response {
    let button = egui::Button::new(
        egui::RichText::new(label)
            .size(FONT_BODY)
            .strong()
            .color(TEXT),
    )
    .fill(bg)
    .stroke(Stroke::new(1_f32, bg));

    let response = ui.add_sized([width, CONTROL_H_PRIMARY], button);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter()
            .rect_filled(response.rect, RADIUS, hover.gamma_multiply(0.35));
    }
    response
}

/// A status pill: glyph and text on a sunken background.
pub fn status_pill(ui: &mut egui::Ui, level: Level, text: &str) {
    egui::Frame::new()
        .fill(PILL_BG)
        .stroke(Stroke::new(1_f32, level.color().gamma_multiply(0.45)))
        .corner_radius(RADIUS_S)
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = SPACE_S;
                ui.label(
                    egui::RichText::new(level.glyph())
                        .size(FONT_BODY)
                        .strong()
                        .color(level.color()),
                );
                ui.label(
                    egui::RichText::new(text)
                        .size(FONT_BODY)
                        .color(level.color()),
                );
            });
        });
}

/// Read-only selectable value with a monospace font, e.g. a share URL.
pub fn value_row(ui: &mut egui::Ui, label: &str, value: &str, copy: &mut Option<String>) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [CONTROL_H, CONTROL_H],
            egui::Label::new(egui::RichText::new(label).size(FONT_SMALL).color(TEXT_DIM)),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(value)
                    .size(FONT_SMALL)
                    .monospace()
                    .color(TEXT),
            )
            .selectable(true)
            .wrap_mode(egui::TextWrapMode::Truncate),
        );
        if ui
            .add_sized(
                [70., CONTROL_H],
                egui::Button::new(egui::RichText::new("Copy").size(FONT_SMALL)),
            )
            .clicked()
        {
            *copy = Some(value.to_owned());
        }
    });
}

/// Tab bar. Returns the newly selected index when the user changes tab.
pub fn tabs(ui: &mut egui::Ui, labels: &[&str], selected: usize) -> usize {
    let mut chosen = selected;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACE_XS;
        for (index, label) in labels.iter().enumerate() {
            let active = index == selected;
            let (fill, text) = if active {
                (PANEL_RAISED, ACCENT)
            } else {
                (Color32::TRANSPARENT, TEXT_DIM)
            };
            let button = egui::Button::new(
                egui::RichText::new(*label)
                    .size(FONT_BODY)
                    .strong()
                    .color(text),
            )
            .fill(fill)
            .stroke(Stroke::new(
                1_f32,
                if active {
                    BORDER_ACTIVE
                } else {
                    Color32::TRANSPARENT
                },
            ));
            if ui.add_sized([128., CONTROL_H], button).clicked() {
                chosen = index;
            }
        }
    });
    chosen
}

/// Severity-coloured validity row used by the folder validation.
pub fn validity_row(ui: &mut egui::Ui, ok: bool, message: &str) {
    let level = if ok { Level::Ok } else { Level::Warn };
    note(ui, level, message);
}
