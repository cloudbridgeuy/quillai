# AI Streaming — API Request, Prompt, and Provider Flow

## Overview

The AI backend lives in `crates/api/src/ai/`. It accepts a rich editor request, validates it at the HTTP boundary, builds a plain-text prompt, calls an OpenAI-compatible streaming endpoint, and translates upstream SSE chunks into the app's NDJSON event stream.

## Architecture

The feature follows Functional Core and Imperative Shell.

**Functional Core**:
- `models.rs` parses request JSON into typed `GenerateInput` values.
- `prompt.rs` builds the provider prompt from the document text, selection text, story context, injected lore, and instruction.
- `segments.rs` owns the minimal Quill-delta contract and sanitization rules.
- provider parsing helpers in `provider.rs` decode UTF-8 chunks, split SSE events, and extract text deltas.

**Imperative Shell**:
- `handlers.rs` extracts JSON, constructs the provider, streams NDJSON, and preserves pre-stream JSON errors.
- `provider.rs` performs HTTP I/O against the OpenAI-compatible `chat/completions` endpoint.

## Request Contract

`POST /ai/generate` accepts:

```json
{
  "action": "generate",
  "document": {
    "delta": { "ops": [{ "insert": "Opening line." }] },
    "text": "Opening line."
  },
  "selection": null,
  "storyContext": "fantasy court intrigue",
  "instruction": "continue the scene"
}
```

`edit` requests send a non-null `selection` slice:

```json
{
  "delta": { "ops": [{ "insert": "line" }] },
  "text": "line",
  "range": { "index": 8, "length": 4 }
}
```

Optional `sectionOverrides` maps `instruction`, `story_context`, `selection`, and `document` to raw content strings; `history` accepts an ordered list of `{role: "user" | "assistant", text: string}`. `system` is rejected with 422, including empty strings. Omission or `{}` leaves existing assembly unchanged. Unknown ids, invalid types/roles, extra history fields, and oversized overrides return 422 before streaming. Text overrides preserve whitespace but pass through normal escaping and fixed section tags; action and selection range metadata are not editable. Empty text keeps the frame. History turns remain separate untagged chat messages with roles/order preserved; both normal and edited history text escape `&`, `<`, and `>` before reaching the provider. Normal history is trimmed first; edited history keeps its whitespace. `[]` removes history. Overrides do not bypass document/selection parsing or change the Quill insertion range. Both API and inspector support Continue/Resume and Edit.

`GET/PUT /settings/author-guidance` reads/saves `{text: string}` in a singleton SQLite row. Missing row and empty text disable guidance. Whitespace is preserved. PUT uses the same 65,536 UTF-8 byte cap as override sections, before escaping; oversize returns 422. The AI handler loads saved text for every accepted request, never from request JSON. The provider keeps its fixed core system message and inserts a separate system message immediately after it: `<author_guidance>\n{escaped text}\n</author_guidance>`. History follows, then the current user prompt. Guidance escapes `&`, `<`, and `>` and cannot replace the core system section. Empty/missing guidance emits no message and preserves the old provider payload byte-for-byte. This is prompt framing, not a guarantee that a model ignores malicious instructions.

There is no application-level cap on the complete assembled prompt. Override content is capped at 65,536 UTF-8 bytes per section (all history turn texts combined) and 262,144 bytes total across override text, before escaping. Lore has a separate 65,536-byte cap. This permits large chapter context while bounding added prompt allocation/provider cost independently of the HTTP body extractor. JSON framing and role names do not count toward the override caps. Override limits are parsed once in `section_overrides.rs`.

## Injected Lore

Optional `lore` accepts an array of `{name, kind, visible_text, level}`. Kind is `character`, `place`, `term`, or `note`; level is `include`, `known_dont_reveal`, or `leave_out`. `lore.rs` rejects malformed entries, unknown fields (including hidden notes), and invalid kinds/levels with 422 before provider startup. It drops `leave_out` entries; the renderer also ignores them defensively. Only name, kind, visible text, and the non-reveal policy enter the prompt. IDs, aliases, keywords, pin state, and hidden notes are not request fields.

