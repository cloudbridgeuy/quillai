# Sync change nudges over SSE

Status: proposed design spike, option A, GUZ-905. No runtime change.
Evidence base: `main` at `41dfe1707702b4d63b957514c51b84191431ef2f`.
Source references below use that revision. “Measured” means source inspection;
browser timing and new behavior remain proposed, not measured.

## Decision

Use server-sent events (SSE) to announce “piece X is now at revision N”.
Treat each announcement as a change nudge, never as document content or an ack.
The browser still fetches the existing delta history and applies its existing OT.
Keep save, retry, uncertain-submit recovery, AI reservation, and flush semantics.
Replace idle one-second reads with a 15-second safety poll plus immediate nudges.
Keep the current one-second cadence for pending saves and retry work.
Add no dependency, schema, replay log, or durable notification service.

## Measured current flow

- Chapter timer: `app/src/md_editor/use_chapter_sync.ts:151-153` calls `tick()`
  every 1000 ms. Edits call `session.edit()`, not an immediate network save
  (`use_chapter_sync.ts:232-238`).
- `app/src/md_editor/chapter_session.ts:84-92` skips a tick while running,
  reserved, stopped, or blocked. `synchronize()` posts pending work and then
  reads history (`chapter_session.ts:152-193`). Idle ticks also issue a GET.
- `chapter_session.ts:240-251` flushes until local changes are saved.
  Switch and leave await flush (`use_chapter_sync.ts:117-132,180-197`).
  AI wait has its own clean-state loop and calls `runSync()` directly
  (`chapter_session.ts:205-222`), including history GET/application
  (`chapter_session.ts:183-190`). Preserve both paths.
- Screenplay has a second 1000 ms loop (`use_script_sync.ts:202-204`).
  `app/src/md_editor/script_poll.ts:8-51` batches scene session reads into
  `GET /pieces/{groupId}/scenes/deltas?since=<sceneId>:<rev>,…`.
  Script flush and leave use all scene sessions (`use_script_sync.ts:330-345,412-417`).
- Chapter and piece GET handlers use `leaf_sync::history`
  (`crates/api/src/manuscripts/chapters/sync.rs:28-37`,
  `crates/api/src/manuscripts/pieces/handlers.rs:257-265`). It reads the head
  and missed revisions in one transaction (`manuscripts/leaf_sync.rs:13-29`).
- Screenplay GET uses `chapter_revisions::missed` inside its group transaction
  (`manuscripts/pieces/scenes.rs:66-83,103-126`).
- The revision INSERT is `crates/api/src/chapter_revisions.rs:100`.
  `leaf_sync::submit` transforms against history and calls `RevisionStore::append`
  (`manuscripts/leaf_sync.rs:31-53`). Append commits at `chapter_revisions.rs:130`.
  No in-process notification is present in this path today.
- State currently passes a `SqlitePool` through routers
  (`crates/api/src/lib.rs:38-50`, `manuscripts/mod.rs:16-24`).
  This proposal needs a shared notification sender as well as that pool.
- Dependencies already exist: Axum 0.8.3, `async-stream`, and `futures-util`
  (`crates/api/Cargo.toml:28-36`); Tokio has `full` (`Cargo.toml:36`).
  Local Axum 0.8.3 source confirms `response::sse` with default features
  (`axum-0.8.3/Cargo.toml:80-90`, `src/response/sse.rs:63-90`).
  `Sse::new` has no heartbeat until `.keep_alive(...)` is selected explicitly.

## Endpoint and wire format

Choose one EventSource per editor tab, multiplexing its active leaf piece IDs:

`GET /sync/events?pieceIds=12,13,14`

For prose the list has one piece ID; for screenplay it has all loaded scene IDs.
Legacy chapters use `GET /series/{id}/books/{book_id}/chapters/{chapter_id}/events`,
an alias for the same stream/filter with server-side `chapter_piece` resolution
(`manuscripts/chapters/sync.rs:14-23`). Never treat chapterId as pieceId: the IDs
are separate (`migrations/20261025000000_story_model.up.sql:230-231`).
Use the same API base URL and origin rules as reads.
Validate IDs, duplicates, active leaf existence, and access under the same rules
as delta GETs before streaming. Do not expose unrelated piece IDs or revisions.
Start with a bounded list of 256 IDs and a 4096-byte query; reject excess input.
For larger scripts use the existing fallback and record the limit for revision.

