# UI stories tool decision (GUZ-1435)

Spike for epic E19 (GUZ-1434). The question: which tool runs the stories for the `app/src/ui/` primitives?

- **A**: Storybook 10 (React + Vite builder) in its own folder with its own `package.json`. The app build is not changed.
- **B**: a small gallery page served by `Bun.serve`, with no new dependencies.

Both options loaded the same two CSF3 story files (`Button.stories.tsx`, `Modal.stories.tsx`). The prototypes are in `spikes/` on branch `worker/GUZ-1435`. That folder is not committed or merged.

## Recommendation

**A (Storybook), isolated, with the stories kept runnable without Storybook.** This choice needs owner approval for 220 dev packages in a separate folder, and a scoped exception to "Don't use vite" for that folder only.

The reasons:

- A already gives the epic's three surfaces: autodocs pages with prop tables from react-docgen, browser play tests through the Vitest addon, and a static build to take screenshots of. B gives none of these.
- To give the same docs, B would need a hand-written prop table and docgen. That is a growing tool to maintain.
- A works under the Bun runtime (`bun --bun`) for both `storybook build` and `storybook dev`. No Node-only step was found.
- A changes nothing in `app/`. The app keeps 32 packages and its Bun build.

Fallback: if the owner declines the dependencies, use B. Its core is about 120 lines and the stories stay the same.

Guardrail for both options: stories import only types from Storybook (`import type`, erased at build time). Play functions use plain DOM calls, not `storybook/test`. This keeps every play function runnable in `bun test` + happy-dom (the existing GUZ-1207 layer, part of `cargo xtask lint`). It also keeps the runner choice reversible.

## Comparison

All numbers were measured on 2026-11-05 (macOS, Bun 1.4.2, Node 26.7.0) unless marked otherwise. The browser runs (Chromium) were run once each outside the sandbox.

