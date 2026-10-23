# Eval set

The eval set is one exported book plus a case file of cut points. The book is a private manuscript: keep it in `eval-fixtures/` (gitignored) and never commit it.

## Export

Start the API, then export one book through the chapter API (`GET /series/{id}/books/{book_id}/chapters?content=true`):

```sh
cargo run -p quillai_api --bin eval -- export --api http://127.0.0.1:2908 --series 1 --book 2 --out eval-fixtures/book-2
```

`--api` defaults to `http://127.0.0.1:2908`. The command writes `<out>/book.json`:

```json
{
  "series_id": 1,
  "book_id": 2,
  "chapters": [
    {"id": 7, "title": "One", "position": 0, "kind": "chapter", "head_rev": 12,
     "content": {"ops": [{"insert": "Hi 😀 there\n"}]}, "text": "Hi 😀 there\n"}
  ]
}
```

Chapters keep the API order (position, then ID). `content` is the rebuilt head Delta; `text` joins its text inserts and drops embeds.

The exporter runs `git check-ignore` on `--out` before any request. It refuses a path that git does not ignore, and a path outside any git repository.

## Case file

A JSON array. Each case is a cut point in one chapter of the fixture:

```json
[
  {"chapter": 7, "offset": 3, "action": "continue"},
  {"chapter": 7, "offset": 5, "action": "edit", "instruction": "Tighten the prose."}
]
```

| Field | Type | Rule |
|-------|------|------|
| `chapter` | integer | Chapter `id` in `book.json` |
| `offset` | integer | UTF-16 offset into that chapter's `text`; `0` through its length, never inside a surrogate pair |
| `action` | string | `continue` or `edit` |
| `instruction` | string | Optional; defaults to empty |

Unknown fields are rejected. Check a case file against a fixture:

```sh
cargo run -p quillai_api --bin eval -- check --fixture eval-fixtures/book-2 --cases eval-fixtures/book-2/cases.json
```

Errors name the case index (from 0), e.g. `case 1: offset 7 is past the end of chapter 8 (6 UTF-16 units)`.

Code: `crates/api/src/eval/` (`fixture.rs` and `cases.rs` are pure; `export.rs` does HTTP and file writes), CLI in `crates/api/src/bin/eval.rs`, tests in `crates/api/tests/eval_export_test.rs`.
