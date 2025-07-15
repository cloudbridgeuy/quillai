import type Quill from "quill";
import {
  forwardRef,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import type {
  DeltaT,
  OnTextChangeFn,
  OnSelectionChangeFn,
  RangeT,
} from "../types";

// Markdown syntax processor
const processMarkdownSyntax = (quill: Quill, delta: DeltaT, source: string) => {
  if (source !== 'user') return;
  
  const text = quill.getText();
  const selection = quill.getSelection();
  
  if (!selection) return;
  
  // Check if user just typed an asterisk
  const lastOp = delta.ops?.[delta.ops.length - 1];
  if (!lastOp || !lastOp.insert || typeof lastOp.insert !== 'string') return;
  
  const lastChar = lastOp.insert.charAt(lastOp.insert.length - 1);
  if (lastChar !== '*') return;
  
  const currentPos = selection.index;
  const lineStart = text.lastIndexOf('\n', currentPos - 1) + 1;
  const lineEnd = text.indexOf('\n', currentPos);
  const lineText = text.substring(lineStart, lineEnd === -1 ? text.length : lineEnd);
  
  // Process different markdown patterns
  const patterns = [
    // Bold italic (***text***)
    {
      regex: /\*\*\*([^*]+)\*\*\*$/,
      format: { bold: true, italic: true },
      length: 6 // 3 asterisks on each side
    },
    // Bold (**text**)
    {
      regex: /\*\*([^*]+)\*\*$/,
      format: { bold: true },
      length: 4 // 2 asterisks on each side
    },
    // Italic (*text*)
    {
      regex: /\*([^*]+)\*$/,
      format: { italic: true },
      length: 2 // 1 asterisk on each side
    }
  ];
  
  for (const pattern of patterns) {
    const match = lineText.match(pattern.regex);
    if (match) {
      const matchStart = lineStart + match.index!;
      const matchEnd = matchStart + match[0].length;
      const textStart = matchStart + pattern.length / 2;
      const textEnd = matchEnd - pattern.length / 2;
      
      // Remove the asterisks and format the text
      quill.deleteText(textEnd, pattern.length / 2, 'silent');
      quill.deleteText(matchStart, pattern.length / 2, 'silent');
      quill.formatText(matchStart, textEnd - textStart, pattern.format, 'silent');
      
      // Move cursor to the end of the formatted text
      quill.setSelection(textEnd - pattern.length / 2, 0, 'silent');
      break;
    }
  }
};

export type ComponentPropsT = {
  readOnly: boolean;
  defaultValue: DeltaT;
  onTextChange: OnTextChangeFn;
  onSelectionChange: OnSelectionChangeFn;
};

// Editor uncontrolled React component
export const Component = forwardRef<Quill, ComponentPropsT>(
  ({ readOnly, defaultValue, onTextChange, onSelectionChange }, ref) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const defaultValueRef = useRef(defaultValue);
    const onTextChangeRef = useRef(onTextChange);
    const onSelectionChangeRef = useRef(onSelectionChange);

    useLayoutEffect(() => {
      onTextChangeRef.current = onTextChange;
      onSelectionChangeRef.current = onSelectionChange;
    });

    useEffect(() => {
      if (!ref || ref == null || typeof ref === "function") return;
      ref.current?.enable(!readOnly);
    }, [ref, readOnly]);

    useEffect(() => {
      if (
        containerRef === null ||
        !ref ||
        ref == null ||
        typeof ref === "function"
      )
        return;

      const container = containerRef.current;

      if (container === null) return;

      const editorContainer = container.appendChild(
        container.ownerDocument.createElement("div"),
      );

      const quill = new window.Quill(editorContainer, {
        theme: "snow",
      });

      ref.current = quill;

      if (defaultValueRef.current) {
        quill.setContents(defaultValueRef.current);
      }

      quill.on(window.Quill.events.TEXT_CHANGE, (...args) => {
        const [delta, oldDelta, source] = args;
        
        // Process markdown syntax
        processMarkdownSyntax(quill, delta, source);
        
        onTextChangeRef.current?.(...args);
      });

      quill.on(window.Quill.events.SELECTION_CHANGE, (...args) => {
        onSelectionChangeRef.current?.(...args);
      });

      return () => {
        ref.current = null;
        container.innerHTML = "";
      };
    }, [ref]);

    return <div ref={containerRef}></div>;
  },
);

Component.displayName = "EditorComponent";

export const Container = () => {
  const [isReady, setIsReady] = useState(false);
  const [range, setRange] = useState<RangeT | null>(null);
  const [lastChange, setLastChange] = useState<DeltaT | null>();
  const [readOnly, setReadOnly] = useState(false);

  useLayoutEffect(() => {
    // Check if the quill.snow.css stylesheet has been appended.
    if (
      !document.querySelector('link[href="/assets/quilljs/quill.snow.css"]')
    ) {
      const link = document.createElement("link");
      link.rel = "stylesheet";
      link.href = "/assets/quilljs/quill.snow.css";
      document.head.appendChild(link);
    }

    if (!window.Quill) {
      try {
        const script = document.createElement("script");
        script.type = "text/javascript";
        script.src = "/assets/quilljs/quill.2.0.3.js";
        script.onload = () => {
          setIsReady(true);
        };
        // Attach to head
        document.getElementsByTagName("head")[0].appendChild(script);
      } catch (error) {
        console.error("Error loading Quill script:", error);
      }
    }
  }, []);

  // Use a ref to access the quill instance directly
  const quillRef = useRef<Quill | null>(null);

  if (!isReady) return null;

  const Delta = window.Quill.import("delta");

  return (
    <div>
      <Component
        ref={quillRef}
        readOnly={readOnly}
        defaultValue={new Delta()
          .insert("Hello")
          .insert("\n", { header: 1 })
          .insert("Some ")
          .insert("initial", { bold: true })
          .insert(" ")
          .insert("content", { underline: true })
          .insert("\n")}
        onSelectionChange={setRange}
        onTextChange={setLastChange}
      />
      <div className="controls">
        <label>
          Read Only:{" "}
          <input
            type="checkbox"
            checked={readOnly}
            onChange={(e) => setReadOnly(e.target.checked)}
          />
        </label>
        <button
          className="controls-right"
          type="button"
          onClick={() => {
            alert(quillRef.current?.getLength());
          }}
        >
          Get Content Length
        </button>
      </div>
      <div className="state">
        <div className="state-title">Current Range:</div>
        {range ? JSON.stringify(range) : "Empty"}
      </div>
      <div className="state">
        <div className="state-title">Last Change:</div>
        {lastChange ? JSON.stringify(lastChange.ops) : "Empty"}
      </div>
    </div>
  );
};

Container.displayName = "EditorContainer";

export default Container;
