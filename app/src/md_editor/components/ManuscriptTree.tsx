import { useEffect, useState } from "react";
import { createManuscriptClient } from "../../manuscripts/client";
import type { Book, ChapterScope, ChapterSummary, Series } from "../../manuscripts/contract";

type ManuscriptSeries = Series & { books: (Book & { chapters: ChapterSummary[] })[] };
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
    <button className="editor-button" type="button" onClick={() => setAttempt((value) => value + 1)}>Retry manuscript</button>
  </div>;

  return <>
    <p className="editor-note">Opening a chapter saves current changes first.</p>
    {state.series.length === 0 ? <p>No series yet.</p> : <ul className="editor-manuscript-tree">
      {state.series.map((series) => <li key={series.id}>
        <details open>
          <summary>{series.title}</summary>
          {series.books.length === 0 ? <p>No books yet.</p> : <ul>
            {series.books.map((book) => <li key={book.id}>
              <details open>
                <summary>{book.title}</summary>
                {book.chapters.length === 0 ? <p>No chapters yet.</p> : <ul>
                  {book.chapters.map((chapter) => <li key={chapter.id}>
                    <button type="button" disabled={disabled}
                      aria-pressed={selectedChapter?.seriesId === series.id && selectedChapter.bookId === book.id && selectedChapter.chapterId === chapter.id}
                      onClick={() => onSelect({ seriesId: series.id, bookId: book.id, chapterId: chapter.id })}>{chapter.title}</button>
                  </li>)}
                </ul>}
              </details>
            </li>)}
          </ul>}
        </details>
      </li>)}
    </ul>}
  </>;
}
