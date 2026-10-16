//! White-label mode (the embedded web app, `white_label`): nothing shown names VectorCraft,
//! ArtCraft or Discord, and the theme defaults to Medium Light on navy.

use serde_json::json;
use vectorcraft_engine::Session;

use crate::menus::{Item, menu_tree_named};
use crate::theme::{Brightness, Tokens};
use crate::{VectorcraftApp, white_label};

const BRANDS: [&str; 5] = ["VectorCraft", "Vector Craft", "ArtCraft", "Crafting Apps", "Discord"];

fn branded(s: &str) -> bool {
    BRANDS.iter().any(|b| s.contains(b))
}

/// Turns white-label mode off again when the test ends, passed or not.
struct WhiteLabel;
impl WhiteLabel {
    fn on() -> Self {
        white_label::enable();
        WhiteLabel
    }
}
impl Drop for WhiteLabel {
    fn drop(&mut self) {
        white_label::disable();
    }
}

fn texts(s: &egui::Shape, out: &mut Vec<String>) {
    match s {
        egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
        egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// The texts of a few whole-window frames.
fn frame_texts(app: &mut VectorcraftApp, ctx: &egui::Context) -> Vec<String> {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let mut shown = vec![];
    for _ in 0..4 {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
        });
        out.textures_delta.clear();
        shown.clear();
        out.shapes.iter().for_each(|c| texts(&c.shape, &mut shown));
    }
    shown
}

fn new_app() -> (VectorcraftApp, egui::Context) {
    let app = VectorcraftApp::new(Session::new(), Default::default());
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx);
    (app, ctx)
}

#[test]
fn the_home_screen_app_bar_and_about_show_only_the_prolytiq_name() {
    let _on = WhiteLabel::on();
    let (mut app, ctx) = new_app();
    app.custom_titlebar = true;
    let home = frame_texts(&mut app, &ctx);
    assert!(home.iter().any(|t| t == "Welcome to Prolytiq Figure Editor"), "{home:?}");
    assert!(!home.iter().any(|t| branded(t)), "{home:?}");
    assert!(!home.iter().any(|t| t == "Community"), "{home:?}");
    app.ui.about = true;
    let about = frame_texts(&mut app, &ctx);
    assert!(about.iter().any(|t| t == white_label::NAME), "{about:?}");
    assert!(!about.iter().any(|t| branded(t)), "{about:?}");
    assert_eq!(crate::chrome::window_title(&app), white_label::NAME);
    app.run("file.new", json!({"width": 100, "height": 100})).unwrap();
    assert!(crate::chrome::window_title(&app).ends_with("\u{2014} Prolytiq Figure Editor"));
    let doc = frame_texts(&mut app, &ctx);
    assert!(!doc.iter().any(|t| branded(t)), "{doc:?}");
}

#[test]
fn menus_leave_out_the_community_links_and_rename_the_rest_in_every_language() {
    let _on = WhiteLabel::on();
    fn walk(items: &[Item], out: &mut Vec<(&'static str, &'static str)>) {
        for item in items {
            match item {
                Item::Cmd(label, id, _) => out.push((label, id)),
                Item::Todo(label, _) | Item::Header(label) => out.push((label, "")),
                Item::Sub(label, sub) => {
                    out.push((label, ""));
                    walk(sub, out);
                }
                Item::Sep => {}
            }
        }
    }
    let tree = menu_tree_named(true);
    let mut entries = vec![];
    for (_, items) in &tree {
        walk(items, &mut entries);
        assert!(!matches!(items.first(), Some(Item::Sep)) && !matches!(items.last(), Some(Item::Sep)));
    }
    for id in ["help.discord", "help.website", "help.appPage", "help.github"] {
        assert!(!entries.iter().any(|(_, i)| *i == id), "{id}");
    }
    assert!(entries.iter().any(|(_, i)| *i == "help.about"));
    for lang in crate::i18n::Lang::all() {
        for (label, id) in &entries {
            let shown = crate::i18n::tr_id(lang, id, label);
            assert!(!branded(shown), "{}: {shown}", lang.code());
        }
        assert!(crate::i18n::tr(lang, "About VectorCraft").contains(white_label::NAME), "{}", lang.code());
    }
    // The links don't open from anywhere else either.
    use std::sync::{Arc, Mutex};
    let opened = Arc::new(Mutex::new(vec![]));
    let o = opened.clone();
    let services = crate::Services { open_url: Some(Box::new(move |u: &str| o.lock().unwrap().push(u.to_string()))), ..Default::default() };
    let mut app = VectorcraftApp::new(Session::new(), services);
    app.open_link("help.discord");
    assert!(opened.lock().unwrap().is_empty());
}

