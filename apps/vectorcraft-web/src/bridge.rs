//! The embed bridge's messages (`?embed=1`): what a parent page posts to the editor in its frame
//! and what the editor posts back. The page side (listening, posting) is in `web.rs`; the protocol
//! is described in `docs/embed-bridge.md`.

use serde_json::{Map, Value, json};
use vectorcraft_engine::cmd::fileio;
use vectorcraft_ui_egui::{VectorcraftApp, io};

/// The most SVG text a `vectorcraft:load` may carry (bytes).
pub const MAX_SVG_LEN: usize = 64 * 1024 * 1024;

/// The prefix of every bridge message's `type`.
const PREFIX: &str = "vectorcraft:";

pub const LOAD: &str = "vectorcraft:load";
pub const EXPORT: &str = "vectorcraft:export";
pub const READY: &str = "vectorcraft:ready";
pub const BOOTED: &str = "vectorcraft:booted";

/// A request from the parent page. `id` is echoed back in the reply (null when not given).
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Open `svg` as the document, in place of any open ones.
    Load { id: Value, svg: String },
    /// Reply with the document as SVG.
    Export { id: Value },
    /// Reply at once: the editor is running.
    Ready { id: Value },
}

/// Why a message isn't a [`Request`].
#[derive(Debug, Clone, PartialEq)]
pub enum Rejected {
    /// Not a bridge message (other scripts on the page post messages too): ignored.
    Foreign,
    /// A bridge message that can't be carried out: answered with `ok: false`.
    Bad { kind: String, id: Value, error: String },
}

/// Read a message's data, as JSON text.
pub fn parse(text: &str) -> Result<Request, Rejected> {
    let Ok(Value::Object(m)) = serde_json::from_str::<Value>(text) else {
        return Err(Rejected::Foreign);
    };
    let Some(kind) = m.get("type").and_then(Value::as_str).filter(|t| t.starts_with(PREFIX)) else {
        return Err(Rejected::Foreign);
    };
    let id = m.get("id").cloned().unwrap_or(Value::Null);
    let bad = |error: String| Rejected::Bad { kind: kind.to_string(), id: id.clone(), error };
    match kind {
        LOAD => {
            let svg = m.get("svg").and_then(Value::as_str).ok_or_else(|| bad("\"svg\" must be a string".into()))?;
            if svg.len() > MAX_SVG_LEN {
                return Err(bad(format!("the SVG is {} bytes, more than the {MAX_SVG_LEN} a load takes", svg.len())));
            }
            Ok(Request::Load { id, svg: svg.to_string() })
        }
        EXPORT => Ok(Request::Export { id }),
        READY => Ok(Request::Ready { id }),
        _ => Err(bad(format!("unknown message type \"{kind}\""))),
    }
}

/// The reply to a request of type `kind`: `{type, id, ok: true}` plus `fields`.
pub fn reply_ok(kind: &str, id: &Value, fields: Map<String, Value>) -> String {
    let mut m = Map::new();
    m.insert("type".into(), kind.into());
    m.insert("id".into(), id.clone());
    m.insert("ok".into(), true.into());
    m.extend(fields);
    Value::Object(m).to_string()
}

/// The reply to a request of type `kind` that failed with `error`.
pub fn reply_err(kind: &str, id: &Value, error: &str) -> String {
    json!({ "type": kind, "id": id, "ok": false, "error": error }).to_string()
}

/// Carry out `req` in `app` → the reply to post to the parent.
pub fn handle(app: &mut VectorcraftApp, req: Request) -> String {
    match req {
        Request::Load { id, svg } => match load(app, &svg) {
            Ok(()) => reply_ok(LOAD, &id, Map::new()),
            Err(e) => reply_err(LOAD, &id, &e),
        },
        Request::Export { id } => match export(app) {
            Ok(svg) => reply_ok(EXPORT, &id, Map::from_iter([("svg".to_string(), Value::String(svg))])),
            Err(e) => reply_err(EXPORT, &id, &e),
        },
        Request::Ready { id } => reply_ok(READY, &id, Map::new()),
    }
}

/// Open `svg` as the only document: the embedding page owns the document, so the open ones close
/// without asking.
fn load(app: &mut VectorcraftApp, svg: &str) -> Result<(), String> {
    if !app.session.documents().is_empty() {
        // The engine's Close All, not the menu's, which asks about modified documents first.
        vectorcraft_ui_egui::background::wait_all(app);
        let closed = app.session.execute("file.closeAll", &json!({})).map_err(|e| e.to_string());
        app.sync_views();
        closed?;
    }
    io::open_bytes(app, "embedded.svg", svg.as_bytes(), None)?;
    if app.session.active().is_none() {
        return Err("the SVG didn't open as a document".into());
    }
    Ok(())
}

/// The active document as SVG text.
fn export(app: &VectorcraftApp) -> Result<String, String> {
    let st = app.session.doc().map_err(|e| e.to_string())?;
    let bytes = fileio::encode(&st.doc, "svg", &Value::Null).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| format!("the SVG isn't UTF-8: {e}"))
}

