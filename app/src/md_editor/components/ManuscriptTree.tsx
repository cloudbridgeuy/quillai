import { useEffect, useLayoutEffect, useReducer, useRef, useState } from "react";
import { Button } from "../../ui/Button";
import { ListButton } from "../../ui/ListButton";
import { TextField } from "../../ui/TextField";
import { Alert } from "../../ui/Alert";
import { createManuscriptClient } from "../../manuscripts/client";
import type { ChapterScope } from "../../manuscripts/contract";
import { createTitle, draftTransition, targetPosition, type CreateTarget, type ManuscriptSeries } from "../../manuscripts/create_draft";
import { treeScrollTarget } from "./tree_scroll";

type TreeState =
  | { status: "loading" }
  | { status: "failed"; message: string }
  | { status: "ready"; series: ManuscriptSeries[] };

export function ManuscriptTree({ selectedChapter, disabled, onSelect }: {
  selectedChapter: ChapterScope | null;
  disabled: boolean;
  onSelect: (scope: ChapterScope) => void;
}) {
  const [client] = useState(createManuscriptClient);
  const [state, setState] = useState<TreeState>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [draft, dispatch] = useReducer(draftTransition, null);
  const submitting = useRef(false);
  const treeRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const tree = treeRef.current;
    const container = tree?.parentElement;
    const row = tree?.querySelector<HTMLElement>('[aria-pressed="true"]');
    if (!container || !row) return;
    for (let ancestor = row.parentElement; ancestor && ancestor !== container; ancestor = ancestor.parentElement) {
      if (ancestor instanceof HTMLDetailsElement) ancestor.open = true;
    }
    const viewportTop = container.getBoundingClientRect().top + container.clientTop;
    const bounds = row.getBoundingClientRect();
    const series = row.closest("details")?.parentElement?.closest("details");
    const blockBottom = series?.lastElementChild?.getBoundingClientRect().bottom ?? bounds.bottom;
    container.scrollTop = treeScrollTarget(container.scrollTop, bounds.top, bounds.bottom, viewportTop, viewportTop + container.clientHeight, blockBottom);
  }, [state.status, selectedChapter?.seriesId, selectedChapter?.bookId, selectedChapter?.chapterId]);

  async function create() {
    if (!draft || disabled || submitting.current || state.status !== "ready") return;
    const title = createTitle(draft.title);
    dispatch({ type: "submit" });
    if (!title) return;
    submitting.current = true;
    try {
      const target = draft.target;
      const input = { title, position: targetPosition(state.series, target) };
      if (target.kind === "series") {
        const series = await client.createSeries(input);
        setState({ status: "ready", series: [...state.series, { ...series, books: [] }] });
      } else if (target.kind === "book") {
        const book = await client.createBook(target, input);
        setState({ status: "ready", series: state.series.map(series => series.id === target.seriesId
          ? { ...series, books: [...series.books, { ...book, chapters: [] }] } : series) });
      } else {
        const chapter = await client.createChapter(target, { ...input, kind: "chapter" });
        setState({ status: "ready", series: state.series.map(series => series.id === target.seriesId
          ? { ...series, books: series.books.map(book => book.id === target.bookId
            ? { ...book, chapters: [...book.chapters, chapter] } : book) } : series) });
        onSelect({ seriesId: target.seriesId, bookId: target.bookId, chapterId: chapter.id });
      }
      dispatch({ type: "created" });
    } catch (error) {
      dispatch({ type: "failed", message: error instanceof Error ? error.message : "Could not create manuscript item." });
    } finally {
      submitting.current = false;
    }
  }

  function createAction(target: CreateTarget) {
    const active = draft && JSON.stringify(draft.target) === JSON.stringify(target);
    const label = target.kind[0]!.toUpperCase() + target.kind.slice(1);
    return active ? <form data-manuscript-draft onSubmit={event => { event.preventDefault(); void create(); }}
      onKeyDown={event => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          dispatch({ type: "cancel" });
        }
        if (event.key === "Enter" && event.nativeEvent.isComposing) event.preventDefault();
      }}>
      <TextField square autoFocus aria-label={`${label} title`} placeholder={`${label} title`} value={draft.title}
        disabled={disabled || draft.pending} aria-invalid={Boolean(draft.error)}
        onChange={event => dispatch({ type: "title", title: event.target.value })} />
      {draft.error ? <Alert tone="inline">{draft.error}</Alert> : null}
      {draft.pending ? <p role="status">Creating {target.kind}…</p> : null}
    </form> : <ListButton tone="tree" disabled={disabled || Boolean(draft?.pending)}
      onClick={() => dispatch({ type: "start", target })}>+ {label}</ListButton>;
  }

  useEffect(() => {
    let active = true;
    setState({ status: "loading" });
    async function load() {
      const series = await client.listSeries();
      return Promise.all(series.map(async (item) => {
        const books = await client.listBooks({ seriesId: item.id });
        return {
          ...item,
          books: await Promise.all(books.map(async (book) => ({
            ...book,
            chapters: await client.listChapters({ seriesId: item.id, bookId: book.id }),
          }))),
        };
      }));
    }
    load().then((series) => {
      if (active) setState({ status: "ready", series });
    }).catch((error: unknown) => {
      if (active) setState({ status: "failed", message: error instanceof Error ? error.message : "Could not load manuscript." });
    });
    return () => { active = false; };
  }, [client, attempt]);

  if (state.status === "loading") return <p role="status">Loading manuscript…</p>;
  if (state.status === "failed") return <div role="alert">
    <p>{state.message}</p>
    <Button onClick={() => setAttempt((value) => value + 1)}>Retry manuscript</Button>
  </div>;

  return <>
    <p className="editor-note">Opening a chapter saves current changes first.</p>
    <div ref={treeRef} className="editor-manuscript-tree">
    {createAction({ kind: "series" })}
    {state.series.length === 0 ? <p>No series yet.</p> : <ul>
      {state.series.map((series) => <li key={series.id}>
        <details open>
          <summary>{series.title}</summary>
          {series.books.length === 0 ? <p>No books yet.</p> : <ul>
            {series.books.map((book) => <li key={book.id}>
              <details open>
                <summary>{book.title}</summary>
                {book.chapters.length === 0 ? <p>No chapters yet.</p> : <ul>
                  {book.chapters.map((chapter) => <li key={chapter.id}>
                    <ListButton tone="tree" disabled={disabled || Boolean(draft?.pending)}
                      pressed={selectedChapter?.seriesId === series.id && selectedChapter.bookId === book.id && selectedChapter.chapterId === chapter.id}
                      onClick={() => onSelect({ seriesId: series.id, bookId: book.id, chapterId: chapter.id })}>{chapter.title}</ListButton>
                  </li>)}
                </ul>}
                {createAction({ kind: "chapter", seriesId: series.id, bookId: book.id })}
              </details>
            </li>)}
          </ul>}
          {createAction({ kind: "book", seriesId: series.id })}
        </details>
      </li>)}
    </ul>}
    </div>
  </>;
}
