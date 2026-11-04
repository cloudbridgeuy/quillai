import { apiErrorMessage } from "../api_error";
import { getApiBaseUrl } from "../api_config";
import {
  parseApiError, parseBook, parseChapter, parseChapterSummary, parseContainer, parseList, parsePiece,
  parsePieceDetail, parseRevision, parseScenePoll, parseScriptGroup, parseSeries,
  type BookScope, type ChapterScope, type ContainerScope, type CreateBook, type CreateChapter,
  type CreateChild, type CreateContainer, type CreatePiece, type CreateUniverse,
  type EditorScope, type PieceScope, type UniverseScope,
  type CreateSeries, type SeriesScope, type SubmitDelta, type UpdateBook,
  type UpdateChapter, type UpdatePiece, type UpdateSeries,
} from "./contract";

export class ManuscriptApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code: string,
    readonly kind: "http" | "invalid_response",
    readonly detail?: string,
  ) {
    super(message);
    this.name = "ManuscriptApiError";
  }
}

type Fetch = (url: string, init?: RequestInit) => Promise<Response>;

function seriesPath({ seriesId }: SeriesScope): string {
  return `/series/${encodeURIComponent(seriesId)}`;
}

function booksPath(scope: SeriesScope): string {
  return `${seriesPath(scope)}/books`;
}

function bookPath(scope: BookScope): string {
  return `${booksPath(scope)}/${encodeURIComponent(scope.bookId)}`;
}

function chaptersPath(scope: BookScope): string {
  return `${bookPath(scope)}/chapters`;
}

function chapterPath(scope: ChapterScope): string {
  return `${chaptersPath(scope)}/${encodeURIComponent(scope.chapterId)}`;
}

function piecePath({ pieceId }: Pick<PieceScope, "pieceId">): string {
  return `/pieces/${encodeURIComponent(pieceId)}`;
}

function universePath({ universeId }: UniverseScope): string {
  return `/universes/${encodeURIComponent(universeId)}`;
}

function documentPath(scope: EditorScope): string {
  return "pieceId" in scope ? piecePath(scope) : chapterPath(scope);
}

function treeRecord(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("Expected an object");
  return value as Record<string, unknown>;
}

function parseTree(value: unknown) {
  const row = treeRecord(value);
  const root = (value: unknown) => {
    const row = treeRecord(value);
    return { ...parsePiece(row), children: parseList(row.children, parsePiece) };
  };
  return {
    series: parseList(row.series, (value) => {
      const row = treeRecord(value);
      return { ...parseSeries(row), books: parseList(row.books, (value) => {
        const row = treeRecord(value);
        return { ...parseBook(row), chapters: parseList(row.chapters, parseChapterSummary) };
      }) };
    }),
    universes: parseList(row.universes, (value) => {
      const row = treeRecord(value);
      return { ...parseSeries(row), containers: parseList(row.containers, (value) => {
        const row = treeRecord(value);
        return { ...parseContainer(row), pieces: parseList(row.pieces, root) };
      }), loose: parseList(row.loose, root) };
    }),
  };
}

