# AI Editor — Quill Shell, Helpers, and Browser URL Constraint

## Overview

The AI editor lives under `app/src/md_editor/`. Quill holds the visible document; the manuscript API holds the saved chapter. Pure helpers handle revision transforms and AI request/stream logic. `components/Editor.tsx` coordinates selection capture, AI streaming, cancellation, and rollback.

## Architecture

The frontend split is deliberate.

**Functional Core**:
- `ai_contract.ts` — request types, stream-event types, URL helper, selection rebasing, and visible-only lore request assembly. `buildAiRequest` accepts optional `model` for both actions and includes it unchanged only when defined; omission leaves the field absent so the server uses the action default.
- `model_catalog.ts` — pure parsing of the public model catalog and per-action choice resolution. Continue uses `defaults.generate`; Edit uses `defaults.edit`. Missing or unknown saved IDs fall back to that action's server default. Empty, malformed, or inconsistent catalogs resolve to unavailable, so requests omit `model`.
- `context_inspector.ts` — pure prompt-section view-model with summed estimated tokens and ordered history turns; injected lore lists names, kinds, levels, keyword/pin reasons, standalone Lore token estimates, and budget skips. It uses the shared matcher and request budget on the transmitted window, omits leave-out entries before budgeting, and never exposes hidden notes. Per-entry estimates include standalone tags and are not additive. The editable live panel selects Continue or Edit and uses the same request builder as sending. Continue shows "with thread" when effective history is nonempty, including history overrides. It refreshes for Quill/settings/thread changes and handles unavailable previews. The right drawer starts closed and remains available inline below 1100px. Only edited ids enter `sectionOverrides`; Reset restores assembly per section. Each action owns separate overrides, cleared after its accepted send or any chapter load/remote reset, never persisted. History is edited per speaker turn, with explicit user/assistant add controls. Editing alone sends no request.
- `editor_panels.ts` — pure mutually exclusive drawer/Settings transitions and Ctrl/Cmd+[ / Ctrl/Cmd+] shortcut mapping. Escape closes drawers, Settings, and the pink Edit note, then restores editor selection/focus. Hover changes rail contrast only; rail mousedown prevents focus theft. The centre column contains only the chapter title, Quill toolbar, and text. The right context drawer holds Story Context, Instruction, and AI Thread/Reset Thread beside the live Continue context. Existing Max Output Tokens and Thread continuation prompt controls are under Advanced AI controls in this drawer. At 1100px and wider the bottom gear opens a Settings popover outside the prose with persisted Window Size, Whole document, and Tint AI text. Outside pointer presses dismiss it without redirecting focus; Escape and Close restore the remembered editor selection. Narrow screens retain window controls in the drawer. Export local chapter is in the left manuscript drawer. The Preview Context button/modal and middle Edit button are removed.
- `prompt_sections.ts` — pure server-parity assembly from accepted requests. `editableText` is raw section content; `text` is provider-rendered content. Text overrides pass through normal escaping and fixed tags/action/selection metadata; empty text keeps its section frame. History overrides are ordered `{role, text}` turns, preserving roles and message boundaries without tagged framing. Both normal and edited history escape `&`, `<`, and `>` before provider delivery and token estimation; normal history is trimmed first, while edited history preserves whitespace. `promptHistory` exposes raw turns for editing; `assemblePromptHistory` exposes escaped provider messages. History display concatenates escaped turn text; current-turn sections join without separators. Token estimates are `ceil(UTF-8 bytes / 4)` per section, summed for totals, excluding chat framing, not tokenizer counts.
- `ai_stream.ts` — incremental NDJSON parser plus final remainder handling.
- `ai_state.ts` — AI run-state ADT and transition helpers.
- `window.ts` — context-window calculation.
- `delta_bridge.ts` — Quill delta normalization at the contract boundary.
- `editor_ai.ts` — pure selection-memory, shared Enter/Apply availability, rollback, and status-presentation helpers. Quiet header save labels omit the repeated chapter title and center above the sheet in document flow. Save/recovery failures show "Not saved" in a red pill floating over the header desk without shifting the sheet. The unchanged detailed reason is in the `title` tooltip and a visually hidden screen-reader description, announced assertively. No fixed save overlay covers prose on scroll.
- `chapter_sync.ts` — server document/revision, one in-flight Delta, composed pending Delta, server-priority remote transforms, and acknowledgement reconciliation.
- `document_diff.ts` — sync document differences skip fast-diff's semantic cleanup because it can shift an edit into a surrogate pair; equal-text Delta diff then reconciles attributes and embeds.
- `editor_persistence.ts` — pure parse/serialize of localStorage AI controls and thread settings, including nullable `threadState.settings.continueModel` and `editModel`. Older settings without these fields remain valid. The editor writes an empty document placeholder, never restores chapter content from this key, and resets the AI thread when a chapter loads or remote content changes.

