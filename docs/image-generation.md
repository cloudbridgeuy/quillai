# Image generation spike (GUZ-1431)

Decision doc for E18 (GUZ-1428): lore images such as character portraits and
profiles, place maps, and place renders at a time of day. Each image uses the
lore state that is active at the chapter's timeline point. This doc recommends.
The owner decides. No code, API calls, or dependencies were added.

Provider docs were read on 2026-11-06. Prices change, so check them again
before the catalog ships.

## 1. Provider

Current backend: one OpenAI-compatible client. It sends
`POST {base_url}/chat/completions` with a bearer key
(`crates/api/src/ai/provider.rs:99-100`). The base URL and key come from
`ai_settings` or `QUILLAI_AI_BASE_URL` / `QUILLAI_AI_API_KEY`
(`crates/api/src/ai/config.rs:46-47`). The HTTP client is reqwest 0.12.

| Option | Endpoint | Auth | Response | Sizes | Price per image | Works with `{base_url}`? |
|---|---|---|---|---|---|---|
| A. OpenAI Images | `POST /v1/images/generations` [1] | Bearer | GPT image models always return `b64_json`. They do not accept `response_format` and return no URL [1] | 1024x1024, 1536x1024, 1024x1536; gpt-image-2 also takes custom sizes up to 3840x2160 [1] | gpt-image-1-mini: $0.005 / $0.011 / $0.036 for low / medium / high at 1024x1024. gpt-image-2: $0.006 / $0.053 / $0.211 [2] | Yes: `{base_url}/images/generations` |
| B. Gemini OpenAI compatibility | `POST .../v1beta/openai/images/generations` [3] | Bearer (Gemini key) | `b64_json` when `response_format: "b64_json"` is sent [3] | `size`; `aspect_ratio` goes through `extra_body` [3] | gemini-3.1-flash-lite-image: $0.0336 at 1K. gemini-3.1-flash-image: $0.067 at 1K [4] | Yes, same path. Parameters that are not supported are silently ignored, and the docs call it beta [3] |
| C. OpenRouter Image API | `POST /api/v1/images` [5][6] | Bearer | `data[].b64_json` and `media_type`. `usage.cost` is present, but the docs give no currency [6] | `resolution` tiers 512 to 4K, `aspect_ratio`, `size` [6] | Depends on the endpoint. Doc examples: $0.05 (Seedream 4.5) and $0.13 (gpt-image-2, high, 16:9) [5] | No: the path is `/images`, not `/images/generations`. Failed generations return 502 and are not billed [5] |
| D. LocalAI (local) | `POST /v1/images/generations` [7] | Not on that page | Not documented for generation. Diffusers output is served under `/generated-images` (a URL) [7] | `size`, e.g. 1024x1024 [7] | $0 per call (local hardware) | Path yes. The response format is not confirmed |

Recommendation: A. Send `POST {base_url}/images/generations` with the
existing base URL and key, and read `data[0].b64_json` only. Do not send
`response_format`, because GPT image models do not accept it [1]. Send
`output_format: "webp"` or `"jpeg"` [1] to keep the stored bytes small. Start
with `gpt-image-1-mini` at medium quality ($0.011 at 1024x1024 [2]). Note:
`gpt-image-1` was shut down on 2026-10-23 [1][2].

Risks:
- If the owner's base URL points at OpenRouter, option A gives the wrong
  path (see C). See open decision 1.
- B probably needs `response_format: "b64_json"`. Its default is not
  documented. Test B before the owner uses it.
- `crates/api/src/ai/provider.rs` has 992 lines, and the cap is 1000. Put
  the image client in a new file, e.g. `crates/api/src/ai/images.rs`.

## 2. Settings: an `image` action

Add an image model next to the generate and edit models. Change these places:

- Migration `migrations/<ts>_ai_settings_image_model.up.sql`:
  `ALTER TABLE ai_settings ADD COLUMN image_model TEXT;`. The down migration
  drops that column. This follows `20261105000000_ai_settings_per_action_model.up.sql`.
- `crates/api/src/ai/models.rs:75-78`: add `AiAction::Image`. Also update
  `as_str` (`:84`) and `TryFrom` (`:227`). The text request path must reject
  `image` with a 400 (`:320`, `:325`, `:335`, `:186`).
- `crates/api/src/ai/config.rs:11,27,34,46-61,69,76`: add `image_model` and
  its source. Do not fall back to `QUILLAI_AI_MODEL`, because a text model
  cannot make images. Add `QUILLAI_AI_IMAGE_MODEL` instead.
- `crates/api/src/settings/ai.rs:21-22,28-29,36,44-45,54-55,81,96-102`: add
  `image_model` to the read and write structs, the SELECT, the validation
  loop, and the upsert. Then run `cargo sqlx prepare` to refresh `.sqlx/`.