export function createManuscriptClient({
  baseUrl = getApiBaseUrl(),
  fetch: fetchRequest = globalThis.fetch,
}: { baseUrl?: string; fetch?: Fetch } = {}) {
  const base = baseUrl.replace(/\/+$/, "");

  async function request<T>(
    path: string,
    method: string,
    status: number,
    parse: (value: unknown) => T,
    body?: unknown,
  ): Promise<T> {
    const response = await fetchRequest(`${base}${path}`, {
      method,
      headers: { Accept: "application/json", ...(body === undefined ? {} : { "Content-Type": "application/json" }) },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    if (!response.ok) {
      let error: { error: string; code: string };
      try {
        error = parseApiError(await response.json());
      } catch {
        throw new ManuscriptApiError(
          `HTTP ${response.status}: invalid error response`, response.status, "INVALID_RESPONSE", "invalid_response",
        );
      }
      throw new ManuscriptApiError(apiErrorMessage(error.code, error.error), response.status, error.code, "http", error.error);
    }
    try {
      if (response.status !== status) throw new Error(`Expected HTTP ${status}`);
      return parse(status === 204 ? await response.text() : await response.json());
    } catch (error) {
      throw new ManuscriptApiError(
        error instanceof Error ? error.message : "Invalid response", response.status, "INVALID_RESPONSE", "invalid_response",
      );
    }
  }

  function deleted(value: unknown): void {
    if (value !== "") throw new Error("Expected an empty delete response");
  }

  return {
    getTree: () => request("/tree", "GET", 200, parseTree),
    listSeries: () => request("/series", "GET", 200, (value) => parseList(value, parseSeries)),
    createSeries: (input: CreateSeries) => request("/series", "POST", 201, parseSeries, input),
    getSeries: (scope: SeriesScope) => request(seriesPath(scope), "GET", 200, parseSeries),
    updateSeries: (scope: SeriesScope, input: UpdateSeries) => request(seriesPath(scope), "PUT", 200, parseSeries, input),
    deleteSeries: (scope: SeriesScope) => request(seriesPath(scope), "DELETE", 204, deleted),
    listBooks: (scope: SeriesScope) => request(booksPath(scope), "GET", 200, (value) => parseList(value, parseBook)),
    createBook: (scope: SeriesScope, input: CreateBook) => request(booksPath(scope), "POST", 201, parseBook, input),
    getBook: (scope: BookScope) => request(bookPath(scope), "GET", 200, parseBook),
    updateBook: (scope: BookScope, input: UpdateBook) => request(bookPath(scope), "PUT", 200, parseBook, input),
    deleteBook: (scope: BookScope) => request(bookPath(scope), "DELETE", 204, deleted),
    listChapters: (scope: BookScope) => request(chaptersPath(scope), "GET", 200, (value) => parseList(value, parseChapterSummary)),
    listChaptersWithContent: (scope: BookScope) => request(`${chaptersPath(scope)}?content=true`, "GET", 200, (value) => parseList(value, parseChapter)),
    createChapter: (scope: BookScope, input: CreateChapter) => request(chaptersPath(scope), "POST", 201, parseChapter, input),
    getChapter: (scope: ChapterScope) => request(chapterPath(scope), "GET", 200, parseChapter),
    updateChapter: (scope: ChapterScope, input: UpdateChapter) => request(chapterPath(scope), "PUT", 200, parseChapter, input),
    deleteChapter: (scope: ChapterScope) => request(chapterPath(scope), "DELETE", 204, deleted),
    listUniverses: () => request("/universes", "GET", 200, (value) => parseList(value, parseSeries)),
    createUniverse: (input: CreateUniverse) => request("/universes", "POST", 201, parseSeries, input),
    deleteUniverse: (scope: UniverseScope) => request(universePath(scope), "DELETE", 204, deleted),
    restoreUniverse: (scope: UniverseScope) => request(`${universePath(scope)}/restore`, "POST", 200, parseSeries),
    createContainer: (scope: UniverseScope, input: CreateContainer) => request(`${universePath(scope)}/containers`, "POST", 201, parseContainer, input),
    deleteContainer: ({ containerId }: ContainerScope) => request(`/containers/${encodeURIComponent(containerId)}`, "DELETE", 204, deleted),
    restoreContainer: ({ containerId }: ContainerScope) => request(`/containers/${encodeURIComponent(containerId)}/restore`, "POST", 200, parseContainer),
    createLoosePiece: (scope: UniverseScope, input: CreatePiece) => request(`${universePath(scope)}/pieces`, "POST", 201, parsePiece, input),
    createContainerPiece: ({ containerId }: ContainerScope, input: CreatePiece) => request(`/containers/${encodeURIComponent(containerId)}/pieces`, "POST", 201, parsePiece, input),
    createChild: (scope: Pick<PieceScope, "pieceId">, input: CreateChild) => request(`${piecePath(scope)}/children`, "POST", 201, parsePiece, input),
    updatePiece: (scope: Pick<PieceScope, "pieceId">, input: UpdatePiece) => request(piecePath(scope), "PUT", 200, parsePieceDetail, input),
    deletePiece: (scope: Pick<PieceScope, "pieceId">) => request(piecePath(scope), "DELETE", 204, deleted),
    restorePiece: (scope: Pick<PieceScope, "pieceId">) => request(`${piecePath(scope)}/restore`, "POST", 200, parsePieceDetail),
    listContainers: (scope: UniverseScope) => request(`${universePath(scope)}/containers`, "GET", 200, (value) => parseList(value, parseContainer)),
    listLoosePieces: (scope: UniverseScope) => request(`${universePath(scope)}/pieces?container=none`, "GET", 200, (value) => parseList(value, parsePiece)),
    listContainerPieces: ({ containerId }: ContainerScope) => request(`/containers/${encodeURIComponent(containerId)}/pieces`, "GET", 200, (value) => parseList(value, parsePiece)),
    getPiece: (scope: Pick<PieceScope, "pieceId">) => request(piecePath(scope), "GET", 200, parsePieceDetail),
    getScriptGroup: (scope: Pick<PieceScope, "pieceId">) => request(`${piecePath(scope)}/scenes`, "GET", 200, parseScriptGroup),
    pollScriptGroup: (scope: Pick<PieceScope, "pieceId">, since: ReadonlyMap<number, number>) => request(
      `${piecePath(scope)}/scenes/deltas?since=${encodeURIComponent([...since].map(([id, rev]) => `${id}:${rev}`).join(","))}`,
      "GET", 200, parseScenePoll),
    getDeltas: (scope: EditorScope, since: number) => request(`${documentPath(scope)}/deltas?since=${encodeURIComponent(since)}`, "GET", 200, (value) => parseList(value, parseRevision)),
    postDelta: (scope: EditorScope, input: SubmitDelta) => request(`${documentPath(scope)}/deltas`, "POST", 200, parseRevision, input),
  };
}