Example frame after piece 12 commits revision 8:

```text
event: changed
data: {"pieceId":12,"rev":8}

```

Both endpoints send `event: ready` with `data: {"pieceIds":[12,13,14]}` after
subscription; the legacy alias returns its sole resolved ID. Its shell maps that
ID to the chapter session, without changing existing GET payloads or adding a lookup.
Send `event: resync` with `data: {}` after receiver lag. Both events request refetch
of all subscribed sessions; native `open` alone does not supply the resolved IDs.
No document operations, user text, client save ID, or acknowledgement travel here.
Use `Content-Type: text/event-stream`, no response caching, and disable proxy
buffering where applicable. Configure an Axum comment keep-alive every 15 seconds.
Comments have no revision meaning. They are separate from the fallback timer.

Choose refetch on every connection, including automatic reconnect; omit `id:`.
There is no replay guarantee and no use of `Last-Event-ID` in this version.
The GET cursor remains each session's committed revision. This avoids inventing
a global ordering across pieces. Native EventSource owns reconnect, with a
one-second `retry:` hint; the 15-second poll remains independent of reconnect.

Subscribe before emitting `ready`. The subsequent history GET covers changes
before subscription; buffered nudges cover changes during that GET.
When loaded screenplay IDs change, reopen with the new list and refetch.
Group GETs detect membership changes on the next eligible fallback read and
retain the existing “Scenes changed; reload” notice. AI reservation can defer
that read. Structural changes without a revision are outside the push guarantee.

Per-scene streams consume many connections; group routes need membership filtering.

## Server implementation boundary

Add one `tokio::sync::broadcast` sender to manuscript application state, shared
by legacy chapter routes, piece routes, and the stream handler. Keep unrelated
routers on their existing pool state where practical. Do not create a sender
per request or per router: it must cover every successful submit in the process.
Use a small typed `{ piece_id, rev }` record and a bounded ring, initially 256.

Publish once in the shared `leaf_sync::submit` success path after append returns;
append has already committed. Publish nothing for stale-head retries, rollback,
or rejected submissions. All HTTP submit paths must use this same sender.
No receiver, disconnect, or failed send can turn a committed save into an error.
Keep revision storage independent of notification delivery and do not await clients.
Runtime revision writes through any future new path must use the same boundary.

Creation and trashing do not append revisions and remain outside this guarantee.
Piece creation stores `EMPTY_DELTA` at revision 0
(`manuscripts/pieces/handlers.rs:69-85,184-198`); legacy chapter creation stores
the supplied content at revision 0 (`manuscripts/chapters/handlers.rs:91-106`).
Neither needs a nudge for the new ID: no subscriber can have loaded it yet.
Trashing only changes `deleted_at` (`manuscripts/pieces/trash.rs:5-20`). A later
leaf history GET fails active-piece lookup (`manuscripts/leaf_sync.rs:19-25`)
with 404 (`crates/api/src/error.rs:28-34`). A grouped screenplay read can instead
reject the removed scene (`manuscripts/pieces/scenes.rs:110-116`). HTTP failures
become `ManuscriptApiError` (`app/src/manuscripts/client.ts:78-87`), which existing
`rememberFailure` marks blocked (`chapter_session.ts:99-108`). Preserve this
existing behavior; deletion nudges and a revised deleted-editor flow need follow-up.

Each stream filters notices to its validated ID set. A slow receiver cannot
block writers. On `RecvError::Lagged`, emit one `resync` and continue receiving;
coalesce repeated resyncs in the client until its current read completes.
History GET, not broadcast retention, repairs any skipped revisions.
`watch` could coalesce a single piece's latest head, but multiplexed IDs would
need a map or per-piece channels; broadcast is the smaller first implementation.