| Measure | A: Storybook 10.6.6 + Vite 8.3.6 | B: Bun gallery |
|---|---|---|
| New packages | 220 installed (204 top level + 16 nested); `bun install` 5.54 s | 0 |
| `node_modules` size | 189 MB (app's own: 98 MB, 32 packages) | 0 (reuses app's `node_modules` via symlink) |
| Config / code to maintain | `package.json` + `main.ts` + `preview.ts` + `vitest.config.ts` + `tsconfig.json`, about 70 lines | `server.ts`, `gallery.tsx`, `csf.tsx`, `build.ts`, `play.test.tsx`, `bunfig.toml`, about 136 lines |
| Static build | `storybook build`: OK with Node (4.2 s) and with `bun --bun` (2.9 s); output 7.7–9.4 MB | `Bun.build` + bun-plugin-tailwind: OK; output 380 KB |
| Dev server | `bun --bun run storybook`: index served after 3 s, Vite HMR client present | `bun server.ts`: page bundled in 63 ms, Bun HMR (`development.hmr`) |
| Tailwind 4 + `tokens.css` | Yes, through `@tailwindcss/vite` 4.1.10. The output CSS has `bg-slate`, `border-rule`, `rounded-button`, `min-h-11`, `text-label`, `shadow-panel`, `backdrop:bg-ink/50`, `--paper` | Yes, through bun-plugin-tailwind. The same classes and tokens are present |
| Docs pages | Autodocs: `ui-button--docs` and `ui-modal--docs` were generated. `__docgenInfo` has prop unions (e.g. `"quiet" \| "primary" \| "danger" \| "link"`) | None. A list of stories only |
| Controls / args editing | Built in | None |
| Play functions in a real browser | Vitest addon, browser mode, Playwright Chromium. 8/8 story tests pass (2 files): 2.04 s with Node (non-fatal DEP0205 warning), 1.63 s with `bun --bun` | Gallery `?play` runs play in the page. Chromium via Playwright: `play: passed` for `ui-modal--open-and-close` |
| Play functions headless (no browser) | Not provided by Storybook | `bun test` + happy-dom: 2 pass / 0 fail (Button disabled clicks, Modal open/close with focus return) |
| Screenshot via existing Playwright | `app`'s `@playwright/test` against `storybook-static/iframe.html?id=ui-modal--open`: OK, 16 KB PNG, only the story | Same Playwright against `?story=ui-modal--open&isolated`: OK, 17 KB PNG. The modal looks the same as A's, but the gallery's story header and canvas strip show behind the backdrop (the isolated mode still needs to drop them) |
| Story IDs | `ui-button--primary`, `ui-modal--open-and-close`, … | The same IDs (Storybook's `title--export-name` rule copied) |
| Type check of stories | `tsc` passes against `@storybook/react-vite` types | Type-only imports need the Storybook types to resolve (see migration) |

### Install findings (A)

- Peer warning: `incorrect peer dependency "vite@8.3.6"`. The cause is `@tailwindcss/vite@4.1.10`, which declares `vite: ^5.2.0 || ^6`. It was pinned to match the app's `tailwindcss@4.1.10`. The build still passed. The other packages accept vite 8:
  - `@storybook/react-vite` and `@storybook/builder-vite`: `^5 || ^6 || ^7 || ^8`
  - `@vitejs/plugin-react@6.2.1`: `^8.0.0`
  - `vitest@5.1.0`: `^6.4 || ^7 || ^8`
  - `@vitest/mocker`: `^6 || ^7 || ^8`
  - `@joshwooding/vite-plugin-react-docgen-typescript`: `^3 … ^8`

  Fix: use `@tailwindcss/vite@4.3.4` (peer `^5.2.0 || ^6 || ^7 || ^8`). The app's `tailwindcss` should move to 4.3.x at the same time, so that one Tailwind version produces the classes.
- Blocked postinstall (`bun pm untrusted`): `@tailwindcss/oxide@4.1.10` runs `node ./scripts/install.js`. It was not trusted or run. The platform binary `@tailwindcss/oxide-darwin-arm64` was installed as an optional dependency, and both builds produced Tailwind CSS. Inferred: the script is a fallback download that this platform does not need.

## Bun / Vite compatibility facts

- Storybook builders are Vite, Webpack and Rspack/Rsbuild. There is no Bun builder. Source: <https://storybook.js.org/docs/builders>.
- Storybook's install requirements list Node.js 20+ and npm 10+, and do not mention Bun. Source: <https://storybook.js.org/docs/get-started/install>. Measured: under `bun --bun`, `storybook build` and `storybook dev` both run. The `storybook` bin has a `#!/usr/bin/env node` shebang, so a plain `bun run` uses Node.
- The Vitest addon needs a Vite-based Storybook framework and Vitest ≥ 3.0, and runs in browser mode with Playwright Chromium. Source: <https://storybook.js.org/docs/writing-tests/integrations/vitest-addon>. The older test-runner works with any framework.
- Bun HTML imports in `Bun.serve` support HMR with `development: { hmr: true }`. Source: `app/node_modules/bun-types/docs/bundler/fullstack.md`.
- B limits (measured or inferred):
  - Bun's browser bundler has no glob import. The gallery `server.ts` writes `stories.gen.ts` with `Bun.Glob` at startup. A new story file needs a server restart; edits to existing files hot-reload (inferred from HMR docs, not exercised).
  - The CSF support covers only `args`, `render`, `play` and `name`. It does not cover decorators, loaders, `argTypes`, `parameters`, or `beforeEach`.
- A layout: the stories resolve `react` from the app's `node_modules`, and Storybook has its own copy, so `main.ts` sets `resolve.dedupe: ["react", "react-dom"]`. `server.fs.allow` opens `app/src`.

## Migration cost (CSF stories are the shared contract)

- **B → A**:
  - Add the Storybook folder: 5 files, about 70 lines.
  - Install with owner approval.
  - Story files do not change.
  - Delete the gallery.
  - Effort: about 1 hour.
- **A → B**:
  - Delete the Storybook folder.
  - Add the gallery: about 136 lines, already prototyped.
  - The stories still work if they keep to the guardrail: plain-DOM play functions and only `args`/`render`/`play`/`name`.
  - Each Storybook-only feature in use (decorators, `storybook/test` `expect`/`userEvent`, `argTypes`, MDX docs) must be removed or rewritten one at a time.
- **Type imports in `app/`** (either way): `import type { Meta, StoryObj } from "@storybook/react-vite"` in `app/src/ui/*.stories.tsx` does not resolve under the app's `tsc`, because the app does not have the package. There are two choices:
  1. Exclude `*.stories.tsx` from the app's `tsconfig` and type-check them from the Storybook folder. This was measured in the spike with a `paths` mapping, and `tsc` passed.
  2. Add a local `Meta`/`StoryObj` type of about 10 lines in `app/src/ui/` for B.

## Not measured

- Hot reload was not exercised by editing a file during a session for A or B. Only the presence of the HMR client was checked.
- WebKit screenshots and baseline flake across macOS and font versions (GUZ-1434 risk).
