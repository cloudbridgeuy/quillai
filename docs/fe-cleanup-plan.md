---
shaping: plan
---

# FE Cleanup Plan — Bundle Quill, Port from Env, Loading State

## Overview

Three independent frontend improvements from the codebase audit (FE-7, FE-9, FE-13):
1. Bundle Quill via npm import instead of runtime script injection (eliminates race condition + vendored assets)
2. Make server port configurable via environment variable
3. Add loading state to APITester

## Pattern Applicability

| Pattern | Applies? | How |
|---------|:--------:|-----|
| Functional Core / Imperative Shell | **Yes** | `formatResponseBody()` in APITester is already pure (functional core). The new `buttonLabel()` helper follows the same pattern — pure function from state to display string, unit-tested. React components are the imperative shell. |
| Make Impossible States Impossible | **No** | The `isLoading` boolean is a simple on/off toggle — the pattern explicitly says "A single `bool` for 'is enabled' does not need an enum with `Enabled` and `Disabled` variants." |
| Parse Don't Validate | **Yes** (minor) | Port parsing: `Number(process.env.PORT)` at the boundary, with fallback to default. |
| Type-Driven Development | **No** | No new types needed — changes are small and concrete. |
| Algebraic Data Types | **No** | No new sum/product types. |
| Composition over Inheritance | **N/A** | Already the default in React (components) and Bun (config objects). |
| CQRS | **No** | Not applicable. |
| Data-Oriented Design | **No** | Not applicable. |

---

## File Map

Modified files:
- `app/src/md_editor/components/Editor.tsx` — replace `window.Quill` with imports, remove runtime loading
- `app/src/api_tester/components/APITester.tsx` — add loading state
- `app/src/index.tsx` — port from env

Deleted files:
- `app/assets/quilljs/` — vendored Quill JS, CSS, and source maps
- `app/assets/highlight/` — vendored highlight.js (unused)

Test files:
- `app/src/api_tester/components/__tests__/APITester.test.ts` — unit tests for `buttonLabel()` pure function

---

## Phase 1 — Three parallel tasks ✅

All three tasks are fully independent: different files, no shared state, no ordering dependency. They run in parallel on the same worktree.

### Task 1A: Bundle Quill via import (FE-7 + FE-13 partial) ✅

**Files:** `app/src/md_editor/components/Editor.tsx`

**Changes:**

1. Add imports at module level:
   ```tsx
   import Quill from "quill";
   import { Delta } from "quill/core";
   import "quill/dist/quill.snow.css";
   ```

2. In `processMarkdownSyntax()` (line 54):
   - Replace `window.Quill.import('delta')` with the module-level `Delta` import
   - `const replacementDelta = new Delta()` — uses the imported `Delta` directly

3. In `Component` (lines 116, 126, 135, 140, 141):
   - Replace `new window.Quill(editorContainer, ...)` → `new Quill(editorContainer, ...)`
   - Replace `window.Quill.events.TEXT_CHANGE` → `Quill.events.TEXT_CHANGE`
   - Replace `window.Quill.events.SELECTION_CHANGE` → `Quill.events.SELECTION_CHANGE`

