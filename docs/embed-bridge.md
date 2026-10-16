# Embed bridge (`?embed=1`)

The web app can run inside another page's `<iframe>`. The parent page sends it a document and takes
the edited document back with `window.postMessage`. This is the integration contract for pages that
embed VectorCraft; the code is `apps/vectorcraft-web/src/bridge.rs` (messages) and `src/web.rs`
(`track_messages`, `post_to_parent`).

Without `?embed=1` in the address the app doesn't listen for messages or post any.

```html
<iframe src="https://example.com/vectorcraft/?embed=1" style="width:100%;height:100%;border:0"></iframe>
```

## Messages

Every message is a JSON object with a `type` starting with `vectorcraft:`. The parent may post the
object itself or its JSON text; the editor always posts objects. Requests carry an `id` (any JSON
value, `null` when left out) that the reply echoes, so the parent can match replies to requests.

| From | Message | Meaning |
|---|---|---|
| editor | `{"type":"vectorcraft:booted"}` | The editor is running. Posted once, when the app first starts (not again after a graphics restart). |
| parent | `{"type":"vectorcraft:ready","id":n}` | Ping. Reply: `{"type":"vectorcraft:ready","id":n,"ok":true}`. |
| parent | `{"type":"vectorcraft:load","id":n,"svg":"<svg …>"}` | Open the SVG as the document. Open documents close first without asking (the parent owns the document). Reply: `{"type":"vectorcraft:load","id":n,"ok":true}`. |
| parent | `{"type":"vectorcraft:export","id":n}` | Reply with the active document as SVG: `{"type":"vectorcraft:export","id":n,"ok":true,"svg":"<svg …>"}`. |

A request that fails gets `{"type":<request type>,"id":n,"ok":false,"error":"…"}`: an SVG that
doesn't open, an export with no document open, a load without a string `svg` or with more than
64 MiB of it, an unknown `vectorcraft:` type. Messages whose `type` doesn't start with
`vectorcraft:` (other scripts post messages too) are logged to the console and ignored.

Requests wait for the editor's next frame, so the parent can post them as soon as it likes: ones
posted before `vectorcraft:booted` are answered once the editor runs. Sending `vectorcraft:ready`
until it is answered also tells the parent the editor is up, if it missed `booted`.

The export is the document as it was last committed: finish an edit in progress (e.g. typing in a
text frame) before asking for it.

## White label

With `?embed=1` the editor presents itself as the **Prolytiq Figure Editor**: nothing shown names
VectorCraft, ArtCraft or Crafting Apps, and nothing links to Discord. The switch is
`vectorcraft_ui_egui::white_label`, which the page turns on before the first frame; without
`?embed=1` (and on the desktop) the default branding is unchanged.

- **Names.** Every interface string goes through `i18n`, which renames VectorCraft → "Prolytiq
  Figure Editor" and ArtCraft / Crafting Apps → "Prolytiq" in every language (the `i18n/*.tsv`
  catalogs are untouched). The app-bar title, window title, About box, Home screen ("Welcome to
  Prolytiq Figure Editor"), menus, status messages and the startup error say the same.
- **Page.** `index.html` carries the embed build's title ("Prolytiq Figure Editor") and loading
  text ("Welcome to the Prolytiq figure editor"), which is what shows before the wasm boots. The
  plain web app puts "VectorCraft" back in the title and loading text as soon as it starts.
- **Removed.** The app bar's Discord button, the Help menu's Discord / website / app page /
  GitHub items, the Home screen's Community section, the About box's links and those commands in
  the command palette. `help.discord` and the others still answer agents with their URL, but the
  UI doesn't open them.
- **Mark.** The dragon app icon is replaced by a mark drawn in code (a navy tile with a white "P"
  over an accent bar); no image asset is added.
- **Home screen.** Its tagline reads "Scientific figures, publication-ready." instead of
  VectorCraft's.
- **Theme.** The interface defaults to Medium Light and the canvas around the artboards to navy
  `#0B1E3A`, which is also the background of the Home screen and of the empty window with no
  document open (with light text on it); the accent and selection colour stays `#1473E6`. An explicit Canvas Color preference
  wins, as does any brightness other than the default Medium Dark (Medium Dark itself reads as
  "not chosen" and shows Medium Light).

Internal identifiers stay as they are: the `vectorcraft:` message types, the `.vectorcraft` file
format, element ids and console log lines.

## Security

The bridge accepts messages from any origin and posts its replies with target origin `"*"`, so the
page that frames the editor can read the document. Serve the embed build only where any framing
page may have the documents it loads, or restrict framing with a `Content-Security-Policy:
frame-ancestors …` header on the host. The page sets no `X-Frame-Options`, so it can be framed.

## Example (parent page)

```js
const frame = document.querySelector("iframe");
let next = 0;
const pending = new Map();
window.addEventListener("message", (e) => {
  if (e.source !== frame.contentWindow || !e.data?.type?.startsWith("vectorcraft:")) return;
  if (e.data.type === "vectorcraft:booted") return onBooted();
  pending.get(e.data.id)?.(e.data);
  pending.delete(e.data.id);
});
function send(type, extra = {}) {
  const id = ++next;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    frame.contentWindow.postMessage({ type, id, ...extra }, "*");
  });
}
// await send("vectorcraft:load", { svg });  …  const { ok, svg } = await send("vectorcraft:export");
```