**Imperative Shell**:
- `components/LoreWindow.tsx` — desktop-only CSS Custom Highlights on name/alias/keyword occurrences in the calculated Continue window, independent of bracket visibility. The shared `context_inspector.ts` lore model supplies occurrences, read-only visible text, levels, and budgeted injected entries; no hidden notes enter this model. The list always previews Continue, even when the selected inspector action is Edit. Pins without matches remain listed with disabled navigation. Hover after 600ms or Alt+L opens a card; Escape closes. Clicking a matched list row scrolls to its first occurrence and opens its card. No Quill format, Delta, or save-path changes. Occurrence offsets map NFC grapheme boundaries back to original UTF-16 indices, including decomposed accents and emoji. `e2e/lore.spec.ts` verifies both desktop widths, saved content/revision invariance, hidden-note exclusion, window changes, pins and budget skips; screenshots go to `target/guz-759/`.
- `components/Editor.tsx` — owns Quill, DOM refs, browser `fetch`, `AbortController`, streamed delta application, and UI state wiring. Loads GET /ai/models on mount with the shared API base URL. The right drawer's Advanced AI controls contain separate Continue model and Edit model selects (not Settings). Choices persist locally and feed the same request builder used for preview/send. Loading, failed, or empty catalogs disable both selects with a short note and omit `model`, retaining server-default behavior. A failed catalog load can be retried by reloading.
- `components/ContinueWindow.tsx` — at 1100px and wider, renders one blue gutter bracket from the existing `calculateDocumentWindow` start to end, with the lower cap at the end index's line bottom, including context after the caret. Whole document spans the full calculated document length. Saved history adds no second or lavender marker. The calculator, request builder, and transmitted slice remain unchanged. Quill bounds, resize observation, editor changes, and scroll events track author typing, concurrent Continue fragments/author edits, and layout changes. An off-screen start shows the top-viewport button `Continue window / starts ↑ N pages up`, with viewport-height distance rounded to an integer, or `just above` below one viewport height; only clicking it scrolls to the start, without moving selection. Settings persists `showContinueWindow` under the same localStorage key as Tint AI text, default true; disabling hides both bracket and flag. Both are absent below 1100px.
- `components/ManuscriptTree.tsx` — loads the ordered series/books/chapter-summary outline through the existing manuscript client when opened, with loading, empty, error, and retry states. Selection requests a save-before-switch and highlights only the open chapter. Reopening refreshes the outline.
- `chapter_session.ts` — hierarchy selection/bootstrap and serialized POST/GET orchestration.
- `use_chapter_sync.ts` — Quill synchronization, one-second polling, save status, unload warning, and per-tab sessionStorage recovery/export.

## Chapter Loading And Saving