4. In `Container` (lines 153-191):
   - Delete the `useEffect` block (lines 159-184) that:
     - Checks for and appends the `<link>` tag for `quill.snow.css`
     - Checks `!window.Quill` and appends the `<script>` tag
     - Sets `isReady` on script load
   - Remove `const [isReady, setIsReady] = useState(false)` state
   - Remove `if (!isReady) return null` guard (line 189)
   - Replace `const Delta = window.Quill.import("delta")` (line 191) with the module-level `Delta` import
   - Remove `useEffect` from the import list if no longer used (it's still used in `Component`, but `Container` may not need it)

5. Remove `useState` from `Container` imports if `isReady` was the only `useState` usage (it's not — `range`, `lastChange`, `readOnly` still use it).

**Verification:**
- `bun --hot app/src/index.tsx` — Editor renders, text formatting works
- Inspect DOM — no injected `<script>` or `<link>` tags for Quill
- Multiple component mounts don't create duplicate scripts (race condition eliminated by design)

---

### Task 1B: Port from env (FE-13 partial) ✅

**File:** `app/src/index.tsx`

**Changes:**

1. Replace `port: 3001` (line 5) with:
   ```tsx
   port: Number(process.env.PORT || 3001),
   ```

**Verification:**
- `bun --hot app/src/index.tsx` — starts on port 3001 (default)
- `PORT=4000 bun --hot app/src/index.tsx` — starts on port 4000

---

### Task 1C: APITester loading state (FE-9) ✅

**File:** `app/src/api_tester/components/APITester.tsx`

**Changes:**

1. Add `useState` to imports:
   ```tsx
   import { useRef, useState, type FormEvent } from "react";
   ```

2. Add loading state inside `APITester`:
   ```tsx
   const [isLoading, setIsLoading] = useState(false);
   ```

3. Wrap the fetch logic in `testEndpoint`:
   ```tsx
   const testEndpoint = async (e: FormEvent<HTMLFormElement>) => {
     e.preventDefault();
     setIsLoading(true);
     try {
       // ... existing fetch logic ...
     } catch (error) {
       // ... existing error handling ...
     } finally {
       setIsLoading(false);
     }
   };
   ```

4. Extract a pure function for button label (FC/IS — pure core, unit-testable):
   ```tsx
   export function buttonLabel(isLoading: boolean): string {
     return isLoading ? "Sending..." : "Send";
   }
   ```

5. Update the Send button:
   ```tsx
   <button type="submit" className="send-button" disabled={isLoading}>
     {buttonLabel(isLoading)}
   </button>
   ```

**Test file:** `app/src/api_tester/components/__tests__/APITester.test.ts`

Unit tests for `buttonLabel()` (pure function — FC/IS pattern):
```tsx
import { describe, test, expect } from "bun:test";
import { buttonLabel } from "../../APITester";

describe("buttonLabel", () => {
  test("returns 'Sending...' when loading", () => {
    expect(buttonLabel(true)).toBe("Sending...");
  });

  test("returns 'Send' when not loading", () => {
    expect(buttonLabel(false)).toBe("Send");
  });
});
```

**Verification:**
- `bun test` — pure function tests pass
- Click Send in APITester — button shows "Sending..." and is disabled until response arrives

---

## Phase 2 — Cleanup + verify (depends on Phase 1) ✅

### Task 2A: Delete vendored assets ✅

**Delete:**
- `app/assets/quilljs/quill.2.0.3.js`
- `app/assets/quilljs/quill.snow.css`
- `app/assets/quilljs/quill.js.map`
- `app/assets/quilljs/quill.snow.css.map`
- `app/assets/highlight/` (entire directory — unused)

**Verification:**
- `bun --hot app/src/index.tsx` — app still works (Quill loads from npm, not assets)
- No 404s in browser console for asset requests

---

### Task 2B: Update audit doc ✅

**File:** `docs/codebase-audit-shaping.md`

Mark resolved items:
- FE-7: **DONE** — eliminated by bundling Quill via import (no runtime script loading)
- FE-9: **DONE** — added loading state with disabled button + "Sending..." label
- FE-13: **DONE** — port from env; asset paths eliminated by bundling Quill via import

Remove RUST-7 and FE-10 from the findings table (not necessary per user decision).

---

## Execution Summary

All phases complete. 22 bun tests passing, vendored assets deleted (~235K lines removed).

```
Phase 1 (parallel):   Task 1A (bundle Quill) ✅  ║  Task 1B (port from env) ✅  ║  Task 1C (loading state) ✅
                            ╲                            │                            ╱
Phase 2 (sequential):       Task 2A (delete vendored assets) ✅ → Task 2B (update audit doc) ✅
```

## Dependencies

```
Task 1A ✅ ──┐
Task 1B ✅ ──┼──▶ Task 2A ✅ ──▶ Task 2B ✅
Task 1C ✅ ──┘
```
