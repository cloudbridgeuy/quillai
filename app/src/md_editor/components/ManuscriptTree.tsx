import { useEffect, useLayoutEffect, useReducer, useRef, useState } from "react";
import { Button } from "../../ui/Button";
import { ButtonLink } from "../../ui/ButtonLink";
import { IconButton } from "../../ui/IconButton";
import { ListButton } from "../../ui/ListButton";
import { TextField } from "../../ui/TextField";
import { Alert } from "../../ui/Alert";
import { createManuscriptClient } from "../../manuscripts/client";
import { Field } from "../../ui/Field";
import type { EditorScope, Piece } from "../../manuscripts/contract";
import {
  addChapter, addChild, addContainer, addRoot, addUniverse, childName, containerKinds, createTitle, draftTransition, formats,
  labelNames, labelOptions, pieceShape, targetPosition, withBooks, type CreateTarget, type ManuscriptTreeData,
} from "../../manuscripts/create_draft";
import { containerKindLabel, facadeBook, pieceRoots, pieceScope, sameScope, treeUniverses, type StoryContainer, type StoryRoot, type StoryUniverse } from "../../manuscripts/story_tree";
import { treeScrollTarget } from "./tree_scroll";
import { landingUrl, scriptGroupId } from "../../landing/routes";
import { createTrashClient } from "../../manuscripts/trash_client";

type TreeState =
  | { status: "loading" }
  | { status: "failed"; message: string }
  | ({ status: "ready" } & ManuscriptTreeData);

