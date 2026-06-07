//! Palette et style de la GUI, inspirés de l'identité kSuite / Infomaniak Mail.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Rose primaire Infomaniak Mail.
pub const PINK: Color32 = Color32::from_rgb(0xFF, 0x5B, 0x97);
/// Rose foncé (hover / accents).
pub const PINK_DARK: Color32 = Color32::from_rgb(0xF2, 0x35, 0x7A);
/// Rose clair.
pub const PINK_LIGHT: Color32 = Color32::from_rgb(0xFF, 0x9A, 0xBF);
/// Fond des cartes.
pub const CARD: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Fond général doux.
pub const BG_SOFT: Color32 = Color32::from_rgb(0xF7, 0xF7, 0xFA);
/// Texte principal.
pub const TEXT: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x33);
/// Texte secondaire / atténué.
pub const MUTED: Color32 = Color32::from_rgb(0x9A, 0x9A, 0xA8);

/// Applique le thème clair Infomaniak à un contexte egui.
pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.dark_mode = false;
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG_SOFT;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(0xF0, 0xF0, 0xF4); // fond des champs de saisie
    v.faint_bg_color = BG_SOFT;
    v.hyperlink_color = PINK_DARK;

    // Sélection rose.
    v.selection.bg_fill = PINK.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, PINK_DARK);

    // Coins arrondis homogènes.
    let r = CornerRadius::same(8);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;

    // Widgets neutres clairs (les boutons d'accent sont colorés au cas par cas).
    v.widgets.inactive.bg_fill = Color32::from_rgb(0xEC, 0xEC, 0xF1);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0xEC, 0xEC, 0xF1);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0xE2, 0xE2, 0xEA);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xE2, 0xE2, 0xEA);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, PINK_LIGHT);

    // Espacements aérés.
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);

    ctx.set_style(style);
}
