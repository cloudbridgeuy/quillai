import type { ManuscriptTreeData } from "./create_draft";
import type { StoryRoot } from "./story_tree";

export type RenameKind = "series" | "book" | "chapter" | "piece";
export type ParsedTitle = { status: "valid"; title: string } | { status: "unchanged" } | { status: "invalid"; message: string };

export function parseTitle(input: string, current: string): ParsedTitle {
  const title = input.trim();
  if (!title) return { status: "invalid", message: "Enter a title." };
  return title === current ? { status: "unchanged" } : { status: "valid", title };
}

export function renameTree(tree: ManuscriptTreeData, kind: RenameKind, id: number, title: string): ManuscriptTreeData {
  const rename = <T extends { id: number; title: string }>(node: T): T => node.id === id ? { ...node, title } : node;
  const root = (node: StoryRoot): StoryRoot => {
    const children = node.children.map(rename);
    return children.some((child, index) => child !== node.children[index]) ? { ...rename(node), children } : rename(node);
  };
  return {
    series: kind === "piece" ? tree.series : tree.series.map(series => kind === "series" ? rename(series) : {
      ...series, books: series.books.map(book => kind === "book" ? rename(book) : { ...book, chapters: book.chapters.map(rename) }),
    }),
    universes: kind === "book" || kind === "chapter" ? tree.universes : tree.universes.map(universe => ({
      ...universe, loose: kind === "piece" ? universe.loose.map(root) : universe.loose,
      containers: universe.containers.map(container => kind === "series"
        ? container.kind === "book_series" ? rename(container) : container
        : { ...container, pieces: container.pieces.map(root) }),
    })),
  };
}
