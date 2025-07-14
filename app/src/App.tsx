import { APITester } from "./api_tester/components/APITester";
import Editor from "./md_editor/components/Editor";
import MarkdownEditor from "./components/markdown_editor";
import "./index.css";

import logo from "./logo.svg";
import reactLogo from "./react.svg";

export function App() {
  return (
    <div className="app">
      <div className="logo-container">
        <img src={logo} alt="Bun Logo" className="logo bun-logo" />
        <img src={reactLogo} alt="React Logo" className="logo react-logo" />
      </div>

      <h1>Bun + React</h1>
      <p>
        Edit <code>src/App.tsx</code> and save to test HMR
      </p>
      
      <div className="section">
        <h2>Markdown Editor</h2>
        <MarkdownEditor 
          initialValue="# Welcome to Markdown Editor

This is a **powerful** markdown editor built with *QuillJS*!

## Features

- Syntax highlighting for code blocks
- Real-time markdown conversion
- Keyboard shortcuts
- Rich text editing

### Code Example

```javascript
const hello = 'world';
console.log(hello);
```

> This is a blockquote example

Start typing to see the magic happen!"
          onChange={(html, markdown) => {
            console.log('Content changed:', { html, markdown });
          }}
        />
      </div>
      
      <div className="section">
        <h2>Original Editor</h2>
        <Editor />
      </div>
      
      <div className="section">
        <h2>API Tester</h2>
        <APITester />
      </div>
    </div>
  );
}

export default App;
