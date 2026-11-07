# E2E coverage for first-run QA (E9–E13)

Every child of GUZ-858 (E9), GUZ-864 (E10), GUZ-872 (E11), GUZ-885 (E12), and GUZ-898 (E13), plus GUZ-931 (already a child of GUZ-858). Linear workflow states as of 2026-10-25. The epics themselves are not rows.

Classes:

- **a** — browser-visible. The regression is an E2E spec. Open rows say `needed`.
- **b** — a Rust or Bun test is the coverage. The test column names the file and test.
- **c** — a decision, spike, or tooling change with nothing to test.

59 children: 48 class a (4 Done, 44 open), 2 class b, 9 class c.

GUZ-861 is class a. `cargo xtask e2e` keeps a single API process on a temporary file (`xtask/src/e2e.rs:136`) and does not pass that path, or a restart hook, to Playwright. `e2e/persistent-dev-database.spec.ts:168` starts `target/debug/api` itself, with no `--db-url` and no `QUILLAI_API_DB_URL`, in a temporary working directory, so the process uses the default `sqlite:quillai.db?mode=rwc`. `crates/api/tests/persistent_database_test.rs:172` `server_restart_keeps_created_series` restarts the API and checks a series row. It does not open the editor.

| Ticket | Title | State | Class | Test |
| --- | --- | --- | --- | --- |
| GUZ-859 | Dev build compiles without SQLX_OFFLINE | Backlog | c | nothing to test |
| GUZ-860 | Remove the cargo `xtask` alias shadowing warning | Backlog | c | nothing to test |
| GUZ-861 | Persistent dev database by default | Done | a | `e2e/persistent-dev-database.spec.ts:168` chapter text survives an API restart on the default file database |
| GUZ-862 | Writer-facing AI error and in-app AI setup | Backlog | a | `e2e/ai-not-configured.spec.ts` checks the model picker, Continue status and Edit alert for friendly setup errors |
| GUZ-1256 | AI settings in the app | Written, not run | a | `e2e/ai-settings.spec.ts` checks settings visibility, key source and removal, validation, key redaction, partial updates, and model picker refresh |
| GUZ-863 | Start the app without a long environment-variable command | Backlog | c | nothing to test |
| GUZ-865 | Edit writes extra text into the manuscript | Done | a | `e2e/edit-replacement.spec.ts:68` Edit keeps only the replacement and drops commentary around it; `e2e/edit-replacement.spec.ts:88` Edit with a missing replacement tag restores the selection, and Edit with a unclosed replacement tag restores the selection |
| GUZ-866 | Continue fails silently when the model spends its budget on reasoning | Backlog | a | needed |
| GUZ-867 | Continue and Edit share one Instruction box | Done | a | `e2e/author-guidance.spec.ts:249` Edit and Continue send only their own instructions |
| GUZ-868 | Define what an AI Thread is for writers | Backlog | c | nothing to test |
| GUZ-869 | Continue mid-sentence carries on inline | Backlog | a | needed |
| GUZ-870 | Stop Continue repeating the text before the caret | Backlog | a | needed |
| GUZ-871 | Spike: score AI output against the request with a classifier | Backlog | c | nothing to test |
| GUZ-873 | Settings as a Popover with no Close button | Backlog | a | needed |
| GUZ-874 | Core primitives: Button, IconButton, TextField, Toggle, Field, Heading, Alert, StickyNote | Done | a | `e2e/ui-components.spec.ts:15` ui primitives render at 1100px and 1440px |
| GUZ-875 | Edit note as a pink overhanging StickyNote | Backlog | a | needed |
| GUZ-876 | Migrate rail buttons to IconButton | Backlog | a | needed |
| GUZ-877 | Design tokens, design doc and `check_ui` lint gate | Done | b | `xtask/src/check_ui.rs:278` tsx_counts_raw_controls_and_brackets; `xtask/src/check_ui.rs:301` css_counts_hex_colours_and_font_sizes; `xtask/src/check_ui.rs:343` compare_fails_new_and_excess_violations |
| GUZ-878 | Modal and Popover primitives | Backlog | a | needed |
| GUZ-879 | Remove dead styles and empty the allowlist | Backlog | a | needed |
| GUZ-880 | Alert for save pill and errors, Toggle for checkboxes | Backlog | a | needed |
| GUZ-881 | Inspector and lore fields use TextField, StickyNote and Heading | Backlog | a | needed |
| GUZ-882 | Lore editing in a Modal with confirmed red Delete | Backlog | a | needed |
| GUZ-883 | API tester: mount through primitives or delete | Backlog | c | nothing to test |
| GUZ-884 | ListButton for the manuscript tree and lore-window rows | In Progress | a | needed |
| GUZ-886 | Rename the "Retry manuscript" action | Backlog | a | `e2e/tree-load-retry.spec.ts` checks the approved label and recovery after one failed tree fetch at 1100px and 1440px; written, not run |
| GUZ-887 | Centre the chapter title | Backlog | a | needed |
| GUZ-888 | Remember caret and scroll position per chapter | Backlog | a | `e2e/caret-restore.spec.ts` checks switch and reload restoration before the first populated frame |
| GUZ-889 / GUZ-1236 / GUZ-1238 / GUZ-1239 / GUZ-1240 | Chapter and book exports and backup | Written, not run | a | chapter-export.spec.ts checks book order, unsaved text, Markdown, text, Word and EPUB downloads, and failure |
| GUZ-890 | Favicon and a real tab title | Backlog | a | needed |
| GUZ-891 | Create chapters, books and series from the app | Backlog | a | needed |
| GUZ-892 | Standalone stories outside books and series | Backlog | a | needed |
| GUZ-893 | Empty state as an alert with an inline create link | Backlog | a | `e2e/empty-book.spec.ts` checks the notice and chapter creation at 1100px and 1440px; written, not run |
| GUZ-894 | Landing page | Backlog | a | needed |
| GUZ-895 | Rename series, books and chapters; optional chapter numbers | Backlog | a | `e2e/tree-rename.spec.ts` covers rename only at 1100px and 1440px; written, not run. Chapter numbers out of scope |
| GUZ-896 | No flash when switching chapters | Backlog | a | `e2e/chapter-switch-no-flash.spec.ts` checks fast switches and creation, plus slow-load status |
| GUZ-897 | Remove the "Opening a chapter saves current changes first." hint | Backlog | a | `e2e/lore.spec.ts:163` checks the hint is absent after the chapter button is visible in the Manuscript outline |
| GUZ-899 | Tint edited text for a while after Edit | Backlog | a | needed |
| GUZ-900 | Explain raw "sqlx error" messages | Backlog | a | needed |
| GUZ-901 | Keyboard shortcuts for Continue and Edit | Backlog | a | needed |
| GUZ-902 | Read-only view of the full assembled prompt | Backlog | a | needed |
| GUZ-903 | Standard notification system | Backlog | a | needed |
| GUZ-904 | Keep the selection highlighted while the Edit note is open | Backlog | a | needed |
| GUZ-905 | Replace 1 s polling with server-sent events | Backlog | a | needed |
| GUZ-906 | Inspector Selection box empty while text is selected | Backlog | a | needed |
| GUZ-907 | Decision: make inspector Document read-only | Backlog | c | nothing to test |
| GUZ-908 | Decision: hard delete or tombstone | Backlog | c | nothing to test |
| GUZ-909 | Better rendering of saved lore entries | Backlog | a | needed |
| GUZ-910 | Plan for many lore matches | Backlog | c | nothing to test |
| GUZ-911 | Clicking a lore entry cycles through its matches | Backlog | a | needed |
| GUZ-912 | AI completion in lore text fields | Written, not run | a | `e2e/lore-complete.spec.ts` checks caret insert, request body, field scope, Stop, rollback, 503, Escape, the model setting and Leave out |
| GUZ-913 | Explain "Pin in AI context" | Backlog | a | needed |
| GUZ-914 | Word, character and line counts | Written, not run | a | `e2e/text-counts.spec.ts` checks document, selection and emoji counts |
| GUZ-915 | Hint for lore keywords | Backlog | a | needed |
| GUZ-916 | Better support for writing dialogue | Backlog | a | needed |
| GUZ-917 | Check lore hover card in Firefox | Backlog | a | needed. Hover card: `e2e/lore.spec.ts:88` inside `e2e/lore.spec.ts:52` desktop lore highlights, cards and Continue window are visual only (runs in the firefox project). The new affordance has no spec |
| GUZ-919 | Edit instruction note text can't be selected | Backlog | a | needed |
| GUZ-920 | Edit fails with glm-5.3-flash (no <replacement> tag) | Backlog | a | needed |
| GUZ-921 | Writer-facing Edit failure message | Backlog | a | needed |
| GUZ-931 | Dev database opens relative to the start folder | Done | b | `xtask/src/dev_database.rs:33` default_url_is_the_workspace_file_and_creates_nothing; `xtask/src/dev_database.rs:51` explicit_url_wins_over_the_workspace_default |

