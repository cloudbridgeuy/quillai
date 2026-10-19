# AI Streaming — API Request, Prompt, and Provider Flow

## Overview

The AI backend lives in `crates/api/src/ai/`. It accepts a rich editor request, validates it at the HTTP boundary, builds a plain-text prompt, calls an OpenAI-compatible streaming endpoint, and translates upstream SSE chunks into the app's NDJSON event stream.

## Architecture

The feature follows Functional Core and Imperative Shell.

**Functional Core**:
- `models.rs` parses request JSON into typed `GenerateInput` values.
- `prompt.rs` builds the provider prompt from the document text, selection text, story context, and instruction.
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

Optional `sectionOverrides` maps `system`, `instruction`, `story_context`, `selection`, and `document` to raw content strings; `history` accepts an ordered list of `{role: "user" | "assistant", text: string}`. Omission or `{}` leaves existing assembly unchanged. Unknown ids, invalid types/roles, extra history fields, and oversized overrides return 422 before streaming. Text overrides preserve whitespace but pass through normal escaping and fixed section tags; action and selection range metadata are not editable. Empty text keeps the frame. System override text is escaped. History turns remain separate untagged chat messages, with roles/order/text preserved; `[]` removes history. Overrides do not bypass document/selection parsing or change the Quill insertion range. Both API and inspector support Continue/Resume and Edit.

No explicit API prompt/request size cap was found in the application code. Override content is capped at 65,536 UTF-8 bytes per section (all history turn texts combined) and 262,144 bytes total across override text, before escaping. This permits large chapter context while bounding added prompt allocation/provider cost independently of the HTTP body extractor. JSON framing and role names do not count toward these content caps. Limits are parsed once in `section_overrides.rs`.

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

`prompt.rs` builds a plain-text provider prompt with action, instruction, story context, selection details when present, and document text. Section overrides replace content, not tags or selection range metadata.

All user-controlled text inserted into tagged sections, including overrides, escapes `&`, `<`, and `>` so strings like `</document>` cannot break the prompt frame. History stays in separate chat messages without section tags, matching non-override history handling.

## Streaming Contract

The browser-facing response is `application/x-ndjson` with one event per line:

```json
{"type":"delta","delta":{"ops":[{"insert":"New text"}]}}
{"type":"done"}
```

Supported event types:

- `delta`
- `done`
- `error`
- `cancelled`

The current provider implementation emits `delta`, `done`, and `error`. Browser cancellation is handled locally by aborting the request.

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