`prompt.rs` renders eligible entries between story context and selection as `<lore>\n{escaped content}\n</lore>\n`. Each entry uses `Name: ...`, `Kind: ...`, and `Text: ...` lines; entries are separated by a blank line. `known_dont_reveal` inserts `Known, but MUST NOT be revealed in prose.` before Text. All content escapes `&`, `<`, and `>`. Missing, empty, or leave-out-only lore emits no block and preserves the existing prompt. The combined content, including labels, names, kinds, non-reveal markers, and entry separators, must fit 65,536 UTF-8 bytes before escaping; overflow returns 422. Outer tags and JSON framing do not count. Lore is not a System override and cannot be replaced through `sectionOverrides.lore`.

This policy marker is model guidance, not a guarantee against disclosure. Hidden notes never enter the AI contract.

## Validation Rules

The backend validates all of these before streaming starts:

- `action` must be `generate` or `edit`.
- `document.delta` must sanitize to valid insert-only Quill-like ops.
- AI request slices currently support text inserts only; embed inserts are rejected at the boundary.
- `document.text` must match the sanitized delta's plain text.
- `edit` requires a non-empty `selection`.
- `selection.range` is validated against `document.text` using UTF-16 indexing so it matches Quill and JavaScript string offsets.
- `selection.text` must match the selected span in `document.text`.
- `selection.delta` plain text must match `selection.text`.

Validation failures return the normal JSON error contract:

```json
{ "error": "...", "code": "..." }
```

## Prompt Assembly

`prompt.rs` builds a plain-text provider prompt with action, instruction, story context, optional lore, selection details when present, and document text. Section overrides replace editable content, not tags, lore policy, or selection range metadata.

All user-controlled text inserted into tagged sections, including overrides, escapes `&`, `<`, and `>` so strings like `</document>` cannot break the prompt frame. Normal and edited history now use the same escaping, while staying in separate chat messages without section tags. For example, history `</document> &` reaches the provider as `&lt;/document&gt; &amp;`.

## Streaming Contract

The browser-facing response is `application/x-ndjson` with one event per line:

```json
{"type":"delta","delta":{"ops":[{"insert":"New text"}]}}
{"type":"usage","promptTokens":12,"completionTokens":34}
{"type":"done","finishReason":"stop"}
```

Supported event types:

- `delta`
- `usage`
- `done`
- `error`
- `cancelled`

`usage` carries non-negative integer `promptTokens` and `completionTokens`. The provider asks upstream for `stream_options: {"include_usage": true}` and emits `usage` only when the final usage chunk includes both `prompt_tokens` and `completion_tokens`. `total_tokens` is not forwarded. A missing, null, or incomplete usage object emits no `usage` event; the stream still ends with `done` or `error`. An earlier `finish_reason` is kept and written on `done` after usage is read. `[DONE]` or the end of the upstream body completes the response.

The browser parser in `app/src/md_editor/ai_stream.ts` accepts `usage`. The editor does not display token counts. Its apply loop still treats any event other than `delta` or `error` as the end of the run, so a `usage` line stops it before `done` and the thread keeps `finishReason` `unknown`.

The current provider implementation emits `delta`, `usage` when present, `done`, and `error`. Browser cancellation is handled locally by aborting the request.

## Local CORS Constraint

`crates/api/src/lib.rs` currently applies a narrow local-development CORS layer that allows the frontend origins:

- `http://localhost:3001`
- `http://127.0.0.1:3001`

That matches the current browser-side AI path assumptions. Running the frontend on a different port keeps the app shell reachable, but browser AI requests will fail unless the CORS allowlist is expanded.

## Provider Behavior

The current provider uses direct `reqwest` HTTP instead of `rig` so the code keeps explicit control over:

- OpenAI-compatible `base_url`
- SSE framing and CRLF normalization
- UTF-8 chunk boundaries
- cancellation and error translation

Required env vars:

- `QUILLAI_AI_API_KEY`
- `QUILLAI_AI_MODEL`

Optional env var:

- `QUILLAI_AI_BASE_URL`

## Tests

```sh
cargo test -p quillai_api
SQLX_OFFLINE=true cargo check -p quillai_api
```

Key backend coverage includes request parsing and validation, UTF-16 selection handling, prompt assembly, SSE chunk parsing, and AI route happy paths plus validation failures.
