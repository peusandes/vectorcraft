//! White-label mode: the web app framed by a host page (`?embed=1`, see `docs/embed-bridge.md`)
//! shows itself as the host's product, the Prolytiq Figure Editor. The shell turns it on with
//! [`enable`] before the first frame; it is off everywhere else (desktop, the normal web app,
//! tests), which keeps the default branding.
//!
//! While on: every interface string ([`crate::i18n`]) has VectorCraft / ArtCraft renamed
//! ([`rebrand`]), the community links (Discord, website, app page, GitHub) are gone ([`hides`]),
//! the brand mark is drawn in code ([`paint_mark`]), the Home screen has the host's tagline
//! ([`tagline`]), and the default theme is Medium Light on a navy canvas ([`brightness`],
//! [`pasteboard`]), the Home screen included ([`home_tokens`]); an explicit preference still wins.
//!
//! The switch is per thread: the web app has one, and tests that turn it on don't reach the other
//! tests running beside them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use egui::{Color32, CornerRadius, Rect, Ui};

/// The product name shown instead of VectorCraft.
pub const NAME: &str = "Prolytiq Figure Editor";

/// The canvas around the artboards: Prolytiq navy.
pub const NAVY: Color32 = Color32::from_rgb(0x0b, 0x1e, 0x3a);

/// The default app name.
const DEFAULT_NAME: &str = "VectorCraft";

/// Renamed in white-label mode, longest first so "VectorCraft" isn't half-renamed by "Craft"
/// rules (there are none, but order still matters between overlapping names).
const RENAMES: [(&str, &str); 4] = [("VectorCraft", NAME), ("Vector Craft", NAME), ("Crafting Apps", "Prolytiq"), ("ArtCraft", "Prolytiq")];

/// Commands white-label mode leaves out: the community links point at VectorCraft's own pages.
const HIDDEN: [&str; 4] = ["help.discord", "help.website", "help.appPage", "help.github"];

/// How many distinct renamed strings are kept: interface strings are a fixed set, so this is only
/// reached if something renames data; past it strings are renamed afresh each time.
const MAX_INTERNED: usize = 4096;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static INTERNED: RefCell<HashMap<String, &'static str>> = RefCell::new(HashMap::new());
}

/// Turn white-label mode on for this thread (the web shell, in embed mode).
pub fn enable() {
    ON.with(|on| on.set(true));
}

/// Turn it off again (tests).
#[cfg(test)]
pub fn disable() {
    ON.with(|on| on.set(false));
}

/// Is white-label mode on?
pub fn on() -> bool {
    ON.with(Cell::get)
}

/// The app's name as shown.
pub fn app_name() -> &'static str {
    if on() { NAME } else { DEFAULT_NAME }
}

/// Does white-label mode leave command `id` out of menus, the palette and the Home screen?
pub fn hides(id: &str) -> bool {
    on() && HIDDEN.contains(&id)
}

fn renamed(s: &str) -> Option<String> {
    if !RENAMES.iter().any(|(from, _)| s.contains(from)) {
        return None;
    }
    Some(RENAMES.iter().fold(s.to_string(), |acc, (from, to)| acc.replace(from, to)))
}

/// `s` as shown: unchanged unless white-label mode is on and it names VectorCraft or ArtCraft.
pub fn rebrand(s: &str) -> std::borrow::Cow<'_, str> {
    match if on() { renamed(s) } else { None } {
        Some(r) => std::borrow::Cow::Owned(r),
        None => std::borrow::Cow::Borrowed(s),
    }
}

/// [`rebrand`] for the translation functions, which hand out borrowed strings: a renamed string
/// is kept for the rest of the run (interface strings are a fixed set, [`MAX_INTERNED`] caps it).
pub fn rebrand_str(s: &str) -> &str {
    if !on() {
        return s;
    }
    let Some(r) = renamed(s) else { return s };
    INTERNED.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(&kept) = m.get(s) {
            return kept;
        }
        if m.len() >= MAX_INTERNED {
            // Full: show the original rather than grow without bound.
            return s;
        }
        let kept: &'static str = Box::leak(r.into_boxed_str());
        m.insert(s.to_string(), kept);
        kept
    })
}