/// What the editor posts to its parent once it is running.
pub fn booted() -> String {
    json!({ "type": BOOTED }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_three_requests() {
        assert_eq!(parse(r#"{"type":"vectorcraft:load","id":1,"svg":"<svg/>"}"#), Ok(Request::Load { id: json!(1), svg: "<svg/>".into() }));
        assert_eq!(parse(r#"{"type":"vectorcraft:export","id":2}"#), Ok(Request::Export { id: json!(2) }));
        assert_eq!(parse(r#"{"type":"vectorcraft:ready","id":"a"}"#), Ok(Request::Ready { id: json!("a") }));
        assert_eq!(parse(r#"{"type":"vectorcraft:ready"}"#), Ok(Request::Ready { id: Value::Null }));
    }

    #[test]
    fn ignores_foreign_messages() {
        for text in ["", "not json", "42", "[]", "{}", r#"{"type":5}"#, r#"{"type":"webpackOk"}"#, r#"{"source":"react-devtools"}"#] {
            assert_eq!(parse(text), Err(Rejected::Foreign), "{text}");
        }
    }

    #[test]
    fn rejects_bad_bridge_messages_with_their_id() {
        let Err(Rejected::Bad { kind, id, .. }) = parse(r#"{"type":"vectorcraft:load","id":3}"#) else { panic!("missing svg accepted") };
        assert_eq!((kind.as_str(), id), (LOAD, json!(3)));
        let Err(Rejected::Bad { kind, id, error }) = parse(r#"{"type":"vectorcraft:frobnicate","id":4}"#) else { panic!("unknown type accepted") };
        assert_eq!((kind.as_str(), id), ("vectorcraft:frobnicate", json!(4)));
        assert!(error.contains("unknown"));
        assert!(matches!(parse(r#"{"type":"vectorcraft:load","id":5,"svg":7}"#), Err(Rejected::Bad { .. })));
    }

    #[test]
    fn caps_the_svg_size() {
        let text = json!({ "type": LOAD, "id": 6, "svg": "x".repeat(MAX_SVG_LEN + 1) }).to_string();
        assert!(matches!(parse(&text), Err(Rejected::Bad { .. })));
    }

    fn app() -> VectorcraftApp {
        VectorcraftApp::new(vectorcraft_engine::Session::new(), vectorcraft_ui_egui::Services::default())
    }

    fn reply(app: &mut VectorcraftApp, text: &str) -> Value {
        serde_json::from_str(&handle(app, parse(text).unwrap())).unwrap()
    }

    #[test]
    fn loads_edits_and_exports() {
        let mut app = app();
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect id="r" x="10" y="10" width="50" height="40" fill="#ff0000"/></svg>"##;
        let load = json!({ "type": LOAD, "id": 1, "svg": svg }).to_string();
        assert_eq!(reply(&mut app, &load), json!({"type": LOAD, "id": 1, "ok": true}));
        // An edit in the editor reaches the export.
        app.run("shape.rectangle", json!({"x": 100, "y": 20, "width": 30, "height": 30})).unwrap();
        let out = reply(&mut app, r#"{"type":"vectorcraft:export","id":2}"#);
        assert_eq!((&out["ok"], &out["id"]), (&json!(true), &json!(2)));
        let exported = out["svg"].as_str().unwrap();
        assert!(exported.contains("<svg"), "{exported}");
        // Loading it again replaces the document instead of adding a second.
        let again = json!({ "type": LOAD, "id": 3, "svg": exported }).to_string();
        assert_eq!(reply(&mut app, &again)["ok"], json!(true));
        assert_eq!(app.session.documents().len(), 1);
        let shapes = |s: &str| s.matches("<rect").count() + s.matches("<path").count();
        let reexported = reply(&mut app, r#"{"type":"vectorcraft:export","id":4}"#);
        assert_eq!(shapes(reexported["svg"].as_str().unwrap()), shapes(exported));
        assert!(shapes(exported) >= 2, "{exported}");
    }

    #[test]
    fn reports_failures() {
        let mut app = app();
        let out = reply(&mut app, r#"{"type":"vectorcraft:export","id":1}"#);
        assert_eq!((&out["ok"], &out["id"]), (&json!(false), &json!(1)));
        assert!(out["error"].is_string());
        let out = reply(&mut app, r#"{"type":"vectorcraft:load","id":2,"svg":"this is not svg"}"#);
        assert_eq!(out["ok"], json!(false), "{out}");
        assert_eq!(reply(&mut app, r#"{"type":"vectorcraft:ready","id":3}"#), json!({"type": READY, "id": 3, "ok": true}));
    }

    #[test]
    fn replies_round_trip() {
        let mut fields = Map::new();
        fields.insert("svg".into(), "<svg>\"q\"</svg>".into());
        let v: Value = serde_json::from_str(&reply_ok(EXPORT, &json!(7), fields)).unwrap();
        assert_eq!(v, json!({"type": EXPORT, "id": 7, "ok": true, "svg": "<svg>\"q\"</svg>"}));
        let v: Value = serde_json::from_str(&reply_err(EXPORT, &json!(8), "no document")).unwrap();
        assert_eq!(v, json!({"type": EXPORT, "id": 8, "ok": false, "error": "no document"}));
        let v: Value = serde_json::from_str(&booted()).unwrap();
        assert_eq!(v, json!({"type": BOOTED}));
    }
}
