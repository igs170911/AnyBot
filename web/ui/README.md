# `web/ui` · shared UI component library (skeleton)

Per `spec/06` section 2.4 and `spec/07` section 2: desktop (`apps/desktop`) and
phone/browser (`apps/web`) **share the same UI**; shared components live here.

M2 is a skeleton: one shell component in `src/shell.tsx`, **no pages, no
routing, no features**. Later feature screens are added to this package and used
by both sides.

- Build: `vite build` (lib mode, outputs `dist/index.js`).
- Tests: `vitest run` (uses `renderToString` from `react-dom/server`, no browser needed).
- Desktop and mobile tests are run together with Playwright end-to-end in later milestones (`spec/06` section 2.7).
