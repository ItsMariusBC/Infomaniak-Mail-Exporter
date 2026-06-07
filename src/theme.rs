//! Palette et style de la GUI, inspirés de l'identité kSuite / Infomaniak Mail.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use std::sync::Arc;

/// Nom de la famille semi-grasse (titres, boutons, libellés).
const SEMIBOLD: &str = "semibold";

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

/// Police d'un titre/élément semi-gras (famille Inter SemiBold).
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Installe la police Inter (lisible, OFL) comme police par défaut + une
/// famille semi-grasse pour les titres. À appeler avant [`apply`].
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    // Inter en tête de la famille proportionnelle (corps de texte).
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".to_owned());
    // Famille semi-grasse dédiée (titres, boutons).
    fonts.families.insert(
        FontFamily::Name(SEMIBOLD.into()),
        vec!["Inter-SemiBold".to_owned(), "Inter".to_owned()],
    );
    ctx.set_fonts(fonts);
}

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
    style.spacing.item_spacing = egui::vec2(10.0, 9.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.spacing.interact_size.y = 30.0; // hauteur des champs/boutons

    // Scrollbar flottant : il ne réserve pas de largeur → marges G/D symétriques.
    style.spacing.scroll.floating = true;

    // Hiérarchie typographique cohérente (Inter).
    style.text_styles = [
        (TextStyle::Heading, semibold(22.0)),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Button, semibold(14.5)),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
    ]
    .into();

    ctx.set_style(style);
}
