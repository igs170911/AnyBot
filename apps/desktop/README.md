# `apps/desktop` · desktop shell (Tauri 2, skeleton)

Per `spec/06` section 2.4 and `spec/07` sections 2, 3:

- **The window is only a view.** The background program `apps/agent` does the
  work; the desktop shell must not depend on `server`, `store-server`,
  `runner`, `models`, `tools`, `computers`, `mcp`, `external-agents`
  (enforced by `tools/depcheck`).
- The UI is the same set shared with mobile: the frontend only loads the
  `web/ui` shell (M2 skeleton, no pages, routing, or features).
- `src-tauri/` is a Rust crate added to the Cargo workspace (`anybot-app-desktop`).

## Running it (the M2 skeleton has no features; it opens one window)

```sh
pnpm install
cargo install --locked tauri-cli   # development tool allowed by AGENTS.md
cargo tauri dev                    # frontend vite (port 1420) + the Tauri window
cargo tauri build                  # local packaging stays off (bundle.active=false); CI turns it on
```

`make check` does not include Tauri packaging: the Rust part of the desktop
shell goes through the original M1 gates (fmt, clippy, nextest, deny, machete,
depcheck), the frontend goes through the TypeScript gates (biome, tsc, vitest,
vite build).

## Things to watch (`spec/06` section 2.4)

- WebKitGTK on Linux is heavier on resources and renders less consistently; it
  needs extra testing (CI installs the system packages).
- Long chats render only the visible messages and keep no resident animations:
  these are rules for when the UI is written later.
- Resource-limit gates (idle window CPU near 0, memory within 200 MB) come when
  the desktop has real content.
