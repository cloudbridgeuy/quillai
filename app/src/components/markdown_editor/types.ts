declare global {
  interface Window {
    Quill: typeof import('quill').default;
    hljs: {
      highlightAuto: (text: string) => { value: string };
      highlight: (text: string, options: { language: string }) => { value: string };
    };
  }
}

export interface MarkdownEditorRef {
  getContent: () => string;
  getMarkdown: () => string;
  setContent: (content: string) => void;
  setMarkdown: (markdown: string) => void;
  focus: () => void;
  blur: () => void;
}

export {};