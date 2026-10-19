import { getApiBaseUrl } from "../api_config";
import {
  parseApiError, parseBook, parseChapter, parseList, parseRevision, parseSeries,
  type BookScope, type ChapterScope, type CreateBook, type CreateChapter,
  type CreateSeries, type SeriesScope, type SubmitDelta, type UpdateBook,
  type UpdateChapter, type UpdateSeries,
} from "./contract";

export class ManuscriptApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code: string,
    readonly kind: "http" | "invalid_response",
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
      throw new ManuscriptApiError(error.error, response.status, error.code, "http");
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
    listChapters: (scope: BookScope) => request(chaptersPath(scope), "GET", 200, (value) => parseList(value, parseChapter)),
    createChapter: (scope: BookScope, input: CreateChapter) => request(chaptersPath(scope), "POST", 201, parseChapter, input),
    getChapter: (scope: ChapterScope) => request(chapterPath(scope), "GET", 200, parseChapter),
    updateChapter: (scope: ChapterScope, input: UpdateChapter) => request(chapterPath(scope), "PUT", 200, parseChapter, input),
    deleteChapter: (scope: ChapterScope) => request(chapterPath(scope), "DELETE", 204, deleted),
    getDeltas: (scope: ChapterScope, since: number) => request(`${chapterPath(scope)}/deltas?since=${encodeURIComponent(since)}`, "GET", 200, (value) => parseList(value, parseRevision)),
    postDelta: (scope: ChapterScope, input: SubmitDelta) => request(`${chapterPath(scope)}/deltas`, "POST", 200, parseRevision, input),
  };
}