- `crates/api/src/ai/catalog.rs:32-36,92,136`: add `defaults.image`.
- TS: `app/src/settings/client.ts:10-16,40-43` (types, `aiModelActions`,
  parser), `app/src/settings/AiSettings.tsx:17,22,49` (form state, options),
  and `app/src/i18n/messages.ts:262,609` (`settings.ai.imageModel` in EN and ES).
- Keep the TS `AiAction` (`app/src/md_editor/ai_contract.ts:13`) for text
  only. `Editor.tsx:318,360` key prompt overrides and Inspector runs by
  `AiAction`, and those keys mean nothing for images. Add
  `type ModelAction = AiAction | "image"` and use it in
  `app/src/md_editor/model_catalog.ts:12`.

## 3. Catalog and cost

- `CatalogModel` (`crates/api/src/ai/catalog.rs:19-28`) is
  `deny_unknown_fields` and has no model kind. Add
  `kind: "text" | "image"` with serde default `"text"`, so existing catalog
  files stay valid. Image entries need `price_per_image` (USD), `size`, and
  `quality`. Then the request and the price are fixed for each entry, and
  there is no price grid. The text price fields become optional for image
  entries. Reject a catalog where `defaults.image` names a text model, and
  the reverse.
- TS mirror: `app/src/md_editor/model_catalog.ts:3-8` (`AiModel`).
- Cost is TS only and in USD per million tokens
  (`app/src/md_editor/cost.ts:35-55`). Add a `{ kind: "per-image", usd }`
  arm to `CostPricing`. The actual cost is images returned x
  `price_per_image`.
- Inspector (`app/src/md_editor/inspector_cost.ts:13-24`): before the run,
  show the model label, size, quality, and "estimate: $0.011 per image".
  After the run, show "actual: 1 image, $0.011". If the price is missing,
  show the existing `unknown` reasons. Do not read the provider's `usage`
  for images, because each provider gives it in a different shape [2][6].

## 4. Storage

| | SQLite BLOB in `quillai.db` | Files next to the DB |
|---|---|---|
| Backup | One file holds all data. Copying `quillai.db` saves the images too. | You must copy two things. If you copy only the DB, the images are lost without a warning. |
| Trash / soft delete | `deleted_at` on the image row. It is set in the same transaction as `lore_entries.deleted_at` (`migrations/20261101000000_lore_deleted_at.up.sql`). | The row and the file can get out of sync. You need an orphan sweep and a rule for when to delete files. |
| DB size | Grows by about 0.1-0.5 MB per WebP or JPEG image (guess). 1,000 images is about 0.5 GB. | The DB stays small. |
| Serving | `GET /lore/{id}/images/{image_id}` returns the bytes, `Content-Type`, and `Cache-Control: immutable`. | The same route reads the file. Or add the `fs` feature of tower-http (`crates/api/Cargo.toml:36`) to get `ServeDir`. |
| Axum body limit | Not a concern. The server downloads the image with reqwest. No upload goes through an extractor, so Axum's default 2 MB request limit (inferred: axum default) does not apply. | Same. |

Today there is no binary storage: no BLOB column, no multipart, no static
serving, and no body-limit configuration. Data is in `quillai.db`
(`crates/api/src/cli.rs:21`, `xtask/src/dev_database.rs:8-15`).

Recommendation: BLOB rows in a separate table, so lore queries never read
the image bytes:

```sql
CREATE TABLE lore_images (
    id INTEGER PRIMARY KEY,
    lore_entry_id INTEGER NOT NULL REFERENCES lore_entries(id) ON DELETE CASCADE,
    point_id INTEGER REFERENCES timeline_points(id) ON DELETE SET NULL,
    purpose TEXT NOT NULL CHECK (purpose IN ('portrait', 'profile', 'map', 'render')),
    time_of_day TEXT,
    prompt TEXT NOT NULL,
    model TEXT NOT NULL,
    media_type TEXT NOT NULL,
    bytes BLOB NOT NULL,
    created_at TEXT NOT NULL,
    deleted_at TEXT
);
```

Limit: if images make the DB larger than about 2 GB, or if backups become
slow, move the bytes to files and keep this table as the index.

Backup today: the only backup is a browser button, "Download backup"
(`app/src/md_editor/components/Editor.tsx:1551`,
`app/src/i18n/messages.ts:111`). It uses the hand-written TS zip writer
(`app/src/md_editor/zip_store.ts`, GUZ-1239). This spike did not read what
it puts in the zip. There is no server-side backup route
(`crates/api/src/lib.rs:47-52` merges only the users, manuscripts, settings
and ai routes).

How each option fits a future backup:
- BLOB rows: a copy of `quillai.db` (or SQLite `VACUUM INTO`) is a full
  backup with no extra step. A browser zip fetches each image through the
  GET route and adds it with `zip_store.ts`.
