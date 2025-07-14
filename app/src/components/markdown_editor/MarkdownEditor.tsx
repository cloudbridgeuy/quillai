import React, { useEffect, useRef, useState } from 'react';
import type Quill from 'quill';
import type { Delta } from 'quill';

interface MarkdownEditorProps {
  initialValue?: string;
  onChange?: (content: string, markdown: string) => void;
  placeholder?: string;
  readOnly?: boolean;
  className?: string;
}

export const MarkdownEditor: React.FC<MarkdownEditorProps> = ({
  initialValue = '',
  onChange,
  placeholder = 'Start typing markdown...',
  readOnly = false,
  className = '',
}) => {
  const editorRef = useRef<HTMLDivElement>(null);
  const quillRef = useRef<Quill | null>(null);
  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let mounted = true;
    
    const initializeEditor = async () => {
      if (!editorRef.current || !window.Quill) return;

      try {
        // Load highlight.js for syntax highlighting
        if (!window.hljs) {
          await new Promise<void>((resolve, reject) => {
            const script = document.createElement('script');
            script.src = '/assets/highlight/highlight.min.js';
            script.onload = () => resolve();
            script.onerror = reject;
            document.head.appendChild(script);
          });
        }

        // Load additional languages for highlight.js
        const languages = ['javascript', 'typescript', 'python', 'css', 'html', 'json', 'bash', 'rust', 'go'];
        await Promise.all(languages.map(lang => {
          return new Promise<void>((resolve) => {
            const script = document.createElement('script');
            script.src = `/assets/highlight/languages/${lang}.min.js`;
            script.onload = () => resolve();
            script.onerror = () => resolve(); // Continue even if language fails to load
            document.head.appendChild(script);
          });
        }));

        // Load GitHub theme for syntax highlighting
        if (!document.querySelector('link[href*="github.min.css"]')) {
          const link = document.createElement('link');
          link.rel = 'stylesheet';
          link.href = '/assets/highlight/styles/github.min.css';
          document.head.appendChild(link);
        }

        if (!mounted) return;

        // Initialize Quill with enhanced configuration
        const quill = new window.Quill(editorRef.current, {
          theme: 'snow',
          placeholder,
          readOnly,
          modules: {
            toolbar: [
              [{ 'header': [1, 2, 3, 4, 5, 6, false] }],
              ['bold', 'italic', 'underline', 'strike'],
              ['blockquote', 'code-block'],
              [{ 'list': 'ordered'}, { 'list': 'bullet' }],
              ['link', 'image'],
              ['clean']
            ],
            syntax: {
              highlight: (text: string) => {
                if (window.hljs) {
                  return window.hljs.highlightAuto(text).value;
                }
                return text;
              }
            },
            history: {
              delay: 1000,
              maxStack: 100,
              userOnly: true
            }
          },
          formats: [
            'header', 'bold', 'italic', 'underline', 'strike',
            'blockquote', 'code-block', 'code', 'link', 'image', 'list'
          ]
        });

        quillRef.current = quill;

        // Set initial content
        if (initialValue) {
          quill.setText(initialValue);
        }

        // Enhanced text change handler with markdown conversion
        quill.on('text-change', (delta: Delta, oldDelta: Delta, source: string) => {
          if (source === 'user') {
            const htmlContent = quill.root.innerHTML;
            const textContent = quill.getText();
            
            // Process markdown-like syntax in real-time
            processMarkdownSyntax(quill, delta);
            
            // Simple markdown conversion from rich text
            const markdownContent = convertToMarkdown(quill.getContents());
            
            onChange?.(htmlContent, markdownContent);
          }
        });

        // Apply markdown shortcuts
        setupMarkdownShortcuts(quill);

        setIsReady(true);
        setIsLoading(false);

      } catch (error) {
        console.error('Error initializing markdown editor:', error);
        setIsLoading(false);
      }
    };

    // Wait for Quill to be available
    if (window.Quill) {
      initializeEditor();
    } else {
      // Fallback if Quill is not loaded yet
      const checkQuill = setInterval(() => {
        if (window.Quill) {
          clearInterval(checkQuill);
          initializeEditor();
        }
      }, 100);

      return () => {
        clearInterval(checkQuill);
        mounted = false;
      };
    }

    return () => {
      mounted = false;
    };
  }, [initialValue, onChange, placeholder, readOnly]);

  // Process markdown syntax in real-time
  const processMarkdownSyntax = (quill: Quill, delta: Delta) => {
    const text = quill.getText();
    const selection = quill.getSelection();
    
    if (!selection) return;
    
    // Get the current line
    const lines = text.split('\n');
    const currentLineIndex = text.substr(0, selection.index).split('\n').length - 1;
    const currentLine = lines[currentLineIndex];
    
    // Process headers (# ## ###)
    const headerMatch = currentLine.match(/^(#{1,6})\s(.+)$/);
    if (headerMatch) {
      const level = headerMatch[1].length;
      const content = headerMatch[2];
      
      setTimeout(() => {
        const lineStart = text.indexOf(currentLine);
        quill.deleteText(lineStart, currentLine.length, 'silent');
        quill.insertText(lineStart, content, 'silent');
        quill.formatLine(lineStart, content.length, 'header', level, 'silent');
      }, 0);
    }
    
    // Process bold (**text**)
    const boldMatch = currentLine.match(/\*\*([^*]+)\*\*/g);
    if (boldMatch) {
      boldMatch.forEach(match => {
        const content = match.replace(/\*\*/g, '');
        const start = text.indexOf(match);
        setTimeout(() => {
          quill.deleteText(start, match.length, 'silent');
          quill.insertText(start, content, { bold: true }, 'silent');
        }, 0);
      });
    }
    
    // Process italic (*text*)
    const italicMatch = currentLine.match(/\*([^*]+)\*/g);
    if (italicMatch) {
      italicMatch.forEach(match => {
        const content = match.replace(/\*/g, '');
        const start = text.indexOf(match);
        setTimeout(() => {
          quill.deleteText(start, match.length, 'silent');
          quill.insertText(start, content, { italic: true }, 'silent');
        }, 0);
      });
    }
    
    // Process inline code (`code`)
    const codeMatch = currentLine.match(/`([^`]+)`/g);
    if (codeMatch) {
      codeMatch.forEach(match => {
        const content = match.replace(/`/g, '');
        const start = text.indexOf(match);
        setTimeout(() => {
          quill.deleteText(start, match.length, 'silent');
          quill.insertText(start, content, { code: true }, 'silent');
        }, 0);
      });
    }
  };

  // Helper function to convert Quill Delta to Markdown
  const convertToMarkdown = (contents: Delta): string => {
    let markdown = '';
    
    contents.ops?.forEach(op => {
      if (typeof op.insert === 'string') {
        let text = op.insert;
        
        if (op.attributes) {
          if (op.attributes.bold) {
            text = `**${text}**`;
          }
          if (op.attributes.italic) {
            text = `*${text}*`;
          }
          if (op.attributes.code) {
            text = `\`${text}\``;
          }
          if (op.attributes.link) {
            text = `[${text}](${op.attributes.link})`;
          }
          if (op.attributes.header) {
            const level = op.attributes.header;
            text = `${'#'.repeat(level)} ${text}`;
          }
        }
        
        markdown += text;
      }
    });
    
    return markdown;
  };

  // Setup markdown shortcuts
  const setupMarkdownShortcuts = (quill: Quill) => {
    // Header shortcuts
    quill.keyboard.addBinding({
      key: '1',
      ctrlKey: true,
      altKey: true,
      handler: () => {
        const range = quill.getSelection();
        if (range) {
          quill.formatLine(range.index, range.length, 'header', 1);
        }
      }
    });

    quill.keyboard.addBinding({
      key: '2',
      ctrlKey: true,
      altKey: true,
      handler: () => {
        const range = quill.getSelection();
        if (range) {
          quill.formatLine(range.index, range.length, 'header', 2);
        }
      }
    });

    quill.keyboard.addBinding({
      key: '3',
      ctrlKey: true,
      altKey: true,
      handler: () => {
        const range = quill.getSelection();
        if (range) {
          quill.formatLine(range.index, range.length, 'header', 3);
        }
      }
    });

    // Code block shortcut
    quill.keyboard.addBinding({
      key: 'E',
      ctrlKey: true,
      altKey: true,
      handler: () => {
        const range = quill.getSelection();
        if (range) {
          quill.formatLine(range.index, range.length, 'code-block', true);
        }
      }
    });

    // Blockquote shortcut
    quill.keyboard.addBinding({
      key: 'Q',
      ctrlKey: true,
      altKey: true,
      handler: () => {
        const range = quill.getSelection();
        if (range) {
          quill.formatLine(range.index, range.length, 'blockquote', true);
        }
      }
    });
  };

  // Update read-only state
  useEffect(() => {
    if (quillRef.current) {
      quillRef.current.enable(!readOnly);
    }
  }, [readOnly]);

  // Exposed methods
  const getContent = () => {
    return quillRef.current?.root.innerHTML || '';
  };

  const getMarkdown = () => {
    if (!quillRef.current) return '';
    return convertToMarkdown(quillRef.current.getContents());
  };

  const setContent = (content: string) => {
    if (quillRef.current) {
      quillRef.current.root.innerHTML = content;
    }
  };

  const setMarkdown = (markdown: string) => {
    if (quillRef.current) {
      quillRef.current.setText(markdown);
    }
  };

  const focus = () => {
    quillRef.current?.focus();
  };

  const blur = () => {
    quillRef.current?.blur();
  };

  // Expose methods via ref if needed
  React.useImperativeHandle(React.createRef(), () => ({
    getContent,
    getMarkdown,
    setContent,
    setMarkdown,
    focus,
    blur
  }));

  if (isLoading) {
    return (
      <div className={`markdown-editor-loading ${className}`}>
        <div className="flex items-center justify-center min-h-[200px] text-gray-500">
          Loading markdown editor...
        </div>
      </div>
    );
  }

  return (
    <div className={`markdown-editor-container ${className}`}>
      <div
        ref={editorRef}
        className="markdown-editor min-h-[300px] bg-white border border-gray-300 rounded-md"
      />
      
      {/* Keyboard shortcuts help */}
      <div className="mt-2 text-xs text-gray-500">
        <strong>Shortcuts:</strong> Ctrl+Alt+1-3 (Headers), Ctrl+Alt+E (Code block), Ctrl+Alt+Q (Quote)
      </div>
    </div>
  );
};

export default MarkdownEditor;