Scope is one API process. Another process writing the same database, a restart,
or the crash window between commit and publish can lose a nudge. Reconnect and
the 15-second fallback repair this. Multiple replicas require a shared bus or
database notification source in a separate design; this note does not promise
sub-second propagation across processes.

## Client ownership and scheduling

Add `sync_push.ts` beside `chapter_session.ts`. A pure state machine consumes
connection, revision, busy, reservation, and timer events; returns open, close,
and request-tick decisions. A thin shell owns EventSource, listeners, and timers.
Use one shell for the current scope. Keep OT and delta reads in sessions and `script_poll.ts`.

Expose read-only session scheduling facts: known revision, pending/retry work,
busy/reserved state, and completion/release notification. Do not change the
algorithm inside submit, recover, flush, or reservation wait to add transport.
The shell calls the existing tick; screenplay calls its grouped tick for all
eligible scene sessions, even for one nudge. Keep child-list reconciliation.

Track the maximum advertised revision per subscribed piece plus a refetch flag.
Drop a nudge at or below the session's confirmed revision. Never advance that
revision from SSE. When our own POST succeeds, its existing subsequent GET and
ack reconciliation normally cover our nudge. Recheck the maximum after completion
and clear covered entries. Do not discard an event merely because a POST is ours:
it can race its response or accompany another writer's revision.

If a session tick is running, retain one pending request rather than losing the
nudge to its early return (`chapter_session.ts:84-87`). Retain it across grouped
screenplay rounds too: `script_poll.ts:42-49` returns while `round !== null`.
After the round clears (`script_poll.ts:24-27`) and sessions complete, tick once
more only if the maximum revision is ahead or an unconditional refetch is due.
This also covers a commit that occurs after the in-flight GET took its snapshot.
Failures keep the one-second retry cadence and existing blocked rules; queued
nudges must never cause a tight retry loop or bypass the blocked state.

During an AI reservation, defer only nudge-triggered ticks until release.
The existing clean-state wait directly calls `runSync()` while reserved
(`chapter_session.ts:205-222`) and can fetch/apply remote deltas before streaming;
leave that wait unchanged. On release, run one queued tick with current revisions.
Do not interleave nudge-triggered remote application with the AI stream.
Script reservations cover all scene sessions
(`use_script_sync.ts:423-439`); do not let a group nudge bypass that exclusion.

On successful switch or `leave()`, close the old source after existing flush
succeeds, then open the next scope if any. A failed flush keeps the active scope.
On hook cleanup and `pagehide`, close immediately and cancel transport scheduling.
Do not add async flush during page teardown. On `pageshow` after back-forward
cache restore, reopen and refetch. Existing beforeunload dirty warnings remain
(`use_chapter_sync.ts:151-160`); pagehide transport handling is new work.

Treat source errors during teardown as expected cancellation. No save alert
comes from EventSource alone. WebKit can report reload fetch cancellation as
“due to access control checks”; existing narrow allowance is at
`e2e/landing.spec.ts:28-29` (GUZ-1163). Keep cleanup idempotent and retain that
allowance; do not suppress CORS errors during an active editor session.

## Fallback and save timing

Use a 15-second periodic history tick while connected AND disconnected.
Start a fresh deadline on scope activation; ready/reconnect requests an immediate
coalesced read and resets that deadline. Errors cause no immediate read or deadline
reset: repeated reconnect failures must not create one-second idle GETs or starve
fallback. If EventSource is unavailable or rejected, stay on this fallback.

Separately retain a one-second scheduler for local pending work, unresolved
flight/ack recovery, and retrying errors. Skip it for a clean idle session.
In screenplay mode, use grouped reads for these ticks too. A save still runs
the existing POST-then-GET cycle; that GET is allowed even inside the 15 s window.
Flush remains immediate when required. Changing the whole existing timer to
15 seconds would delay saves and recovery; that is explicitly not this design.

## What counts as a real change