E10 children GUZ-920 and GUZ-921 are listed with the other tickets in identifier order. GUZ-931 is a child of GUZ-858.

## Open class (a) tickets

These tickets need this acceptance line: `ships with an E2E regression spec`.

- GUZ-862 Writer-facing AI error and in-app AI setup
- GUZ-866 Continue fails silently when the model spends its budget on reasoning
- GUZ-869 Continue mid-sentence carries on inline
- GUZ-870 Stop Continue repeating the text before the caret
- GUZ-873 Settings as a Popover with no Close button
- GUZ-875 Edit note as a pink overhanging StickyNote
- GUZ-876 Migrate rail buttons to IconButton
- GUZ-878 Modal and Popover primitives
- GUZ-879 Remove dead styles and empty the allowlist
- GUZ-880 Alert for save pill and errors, Toggle for checkboxes
- GUZ-881 Inspector and lore fields use TextField, StickyNote and Heading
- GUZ-882 Lore editing in a Modal with confirmed red Delete
- GUZ-884 ListButton for the manuscript tree and lore-window rows
- GUZ-887 Centre the chapter title
- GUZ-889 Real exports instead of the recovery dump
- GUZ-890 Favicon and a real tab title
- GUZ-891 Create chapters, books and series from the app
- GUZ-892 Standalone stories outside books and series
- GUZ-894 Landing page
- GUZ-899 Tint edited text for a while after Edit
- GUZ-900 Explain raw "sqlx error" messages
- GUZ-901 Keyboard shortcuts for Continue and Edit
- GUZ-902 Read-only view of the full assembled prompt
- GUZ-903 Standard notification system
- GUZ-904 Keep the selection highlighted while the Edit note is open
- GUZ-905 Replace 1 s polling with server-sent events
- GUZ-906 Inspector Selection box empty while text is selected
- GUZ-909 Better rendering of saved lore entries
- GUZ-911 Clicking a lore entry cycles through its matches
- GUZ-912 AI completion in lore text fields
- GUZ-913 Explain "Pin in AI context"
- GUZ-914 Word, character and line counts
- GUZ-915 Hint for lore keywords
- GUZ-916 Better support for writing dialogue
- GUZ-917 Check lore hover card in Firefox
- GUZ-919 Edit instruction note text can't be selected
- GUZ-920 Edit fails with glm-5.3-flash (no <replacement> tag)
- GUZ-921 Writer-facing Edit failure message