#[test]
fn the_theme_defaults_to_medium_light_on_navy_and_preferences_still_win() {
    let _on = WhiteLabel::on();
    let (mut app, ctx) = new_app();
    crate::prefs_dialog::apply_runtime(&mut app, &ctx);
    assert_eq!(app.ui.brightness, Brightness::MediumLight);
    assert_eq!(Tokens::get(&ctx).pasteboard, white_label::NAVY);
    app.run("prefs.set", json!({"key": "canvasColor", "value": "white"})).unwrap();
    crate::prefs_dialog::apply_runtime(&mut app, &ctx);
    assert_eq!(Tokens::get(&ctx).pasteboard, egui::Color32::WHITE);
    app.run("prefs.set", json!({"key": "uiBrightness", "value": "dark"})).unwrap();
    crate::prefs_dialog::apply_runtime(&mut app, &ctx);
    assert_eq!(app.ui.brightness, Brightness::Dark);
}

#[test]
fn off_the_default_branding_is_unchanged() {
    let (mut app, ctx) = new_app();
    assert_eq!(crate::chrome::window_title(&app), "VectorCraft");
    let home = frame_texts(&mut app, &ctx);
    assert!(home.iter().any(|t| t == "Welcome to VectorCraft"), "{home:?}");
    assert!(menu_tree_named(true).iter().any(|(_, items)| items.iter().any(|i| matches!(i, Item::Cmd(_, "help.discord", _)))));
    crate::prefs_dialog::apply_runtime(&mut app, &ctx);
    assert_eq!(app.ui.brightness, Brightness::MediumDark);
}

fn rects(s: &egui::Shape, out: &mut Vec<(egui::Rect, egui::Color32)>) {
    match s {
        egui::Shape::Rect(r) => out.push((r.rect, r.fill)),
        egui::Shape::Vec(v) => v.iter().for_each(|s| rects(s, out)),
        _ => {}
    }
}

/// The fills of the window-sized rectangles (40% of the window or more) in a whole-window frame:
/// the panels behind and the Home screen's (or the empty window's) background.
fn backgrounds(app: &mut VectorcraftApp, ctx: &egui::Context) -> Vec<egui::Color32> {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let mut found = vec![];
    for _ in 0..4 {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
        });
        out.textures_delta.clear();
        found.clear();
        out.shapes.iter().for_each(|c| rects(&c.shape, &mut found));
    }
    found.into_iter().filter(|(r, c)| c.a() > 0 && r.area() >= 0.4 * screen.area()).map(|(_, c)| c).collect()
}

#[test]
fn the_home_screen_sits_on_navy_with_the_prolytiq_tagline_until_canvas_color_is_set() {
    let _on = WhiteLabel::on();
    let (mut app, ctx) = new_app();
    assert!(backgrounds(&mut app, &ctx).contains(&white_label::NAVY));
    let home = frame_texts(&mut app, &ctx);
    assert!(home.iter().any(|t| t == white_label::TAGLINE), "{home:?}");
    assert!(!home.iter().any(|t| t.starts_with("Vector illustration")), "{home:?}");
    // Light text on the dark navy.
    assert!(crate::white_label::home_tokens(&ctx).text.r() > 0x90);
    // The empty window (Home off) too.
    app.session.prefs.show_home_screen = false;
    assert!(backgrounds(&mut app, &ctx).contains(&white_label::NAVY));
    app.session.prefs.show_home_screen = true;
    // An explicit Canvas Color wins.
    app.run("prefs.set", json!({"key": "canvasColor", "value": "white"})).unwrap();
    let shown = backgrounds(&mut app, &ctx);
    assert!(shown.contains(&egui::Color32::WHITE) && !shown.contains(&white_label::NAVY), "{shown:?}");
}

#[test]
fn off_the_home_screen_keeps_its_grey_and_tagline() {
    let (mut app, ctx) = new_app();
    crate::prefs_dialog::apply_runtime(&mut app, &ctx);
    let grey = Tokens::get(&ctx).panel_darker;
    assert_ne!(grey, white_label::NAVY);
    let shown = backgrounds(&mut app, &ctx);
    assert!(shown.contains(&grey) && !shown.contains(&white_label::NAVY), "{shown:?}");
    let home = frame_texts(&mut app, &ctx);
    assert!(home.iter().any(|t| t == "Vector illustration — fast, open, scriptable."), "{home:?}");
    assert!(!home.iter().any(|t| t == white_label::TAGLINE), "{home:?}");
}