Measured: `chapter_sync.ts:29-33` computes `diffDocuments` against server plus
in-flight content, so a document equal to that baseline has no pending delta.
`app/src/md_editor/components/Editor.tsx:1487-1488` unconditionally delegates
onTextChange to `chapterSync.documentChanged`. The chapter and script hooks guard
that method (`use_chapter_sync.ts:232-234`, `use_script_sync.ts:443-445`).
Their remote applications set `applyingRef` around Quill updates
(`use_chapter_sync.ts:86-103`, `use_script_sync.ts:137-156`).
This prevents remote replay from becoming a local save; origin alone does not define a real edit.

Measured statically: `document_diff.ts:4-25` preserves attribute differences;
format-only changes can therefore produce pending document deltas. AI inserts
use Quill source `api` (`Editor.tsx:950-953`) without the remote-apply guard and
are intended to persist. No new browser experiment was run for either case.
The guards do not prove a strict “plain text characters only” policy.
Proposed interpretation: real document changes include formatting and AI text.
No save guard fix is justified by this inspection. Dropping either would risk
lost formatting or AI output. T2 should test identical-document no-op, remote
replay no echo, formatting save, and AI api insert save before changing guards.

## Existing E2E impact

Measured census: 26 `e2e/*.spec.ts` files contain `/deltas`. Impact below is proposed, not executed.
References are relative to `e2e/`. Keep means retain the behavior assertion.

| Spec | Proposed impact and source evidence |
| --- | --- |
| `chapter-sync.spec.ts` | Keep POST fault fixtures (`:181,208`); save cadence stays 1 s. |
| `save-recovery.spec.ts` | Keep failed/held POST and recovery (`:45,50,76-78`). |
| `alerts-toggles.spec.ts` | Keep delta POST failure/alert (`:51-64`); SSE errors create no save alert. |
| `author-guidance.spec.ts` | Keep POST failure fixture (`:396-399`). |
| `reload-in-flight.spec.ts` | Keep held POST/reload assertions (`:53,93-96`); add stream cleanup coverage. |
| `offline-stale-base.spec.ts` | Edit offline fixture (`:52-53,103-109`): also disconnect/fake SSE, so remote nudges cannot defeat simulated offline reads. |
| `continue-poll.spec.ts` | Edit delta GET fixture (`:43,50`): new fake drives deterministic nudge during AI; retain deferred remote-apply assertion. |
| `continue-unsaved.spec.ts` | Edit GET fixture (`:36,45`): drive recovery explicitly or with fake/fallback clock; retain unsaved-Continue contract. |
| `screenplay-scroll.spec.ts` | Keep delta response filter (`:74-76`), API seeds (`:91-92`), remote edit (`:168`), and POST-only failure fixture (`:279-280,288`); verify nudge preserves scroll. |
| `screenplay-cross-scene.spec.ts` | Keep POST observation, API seeds, and scene GET wait (`:71,85-87,99`). |
| `chapter-emoji-sync.spec.ts` | Keep POST fault fixtures and request log (`:67,207,218`). |
| `emoji-cursor-remote.spec.ts` | Keep real remote saves and cursor checks (`:100,125`). |
| `real-route-stream.spec.ts` | Keep real remote POST (`:83`). |
| `stream-remote-edit.spec.ts` | Keep real remote POST (`:94`). |
| `continue-tie-order.spec.ts` | Keep delta/remote-save checks (`:38,90`). |
| `screenplay-ai.spec.ts` | Edit poll-count timing during AI (`:58`); keep POST/API edit behavior (`:64,212`), use fake for reserved nudge. |
| `screenplay-format.spec.ts` | Keep API seed/mutation and formatting checks (`:72-73,174`). |
| `screenplay-notice.spec.ts` | Keep API seed (`:85-86`). |
| `screenplay-new-scene.spec.ts` | Keep API seed (`:80-83`); membership still has fallback. |
| `screenplay-fdx.spec.ts` | Keep API seed (`:34-35`). |
| `container-trash.spec.ts` | Keep API seed (`:25,80,84`). |
| `notices.spec.ts` | Keep API seed (`:39-40`). |
| `piece-trash.spec.ts` | Keep API seed and explicit deleted-history GET (`:21,23,53,71-72`). |
| `lore-scope.spec.ts` | Keep API seed (`:25`). |
| `piece-move-lore.spec.ts` | Keep API seed (`:27`). |
| `tree-delete.spec.ts` | Keep held chapter-save POST (`:148`). |