- A URL with `seriesId`, `bookId`, and `chapterId` opens that scoped server chapter. All three IDs must be positive integers.
- Without IDs, selection walks the ordered series/books hierarchy and opens its first chapter. The selected scope is written to the URL, so reload opens the same server chapter with its current content and `head_rev`.
- Chapter lists contain only `id`, `title`, `position`, `kind`, and `head_rev`. Initial hierarchy selection fetches the first chapter through the single-chapter GET; bootstrap only needs the listed ID. Tree selection disables editing and further switches, awaits any active synchronization cycle and flushes remaining pending edits, then issues one single-chapter GET. A successful load replaces the session, clears undo/AI context, and updates the title, URL, and selected highlight. Failed saves or loads leave the current session and text in place. A failed delta POST is reconciled with `GET .../deltas?since=baseRev` before another submission: a history revision that matches the in-flight delta, after earlier revisions transform it with server priority, is acknowledged and not sent again. Only a history that does not contain that submission is composed back into pending and sent on a later poll. A history read that fails leaves the in-flight delta unsent. Gaps and other non-network history errors still stop saving.
- An empty hierarchy offers **Create chapter**. Only this explicit action creates missing default series/book/chapter resources through the manuscript API. Concurrent bootstrap actions can create separate defaults; no backend uniqueness is assumed.
- All local document changes enter the same sync path: typing, formatting, undo/redo, markdown conversion, AI selection deletion, streamed fragments, and cancellation/failure restoration. Markdown's silent replacement is captured by comparing the final Quill document with the sync core's visible document.
- Quill's document normalization, including the required terminal newline, enters that same local path after chapter load or a remote update.
- Each one-second cycle permits at most one POST `{baseRev, delta}`, then GETs revisions since the last acknowledged server revision. Polling never overlaps a submission or another poll. New edits compose into pending work while network I/O is active.
- The POST response identifies the client's revision in the returned history. Earlier remote revisions transform both in-flight and pending work with server collision priority. The own revision acknowledges rather than reapplies the local edit. Later remote revisions transform remaining pending work.
- AI starts only when local work is saved and no network cycle is active. Polling/sending pauses during the AI run to keep stream anchors and rollback ranges stable. The complete resulting edit enters the next save cycle.
- An uncertain POST failure keeps the red **Not saved** pill and reconciles before resubmitting, because the API has no idempotency key. The next poll sends again only when history since `baseRev` does not already contain the submission. A matching revision clears the pill without a second insert. GET network failures after a successful POST retain the acknowledgement and retry history only. Protocol and history gaps stop saving and make the editor read-only. The header shows quiet **Saved**/**Unsaved changes** labels or a floating red **Not saved** pill without layout shift. Full failure/recovery details remain in its tooltip and screen-reader description. Local work remains available through **Export local chapter** in the left manuscript drawer.
- Unsaved visible documents and chapter scope are copied to per-tab sessionStorage. Reload archives any previous unsaved copy and loads the server chapter, never silently replays uncertain local changes. **Export recovery** downloads archived copies. An unload warning protects unsaved work; unavailable/full recovery storage is shown in save status. Closing the tab ends sessionStorage retention, so export before closing.

## Browser URL Constraint

AI and manuscript requests share `app/src/api_config.ts`. Its pure `parseApiBaseUrl(override)` returns a typed result (`default`, `override`, or `rejected` with a reason) and never throws; `resolveApiBaseUrl(override)` returns its URL. The default is `http://127.0.0.1:2908`. An override must be an absolute HTTP(S) URL whose host is loopback (`127.0.0.1`, `localhost`, or `[::1]`), on any port; whitespace and trailing slashes are trimmed. Malformed values, other schemes, non-loopback hosts (including `127.0.0.1.evil.example` and userinfo tricks such as `http://127.0.0.1@evil.example`), credentials, queries, and fragments all fall back to the default, so chapter text cannot be sent off-machine.

Set the frontend URL's `apiBaseUrl` query parameter to override the API URL, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. `getApiBaseUrl()` in the same module reads `window.location.search` when called (`buildAiGenerateUrl()` per AI request, `createManuscriptClient()` when the client is created; returning the default when no `window` exists) and passes the value to the pure resolver; the editor shell and clients do not pass it in. Chapter selection preserves the parameter. Reload after changing it. This uses the browser's native URL support with Bun-served bundles in development and production: no config endpoint, injected script, rebuild, or browser-side `process.env` (which previously caused `process is not defined`). The E2E sandbox can select any API port with its initial navigation URL.

Backend CORS defaults to `http://localhost:3001` and `http://127.0.0.1:3001`. Repeat the API's `--cors-origin <origin>` flag to replace the allow-list for another frontend origin. API port configuration alone does not change CORS; `cargo xtask dev` defaults are unchanged.

## Browser Smoke Sandbox

`model-picker.spec.ts` is in the Playwright allowlist. It mocks GET /ai/models and POST /ai/generate, selects different Continue/Edit IDs, reloads to prove persistence, and checks both request bodies. The HTTP failure case checks disabled selects and omitted `model` for both actions. Screenshots at 1100px and 1440px go to `target/guz-680/`.

The right drawer uses two font sizes: 12px labels, small-caps eyebrows, and existing mono prompt text; 13px body, controls, help, summaries, and note placeholders. Story Context, Instruction, Thread continuation prompt, and author guidance reuse the yellow `StickyNote`; read-only prompt fields remain plain. Desktop drawers span the viewport with internal scrolling and contained scroll chaining. Browser coverage checks typography, shared note styling, and top/bottom geometry before and after internal scrolling at 1100px and 1440px. `drawer-top-*.png` and `drawer-bottom-*.png` capture the viewport, not a full document image that extends below a fixed drawer.

On desktop, the right drawer's heading stays pinned on opaque paper with a 1px divider while the content scrolls underneath. There is no Close context button: the Context rail button toggles the drawer, sets `aria-expanded`, and stays ink-on-green with the right rail unmuted while open. A second click or Escape closes and restores editor focus; Ctrl/Cmd+] still toggles it. Heading alignment with QuillAI and the Continue arrow remains within 2px. The browser spec verifies active/inactive rail states, focus restoration, shortcuts, and unchanged header offsets after scrolling to the bottom, and hit-tests its opaque area to exclude scrolled content above the divider.

Catalog precedence: `--ai-models` > `QUILLAI_AI_MODELS_FILE` > `e2e/fixtures/ai-models.json`, resolved from the compile-time workspace root, independent of the invocation directory. Only the default fixture sets the API's provider URL to `http://127.0.0.1:9/v1` with a fake key, overriding inherited provider credentials; caller catalogs retain caller provider configuration. Provider settings cannot live in the catalog's strict JSON schema. Generation specs intercept `/ai/generate` with Playwright routes or in-page fetch stubs, never a live provider. Outside the sandbox, a missing catalog still returns the API's existing 500: the browser can report its native network error; the editor logs nothing extra, disables both model menus, and keeps requests on server defaults. This preserves configuration diagnostics without interrupting writing.

`continue-window.spec.ts` is in the explicit Playwright allowlist. At 1100px and 1440px it checks exact start-to-end DOM geometry, Whole document full length, blue stroke dimensions, unchanged transmitted slices, saved history's single marker with no lavender, integer flag distance and the one-screen threshold, scroll navigation, persisted visibility, and hiding below the desktop breakpoint. It verifies marker movement during idle author typing and concurrent Continue fragments/author edits. Four screenshots are saved to `target/guz-757/` (`1100-flag-hidden.png`, `1100-flag-visible.png`, `1440-flag-hidden.png`, `1440-flag-visible.png`); hidden-flag screenshots show the full bracket using a small window selected through Settings.

Run `cargo xtask e2e` from the workspace. On first use, install Chromium with `cd app && bun --bun x playwright install chromium`. The runner installs app dependencies if Playwright is missing, builds the API, creates a temporary on-disk SQLite database, and chooses random free loopback ports for the API and frontend. Use `cargo xtask e2e --port 3107` to select the frontend port explicitly, before any Playwright arguments. The runner passes the chosen frontend origin to API `--cors-origin`, the frontend port through `PORT`, and the frontend URL to Playwright through `QUILLAI_E2E_APP_URL`.

The runner waits for HTTP 200 from API `/series` and frontend `/`, then runs `bun --bun x playwright test` from `app/` with the config in root `e2e/`. Additional flags pass through unchanged: `cargo xtask e2e --headed --trace on`. Tests supply `apiBaseUrl` and chapter IDs on both page URLs. The chapter smoke checks sequential edits, convergence, saved revisions/content, and reload. Emoji coverage adds concurrent append/replacement/pending edits and six seeded runs of 50 rapid, codepoint-safe Quill clipboard pastes per tab with delayed network I/O; both tabs must converge with all inserted codepoints retained and no HTTP or console errors. AI browser coverage mocks responses to check Continue thread history, Stop, and pink-note Edit submission; it does not call a live AI provider. Browser tests live outside `app/` to keep `bun test` separate. `e2e/package.json` marks this directory as ESM for Bun's Playwright loader; imports reuse the dependency installed under `app/node_modules/`. Failure screenshots and traces are under `target/xtask-e2e/results/`.

Services, build/install commands, and Playwright run in managed process groups. Normal exit, startup/test failure, and Ctrl-C terminate those groups before removing the database directory and SQLite sidecars. The random API port is released before API bind; a competing bind fails startup rather than silently reusing another service. The chosen frontend port is reserved during setup and released immediately before frontend startup.

## Author Guidance And Locked System

The inspector shows System as a read-only textarea with no reset control. System is excluded from typed section overrides; the server rejects raw `sectionOverrides.system` with 422. Other section edits and history resets remain request-local.

`settings/client.ts` uses the shared API base URL, parses `{text: string}` responses, and exposes structured HTTP/protocol errors. The inspector loads saved author guidance from `GET /settings/author-guidance` on mount/reload, never localStorage. The separate Author guidance editor saves explicitly with PUT; empty text clears guidance, and failures keep the draft and display an error. Controls stay disabled until loading succeeds (reload to retry) and while saving or generating. Only saved text enters the live prompt preview. When nonempty, `prompt_sections.ts` adds `author_guidance` directly after System, with the same escaped tag/text as the server and its own token count. Unset guidance adds no section. Other tabs' saves become visible on reload; the server always reads its latest saved text for generation.

`cargo xtask e2e author-guidance.spec.ts` verifies System read-only/no reset, guidance save, escaped live context, server round-trip, reload, and clearing. It also checks moved control locations, the single Continue control's thread indicator and Stop behavior, and pink-note Edit selection handling. Settings coverage changes and reloads all three local preferences, checks dismissal and selection restoration, and forces a failed save to verify the red pill and export. Saved, gear-open, and failed-save screenshots at 1100px and 1440px go to `target/guz-758/`. Shared Rust/Bun fixtures cover guidance with and without replay and unchanged empty guidance.

## Continue And Edit Flow

Both actions use series lore loaded by `manuscripts/use_lore.ts` through the parsed, typed `lore_client.ts`. The left drawer's `LorePanel.tsx` provides CRUD with explicit author-only hidden notes, levels, and pin. Loading and failed states are visible with retry; series changes immediately remove previous entries. Optional caller-supplied `loreEntries` remain available for isolated tests. Assembly removes `leave_out` before matching/budgeting, matches only the transmitted document window, then projects name, kind, visible text, and level into `lore`. The matcher itself still carries every level unchanged. Its default request budget is 65,536 UTF-16 units of name + visible text, overridable with `maxLoreCharacters`; pins can exceed it. The backend independently enforces 65,536 UTF-8 bytes including rendered labels/markers, so oversize requests can return 422. Hidden notes and matching metadata never enter AI requests. Derived Lore is read-only to preserve the non-reveal marker. `lore_prompt.test.ts`, the shared parity fixture, backend HTTP tests, and the mocked Continue browser test cover request assembly.

`Continue`:

- One right-rail control, with a visible "with thread" badge when saved or overridden history will be sent. No separate Resume or inline Continue control. While either AI stream runs, the same control shows Stop and uses the existing abort/cancellation/rollback path. The badge retains the active request's history state after request-local overrides reset. `shouldContinueWithThread` and `continueClickAction` provide pure decisions covered by unit tests; the browser smoke mocks a length-limited response and an abortable stream. A `usage` event before `done` leaves the run active, and `finishReason: length` still shows Continue with thread.
- resolves the current caret or remembered caret
- builds a windowed request
- starts `fetch`
- applies streamed text-only delta fragments at the insertion anchor

`Edit`:
- Only the right-rail Edit selection control opens the pink sticky note; there is no middle Edit button. Enter and Apply share the selection, busy, chapter-ready, and blocked-save checks before starting the same Edit action. Losing the selection while the note is open prevents submission. Shift+Enter adds a newline, composing text does not submit, and Escape cancels the note without changing manuscript text.
- resolves the current non-empty selection or remembered pre-blur selection
- deletes the selected text when the stream starts
- inserts streamed text at the original anchor
- restores or rolls back the original text depending on cancellation or failure timing

## Cancellation And Failure Rules

- Edit and narrow-screen Continue make the editor read-only; desktop Continue permits concurrent author edits.
- Desktop Continue abort, cancellation, failure, and premature EOF remove surviving AI-owned text while retaining author edits and logical selection.
- Cancel before first inserted token restores the original selection.
- Failed edits roll the original text back into place.
- Narrow-screen Continue retains partial AI output on abort/failure; failed Edit restores its original selection text.

## Stream Contract Expectations

The frontend expects NDJSON events with Quill-delta payloads, but the editor shell currently accepts text inserts only from streamed `delta` events. Non-text embed inserts are rejected at the shell boundary, and request slices containing embeds are rejected before `fetch` starts. `ai_stream.ts` accepts a `usage` event with integer `promptTokens` and `completionTokens`. Continue and Edit ignore it and do not display token counts. The run still ends only on `done`, `error`, `cancelled`, or EOF, so `finishReason` on the following `done` is kept.

## Tests

```sh
bun test --cwd app
sh -lc 'cd app && bun x tsc --noEmit'
bun run --cwd app build
```

Pure helper coverage currently lives in:

- `ai_contract.test.ts`
- `model_catalog.test.ts` — public response parsing, missing/unknown saved ID fallback, and selected/omitted model request assembly.
- `editor_persistence.test.ts` — settings/thread round-trip including both model IDs and backward-compatible parsing.
- `prompt_parity.test.ts` — replays `fixtures/prompt/parity.json` with Rust `crates/api/tests/prompt_parity_test.rs`, checking section ids/tags/text, normalized history, resume/edit behavior, Unicode, and model-independent prompts against captured provider requests.
- `ai_stream.test.ts`
- `ai_state.test.ts`
- `window.test.ts`
- `delta_bridge.test.ts`
- `editor_ai.test.ts`
- `continue_control.test.ts` — effective history indication, including overrides, and Continue clicks stopping active streams.
- `editor_controls.test.ts` — shared Enter/Apply availability and quiet save labels with prominent failure/recovery messages.
- `editor_panels.test.ts` — drawer toggle/switch/close and shortcut modifier/composition handling. The chapter browser smoke checks Ctrl and Cmd inspector shortcuts, hover/click behavior, manuscript loading, and Escape selection/focus restoration, including from the guidance field.
- `chapter_sync.test.ts`
- `chapter_session.test.ts`

The second `chapter-sync.spec.ts` browser test seeds two chapters, checks save-before-switch ordering and the single detail GET, then checks failed-save text, current scope/highlight, and recovery retention.

There is still no direct component-level test for the full Quill streaming shell, so browser QA remains important for Continue/Edit trigger regressions.
