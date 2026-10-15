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