/// The default interface brightness when the preference is at its default (`mediumDark`).
pub fn brightness(pref: crate::theme::Brightness) -> crate::theme::Brightness {
    if on() && pref == crate::theme::Brightness::MediumDark { crate::theme::Brightness::MediumLight } else { pref }
}

/// The canvas colour when Canvas Color matches the interface (`None`): navy in white-label mode.
pub fn pasteboard(chosen: Option<Color32>) -> Option<Color32> {
    chosen.or_else(|| on().then_some(NAVY))
}

/// The Home screen's tagline in white-label mode (English: the host's product line).
pub const TAGLINE: &str = "Scientific figures, publication-ready.";

/// The Home screen's tagline: [`TAGLINE`] in white-label mode, else `default`.
pub fn tagline(default: &str) -> &str {
    if on() { TAGLINE } else { default }
}

/// The tokens the Home screen and the empty window are drawn with. In white-label mode they sit
/// on the canvas colour (`pasteboard`: navy, or Canvas Color when set) instead of `panel_darker`,
/// with light text when that colour is dark; otherwise the tokens are unchanged.
pub fn home_tokens(ctx: &egui::Context) -> crate::theme::Tokens {
    let mut t = crate::theme::Tokens::get(ctx);
    if !on() {
        return t;
    }
    t.panel_darker = t.pasteboard;
    let [r, g, b, _] = t.pasteboard.to_array();
    let luma = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
    if luma < 128 && !t.dark {
        let ink = crate::theme::Tokens::for_brightness(crate::theme::Brightness::Dark);
        (t.text, t.text_dim, t.text_strong, t.icon) = (ink.text, ink.text_dim, ink.text_strong, ink.icon);
    }
    t
}

/// The white-label brand mark, drawn in code (no image asset): a navy rounded tile with a white
/// "P" and an accent bar.
pub fn paint_mark(ui: &Ui, r: Rect) {
    let p = ui.painter();
    let side = r.width().min(r.height());
    p.rect_filled(r, CornerRadius::same((side * 0.22).clamp(0.0, 255.0) as u8), NAVY);
    let bar = Rect::from_min_max(r.left_bottom() + egui::vec2(side * 0.2, -side * 0.24), r.right_bottom() - egui::vec2(side * 0.2, side * 0.16));
    p.rect_filled(bar, CornerRadius::ZERO, Color32::from_rgb(0x14, 0x73, 0xe6));
    p.text(r.center() - egui::vec2(0.0, side * 0.06), egui::Align2::CENTER_CENTER, "P", crate::theme::semibold(side * 0.6), Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_by_default_and_renames_only_when_on() {
        assert!(!on());
        assert_eq!(rebrand_str("About VectorCraft"), "About VectorCraft");
        assert_eq!(app_name(), "VectorCraft");
        assert!(!hides("help.discord"));
        enable();
        assert_eq!(rebrand_str("About VectorCraft"), "About Prolytiq Figure Editor");
        assert_eq!(rebrand("Part of ArtCraft."), "Part of Prolytiq.");
        assert_eq!(rebrand_str("New…"), "New…");
        assert_eq!(app_name(), NAME);
        assert!(hides("help.discord") && !hides("help.about"));
        // Interned: the same string comes back.
        assert!(std::ptr::eq(rebrand_str("Quit VectorCraft"), rebrand_str("Quit VectorCraft")));
        disable();
    }

    #[test]
    fn theme_defaults_yield_to_explicit_preferences() {
        use crate::theme::Brightness;
        enable();
        assert_eq!(brightness(Brightness::MediumDark), Brightness::MediumLight);
        assert_eq!(brightness(Brightness::Dark), Brightness::Dark);
        assert_eq!(pasteboard(None), Some(NAVY));
        assert_eq!(pasteboard(Some(Color32::WHITE)), Some(Color32::WHITE));
        disable();
        assert_eq!(brightness(Brightness::MediumDark), Brightness::MediumDark);
        assert_eq!(pasteboard(None), None);
    }
}