- Files: every backup path must also walk the image directory, and the
  backup must stay consistent with the rows. A copy of the DB file alone
  loses the images without a warning.

## 5. Prompting from lore

- Resolve the state with the rule the editor uses now: `effectivePoint` and
  then `resolveLoreEntry` (`app/src/md_editor/lore_states.ts:7,16,29`). This
  rule picks the latest state at or before the chapter's point. The state's
  `visible_text` replaces the entry's text.
- The client sends only `{ name, kind, visible_text, level, purpose,
  time_of_day? }`. The server parses this into a `deny_unknown_fields` struct,
  in the same way as the text path (`crates/api/src/ai/lore.rs:6-13`). Then a
  `hidden_notes` field fails the parse and cannot reach the provider.
- `leave_out`: the server rejects the request with a 422. The UI hides the
  generate button. This matches `crates/api/src/ai/lore.rs:47` and
  `app/src/md_editor/ai_contract.ts:124`.
- `known_dont_reveal`: allowed. Its `visible_text` already goes to the text
  AI (`app/src/md_editor/prompt_sections.ts:66-67`). It is an
  author-facing image, not prose.
- Template (server, pure function):
  `"{purpose preamble}. Subject: {name} ({kind}). {visible_text}. {time_of_day clause}. No text or lettering."`
  Preambles: portrait means "head-and-shoulders portrait". Map means
  "top-down illustrated map". Render means "establishing view". Store the
  final prompt in `lore_images.prompt`, and show it in the Inspector before
  the author sends it.

## 6. Base64 decode without a new dependency

- `base64 0.22.1` is already compiled into the build through reqwest
  (`cargo tree -i base64`: reqwest -> quillai_api). It is not a direct
  dependency.
- Option A: decode by hand, in about 30 lines plus tests. Precedent:
  the GUZ-1239 zip writer (`app/src/md_editor/zip_store.ts`, 63 lines,
  hand-written CRC32).
- Option B: add `base64 = "0.22"` to `crates/api/Cargo.toml`. This adds no
  new crate to `Cargo.lock` and no new build time. Padding and invalid
  input are handled by tested code. This needs owner approval.
- Recommendation: B. The crate is already in the build, and a hand-written
  decoder adds code that can have bugs. If the owner says no, use A, with
  tests for padding, whitespace, and invalid characters.

## 7. Open decisions for the owner

1. Which provider is on the base URL now? If it is OpenRouter, accept a
   per-catalog-entry `images_path` (default `/images/generations`), or
   support only `/images/generations`.
2. Use one key and base URL for text and images (recommended), or add a
   separate image base URL and key.
3. Start model and quality: `gpt-image-1-mini` at medium (recommended), or
   `gpt-image-2`.
4. Storage: BLOB table (recommended), or files.
5. `base64` as a direct dependency (recommended), or a hand-written decoder.
6. Should "Download backup" include images?
7. Keep old images when a lore state changes (recommended: keep, tagged with
   `point_id`), or replace them.

## 8. Proposed follow-up tickets (text only; Linear not changed)

**GUZ-1432: Character images.** Add the `image_model` setting and migration,
catalog `kind` and `price_per_image`, the `ai/images.rs` client
(`/images/generations`, `b64_json`), the `lore_images` table and its GET
route, and Portrait and Profile buttons on character lore entries. The
prompt is built from the state that is active at the chapter's timeline
point. `hidden_notes` and `leave_out` never reach the provider. The
Inspector shows the estimated and actual cost per image. Tests: a Rust
integration test with a stub provider, a test that rejects `leave_out`, a
test that rejects a `hidden_notes` field, and the Inspector cost unit. E2E
spec written, not run.

**GUZ-1433: Placed images (maps and renders).** Depends on GUZ-1432. Place
lore entries get "Map" and "Render", and a render has a time of day (dawn,
day, dusk, night). Images are listed per entry and filtered by timeline
point, with soft delete through the trash. Optional: choose the landscape
size per purpose in the catalog. Tests: prompt template unit tests for each
purpose and time of day, and soft-delete cascade with the lore entry. E2E
spec written, not run.

## Sources (read 2026-11-06)

1. https://developers.openai.com/api/reference/resources/images/methods/generate
2. https://developers.openai.com/api/docs/guides/image-generation (cost table)
   and https://developers.openai.com/api/docs/pricing
3. https://aistudio.google.com/docs/openai (redirected from ai.google.dev/gemini-api/docs/openai)
4. https://aistudio.google.com/docs/pricing (redirected from ai.google.dev/gemini-api/docs/pricing)
5. https://openrouter.ai/docs/features/multimodal/image-generation
6. https://openrouter.ai/docs/api/api-reference/images/generate-an-image
7. https://localai.io/docs/features/image-generation/