export function ManuscriptTree({ selectedChapter, disabled, busy = true, onCreatePending, savesBeforeOpening = false, collapsedExcept, onSelect, onDelete, onLeave, onChange, activeScript }: {
  selectedChapter: EditorScope | null;
  disabled: boolean;
  busy?: boolean;
  onCreatePending?: (pending: boolean) => void;
  savesBeforeOpening?: boolean;
  collapsedExcept?: number | null;
  onSelect: (scope: EditorScope, opensScript: boolean) => void;
  onDelete?: (remove: () => Promise<void>, path: string, title: string) => Promise<void>;
  onLeave?: (target: URL) => Promise<void>;
  onChange?: () => void;
  activeScript?: number;
}) {
  const [client] = useState(createManuscriptClient);
  const [trash] = useState(createTrashClient);
  const [deleted, setDeleted] = useState<{ title: string; resourcePath: string } | null>(() => {
    const params = new URL(window.location.href).searchParams;
    const resourcePath = params.get("deleted");
    const title = params.get("deletedTitle");
    return resourcePath && title ? { title, resourcePath } : null;
  });
  const [deleteError, setDeleteError] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [state, setState] = useState<TreeState>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [draft, dispatch] = useReducer(draftTransition, null);
  const [createdUniverses, setCreatedUniverses] = useState<number[]>([]);
  const submitting = useRef(false);
  const deleteBusy = useRef(false);
  const treeRef = useRef<HTMLDivElement>(null);
  const selectedKey = JSON.stringify(selectedChapter);
  const draftKey = draft && JSON.stringify(draft.target);

  async function remove(title: string, path: string, action: () => Promise<void>, affected: boolean) {
    if (disabled || deleteBusy.current || draft?.pending) return;
    deleteBusy.current = true;
    setDeleting(true);
    setDeleted(null);
    setDeleteError("");
    const url = new URL(window.location.href);
    url.searchParams.delete("deleted");
    url.searchParams.delete("deletedTitle");
    window.history.replaceState(null, "", url);
    let removed = false;
    const removeItem = async () => {
      await action();
      removed = true;
      setDeleted({ title, resourcePath: path });
      setAttempt(value => value + 1);
      onChange?.();
    };
    try {
      if (affected && onDelete) return await onDelete(removeItem, path, title);
      await removeItem();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Action failed.";
      setDeleteError(removed ? `Item deleted, but returning to the library failed: ${message} Use Undo to restore it.` : message);
    } finally {
      deleteBusy.current = false;
      setDeleting(false);
    }
  }

  function deleteButton(title: string, path: string, action: () => Promise<void>, affected: boolean) {
    return <IconButton className="tree-delete" tabIndex={0} label={`Delete '${title}'`} glyph={<svg aria-hidden="true" className="mx-auto size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7" /></svg>}
      disabled={disabled || deleting || Boolean(draft?.pending && busy)} onClick={event => { event.preventDefault(); event.stopPropagation(); void remove(title, path, action, affected); }} />;
  }

  async function undo() {
    if (!deleted || disabled || deleteBusy.current || draft?.pending) return;
    deleteBusy.current = true;
    setDeleting(true);
    setDeleteError("");
    try {
      await trash.restoreResource(deleted.resourcePath);
      setDeleted(null);
      const url = new URL(window.location.href);
      url.searchParams.delete("deleted");
      url.searchParams.delete("deletedTitle");
      window.history.replaceState(null, "", url);
      setAttempt(value => value + 1);
      onChange?.();
    } catch (error) {
      setDeleteError(error instanceof Error ? error.message : "Could not restore item.");
    } finally {
      deleteBusy.current = false;
      setDeleting(false);
    }
  }

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
  }, [state.status, selectedKey]);

  useLayoutEffect(() => {
    treeRef.current?.querySelector("[data-manuscript-draft]")?.scrollIntoView({ block: "nearest" });
  }, [draftKey]);

  async function create() {
    if (!draft || disabled || deleting || submitting.current || state.status !== "ready") return;
    const title = createTitle(draft.fields.title);
    dispatch({ type: "submit" });
    if (!title) return;
    submitting.current = true;
    const opensChapter = draft.target.kind === "chapter" || draft.target.kind === "child" || (draft.target.kind === "piece" && pieceShape(draft.fields.label) === "leaf");
    if (opensChapter) onCreatePending?.(true);
    try {
      const { target, fields } = draft;
      const input = { title, position: targetPosition(state, target) };
      let tree: ManuscriptTreeData = state;
      let opened: EditorScope | null = null;
      let opensScript = false;
      if (target.kind === "universe") {
        const universe = await client.createUniverse(input);
        setCreatedUniverses(ids => [...ids, universe.id]);
        tree = addUniverse(state, universe);
      }
      else if (target.kind === "container") {
        tree = addContainer(state, await client.createContainer(target, { ...input, kind: fields.containerKind }));
      } else if (target.kind === "piece") {
        const pieceInput = { ...input, label: fields.label, format: fields.format, shape: pieceShape(fields.label) };
        const piece = target.container
          ? await client.createContainerPiece({ containerId: target.container.id }, pieceInput)
          : await client.createLoosePiece(target, pieceInput);
        tree = addRoot(state, piece);
        if (target.container?.kind === "book_series" && facadeBook(piece)) {
          tree = withBooks(tree, target.container.id, await client.listBooks({ seriesId: target.container.id }));
        }
        if (piece.shape === "leaf") opened = { universeId: piece.universeId, pieceId: piece.id };
      } else if (target.kind === "child") {
        const child = await client.createChild(target, { ...input, label: fields.label });
        tree = addChild(state, child);
        opened = { universeId: child.universeId, pieceId: child.id };
        opensScript = scriptGroupId(child) !== undefined;
      } else {
        const kind = fields.label === "prologue" || fields.label === "epilogue" ? fields.label : "chapter";
        const chapter = await client.createChapter(target, { ...input, kind });
        tree = addChapter(state, target, chapter);
        opened = { seriesId: target.seriesId, bookId: target.bookId, chapterId: chapter.id };
      }
      setState({ status: "ready", series: tree.series, universes: tree.universes });
      if (opened) onSelect(opened, opensScript);
      dispatch({ type: "created" });
    } catch (error) {
      dispatch({ type: "failed", message: error instanceof Error ? error.message : "Could not create manuscript item." });
    } finally {
      submitting.current = false;
      if (opensChapter) onCreatePending?.(false);
    }
  }

  function createAction(target: CreateTarget, button: string, noun: string) {
    const active = draft && draftKey === JSON.stringify(target);
    if (!active) return <ListButton tone="tree" disabled={disabled || deleting || Boolean(draft?.pending && busy)}
      onClick={() => dispatch({ type: "start", target })}>+ {button}</ListButton>;
    const locked = disabled || deleting || (draft.pending && busy);
    const labels = labelOptions(target);
    return <form data-manuscript-draft className="flex scroll-my-2 flex-col gap-2" onSubmit={event => { event.preventDefault(); void create(); }}
      onKeyDown={event => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          dispatch({ type: "cancel" });
        }
        if (event.key === "Enter" && event.nativeEvent.isComposing) event.preventDefault();
      }}>
      <TextField square autoFocus aria-label={`${noun} title`} placeholder={`${noun} title`} value={draft.fields.title}
        disabled={locked} aria-invalid={Boolean(draft.error)}
        onChange={event => dispatch({ type: "edit", fields: { title: event.target.value } })} />
      {target.kind === "container" ? <Field label="Kind">
        <TextField as="select" square value={draft.fields.containerKind} disabled={locked}
          onChange={event => dispatch({ type: "edit", fields: { containerKind: containerKinds.find(kind => kind === event.target.value) } })}>
          {containerKinds.map(kind => <option key={kind} value={kind}>{containerKindLabel[kind]}</option>)}
        </TextField>
      </Field> : null}
      <div className={target.kind === "piece" ? "grid grid-cols-2 gap-2" : "contents"}>
      {labels.length ? <Field label="Label">
        <TextField as="select" square value={draft.fields.label} disabled={locked}
          onChange={event => dispatch({ type: "edit", fields: { label: labels.find(label => label === event.target.value) } })}>
          {labels.map(label => <option key={label} value={label}>{labelNames[label]}</option>)}
        </TextField>
      </Field> : null}
      {target.kind === "piece" ? <Field label="Format">
        <TextField as="select" square value={draft.fields.format} disabled={locked}
          onChange={event => dispatch({ type: "edit", fields: { format: formats.find(format => format === event.target.value) } })}>
          {formats.map(format => <option key={format} value={format}>{format === "script" ? "Script" : "Prose"}</option>)}
        </TextField>
      </Field> : null}
      </div>
      <div className="flex gap-2">
        <Button type="submit" variant="primary" size="sm" disabled={locked}>Create</Button>
        <Button size="sm" disabled={locked} onClick={() => dispatch({ type: "cancel" })}>Cancel</Button>
      </div>
      {draft.error ? <Alert tone="error" variant="inline">{draft.error}</Alert> : null}
      {draft.pending && busy ? <p role="status">Creating {noun.toLowerCase()}…</p> : null}
    </form>;
  }

  useEffect(() => {
    let active = true;
    setState({ status: "loading" });
    async function roots(pieces: Piece[]): Promise<StoryRoot[]> {
      return Promise.all(pieces.map(async (piece) => {
        const detail = piece.shape === "group" ? await client.getPiece({ pieceId: piece.id }) : null;
        return { ...piece, children: detail?.shape === "group" ? detail.children : [] };
      }));
    }
    async function loadUniverses(): Promise<StoryUniverse[]> {
      return Promise.all((await client.listUniverses()).map(async (universe) => {
        const scope = { universeId: universe.id };
        const [containers, loose] = await Promise.all([client.listContainers(scope), client.listLoosePieces(scope)]);
        return {
          ...universe,
          loose: await roots(loose),
          containers: await Promise.all(containers.map(async (container) => ({
            ...container,
            pieces: await roots(await client.listContainerPieces({ containerId: container.id })),
          }))),
        };
      }));
    }
    async function loadSeries() {
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
    Promise.all([loadSeries(), loadUniverses()]).then(([series, universes]) => {
      if (active) setState({ status: "ready", series, universes });
    }).catch((error: unknown) => {
      if (active) setState({ status: "failed", message: error instanceof Error ? error.message : "Could not load manuscript." });
    });
    return () => { active = false; };
  }, [client, attempt]);

  if (state.status === "loading") return <p role="status">Loading manuscript…</p>;
  if (state.status === "failed") return <Alert tone="error" variant="inline">
    <p>{state.message}</p>
    <Button onClick={() => setAttempt((value) => value + 1)}>Retry manuscript</Button>
  </Alert>;

  const series = state.series;
  const universes = collapsedExcept === undefined
    ? treeUniverses(state.universes, selectedChapter)
    : state.universes.map(universe => ({ universe, open: universe.id === collapsedExcept }));

  function includesPiece(piece: StoryRoot) {
    const selected = selectedChapter && "pieceId" in selectedChapter ? selectedChapter.pieceId : null;
    return piece.id === selected || piece.children.some(child => child.id === selected) || piece.id === activeScript;
  }

  function pieceButton(story: StoryUniverse, piece: Piece) {
    const scope = pieceScope(story, piece);
    return <ListButton tone="tree" disabled={disabled || deleting || Boolean(draft?.pending && busy) || !scope}
      pressed={sameScope(selectedChapter, scope)} onClick={() => { if (scope) onSelect(scope, scriptGroupId(piece) !== undefined); }}>{piece.title}</ListButton>;
  }

  function rootItem(story: StoryUniverse, piece: StoryRoot) {
    const noun = childName(piece.format);
    return <li key={`piece-${piece.id}`}>
      {piece.shape === "leaf" ? <div className="tree-row">{pieceButton(story, piece)}{deleteButton(piece.title, `/pieces/${piece.id}`, () => client.deletePiece({ pieceId: piece.id }), includesPiece(piece))}</div> : <details open>
        <summary className="tree-row">{piece.format === "script" ? <ListButton tone="tree" disabled={disabled || deleting || Boolean(draft?.pending && busy)}
          pressed={sameScope(selectedChapter, { universeId: story.id, pieceId: piece.id })}
          onClick={() => onSelect({ universeId: story.id, pieceId: piece.id }, true)}>{piece.title}</ListButton> : piece.title}{deleteButton(piece.title, `/pieces/${piece.id}`, () => client.deletePiece({ pieceId: piece.id }), includesPiece(piece))}</summary>
        {piece.children.length === 0 ? <p>{piece.format === "script" ? "No scenes yet." : "No chapters yet."}</p> : <ul>
          {piece.children.map((child) => <li key={`piece-${child.id}`} className="tree-row">{pieceButton(story, child)}{deleteButton(child.title, `/pieces/${child.id}`, () => client.deletePiece({ pieceId: child.id }), activeScript === piece.id || sameScope(selectedChapter, pieceScope(story, child)))}</li>)}
        </ul>}
        {createAction({ kind: "child", universeId: story.id, pieceId: piece.id, format: piece.format }, noun, noun)}
      </details>}
    </li>;
  }

  function bookItems(container: StoryContainer) {
    if (container.kind !== "book_series") return [];
    return (series.find(item => item.id === container.id)?.books ?? []).map((book) => <li key={`book-${book.id}`}>
      <details open>
        <summary className="tree-row">{book.title}{deleteButton(book.title, `/series/${container.id}/books/${book.id}`, () => client.deleteBook({ seriesId: container.id, bookId: book.id }), Boolean(selectedChapter && "bookId" in selectedChapter && selectedChapter.seriesId === container.id && selectedChapter.bookId === book.id))}</summary>
        {book.chapters.length === 0 ? <p>No chapters yet.</p> : <ul>
          {book.chapters.map((chapter) => <li key={chapter.id} className="tree-row">
            <ListButton tone="tree" disabled={disabled || deleting || Boolean(draft?.pending && busy)}
              pressed={sameScope(selectedChapter, { seriesId: container.id, bookId: book.id, chapterId: chapter.id })}
              onClick={() => onSelect({ seriesId: container.id, bookId: book.id, chapterId: chapter.id }, false)}>{chapter.title}</ListButton>
            {deleteButton(chapter.title, `/series/${container.id}/books/${book.id}/chapters/${chapter.id}`, () => client.deleteChapter({ seriesId: container.id, bookId: book.id, chapterId: chapter.id }), sameScope(selectedChapter, { seriesId: container.id, bookId: book.id, chapterId: chapter.id }))}
          </li>)}
        </ul>}
        {createAction({ kind: "chapter", seriesId: container.id, bookId: book.id }, "Chapter", "Chapter")}
      </details>
    </li>);
  }

  return <>
    {deleteError ? <Alert tone="error" variant="inline">{deleteError}</Alert> : null}
    {deleted ? <Alert tone="success" variant="inline">Deleted '{deleted.title}'. <Button size="sm" disabled={disabled || deleting || Boolean(draft?.pending && busy)} onClick={() => void undo()}>Undo</Button>{" "}
      <ButtonLink size="sm" aria-disabled={disabled || deleting} href={(() => { const url = landingUrl(window.location.href); url.searchParams.delete("deleted"); url.searchParams.delete("deletedTitle"); url.hash = "recently-deleted"; return url.toString(); })()}
        onClick={event => {
          if (!disabled && !deleting && !onLeave) return;
          event.preventDefault();
          if (!disabled && !deleting && onLeave) void onLeave(new URL(event.currentTarget.href));
        }}>Recently deleted</ButtonLink></Alert> : null}
    {savesBeforeOpening ? <p className="editor-note">Opening a chapter saves current changes first.</p> : null}
    <div ref={treeRef} className="editor-manuscript-tree">
    {createAction({ kind: "universe" }, "Universe", "Universe")}
    {state.universes.length === 0 ? <p>No universes yet.</p> : <ul>
      {universes.map(({ universe, open }) => <li key={`universe-${universe.id}`}>
        <details open={open || createdUniverses.includes(universe.id)}>
          <summary className="tree-row">{universe.title}{deleteButton(universe.title, `/universes/${universe.id}`, () => client.deleteUniverse({ universeId: universe.id }), Boolean(selectedChapter && ("universeId" in selectedChapter ? selectedChapter.universeId === universe.id : universe.containers.some(container => container.id === selectedChapter.seriesId))))}</summary>
          {universe.containers.length === 0 && universe.loose.length === 0 ? <p>No works yet.</p> : <ul>
            {universe.containers.map((container) => {
              const items = [...bookItems(container), ...pieceRoots(container).map((piece) => rootItem(universe, piece))];
              return <li key={`container-${container.id}`}>
                <details open>
                  <summary className="tree-row">{container.title} · {containerKindLabel[container.kind]}{deleteButton(container.title, container.kind === "book_series" ? `/series/${container.id}` : `/containers/${container.id}`, () => container.kind === "book_series" ? client.deleteSeries({ seriesId: container.id }) : client.deleteContainer({ containerId: container.id }), Boolean(selectedChapter && ("seriesId" in selectedChapter ? selectedChapter.seriesId === container.id : container.pieces.some(includesPiece))))}</summary>
                  {items.length === 0 ? <p>No pieces yet.</p> : <ul>{items}</ul>}
                  {createAction({ kind: "piece", universeId: universe.id, container: { id: container.id, kind: container.kind } }, "Piece", "Piece")}
                </details>
              </li>;
            })}
            {universe.loose.map((piece) => rootItem(universe, piece))}
          </ul>}
          {createAction({ kind: "container", universeId: universe.id }, "Container", "Container")}
          {createAction({ kind: "piece", universeId: universe.id, container: null }, "Loose piece", "Piece")}
        </details>
      </li>)}
    </ul>}
    </div>
  </>;
}