Add a shared EventSource fake for deterministic ready, changed, resync, error,
reconnect, close, and duplicate events. Install before page scripts in fixture
tests; keep their delta GET/POST interception. Native SSE integration tests must
use the real API and stream, so a fake cannot hide a missing server publication.

## New regression gates

- `e2e/sync-push-chapter.spec.ts`: idle open chapter sends zero delta GETs for
  5 seconds after initial ready/refetch settles. Main fails due to its 1 s loop.
  Then API-submit another writer's delta and show it in the open editor within
  1 second of POST completion, with a real stream connected before the write.
- `e2e/sync-push-screenplay.spec.ts`: same idle and remote-update assertions
  for a non-active scene; assert one stream and batched scene delta GETs.
  Both proposed specs run at widths 1100 and 1440 px, height 1000 px.
- Timing alone can occasionally pass on main's poll. Require native ready and
  freeze the fallback timer with the test clock for the remote-write assertion;
  keep network delivery live. Main lacks the stream and cannot pass this gate.
- Cover fake-driven own-save de-duplication, nudge during held POST/GET/group round, nudge
  during AI reservation, reconnect refetch, lag/resync, 15 s offline fallback,
  failed flush retaining scope, pagehide close, and pageshow reopen in those specs.
  Use clock advancement for 15 s cases, not wall-clock sleeps.
- T2/T3: trash a subscribed leaf; assert no revision nudge and preserve blocked
  handling on the next eligible history read (404 for a leaf, removed-scene
  rejection for a group). Track the deleted-editor flow as separate follow-up.
- T1 Rust tests: committed submit emits correct ID/rev; failed submit emits none;
  validate/filter subscriptions and legacy alias scope/resolved IDs; lag emits resync; reconnect race and
  disconnect cleanup. Test a nudge followed by GET sees the committed revision.
  Run changed specs in Chromium; owner runs Firefox/WebKit reload coverage.

## Delivery order and risks

1. T1, server stream plus Rust tests: M, about 1–2 days. Shared sender, bounded
   multiplexed endpoint, heartbeat, after-commit publication. Existing clients
   keep polling, so server deployment can go first. Risk: missing a submit path,
   accidental cross-piece disclosure, buffering, unbounded slow consumers.
2. T2, chapter client plus fallback: M, about 2 days. Pure scheduler/thin shell,
   dirty-work cadence, lifecycle, fake, and chapter regression. Risk: delayed
   saves, lost in-flight nudge, own-save echo, AI race, WebKit reload noise.
3. T3, screenplay client: M, about 1–2 days after T2. Reuse transport, preserve
   grouped reads/reservations, membership reopening, screenplay regression.
   Risk: connection explosion, missed scene IDs, stale membership, scroll jumps.
4. T4, optional obsolete poll-spec cleanup: S, half a day after T2/T3. Replace
   timer-specific fixtures only after equivalent AI/recovery regressions pass.
   Risk: deleting behavior coverage with a misleading “poll” test name.

For rollback, disable the client stream scheduler and restore the existing
one-second trigger; the server endpoint is additive and delta contracts stay.
Open owner questions, non-blocking for this spike: confirm the real-document
interpretation of “text changes”; confirm the proposed 256-ID ceiling and whether
multiple API processes or immediate structural notifications are future needs.

## Rejected alternatives

Full delta push over SSE duplicates history response semantics, gap repair, and
ack ordering. Nudges reuse GET/OT and recover through existing history.

WebSockets add a bidirectional protocol for server-to-browser invalidation.
HTTP POST saves already handle errors and recovery. SSE uses current dependencies.

Keeping one-second polling preserves simple operation but spends idle requests
for every open tab and scene group. Slowing polling alone raises remote-edit
latency. Keep it only as the 15-second repair path behind prompt nudges.
