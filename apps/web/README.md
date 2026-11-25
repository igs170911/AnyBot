# `apps/web` · PWA (phones and browsers, skeleton)

Per `spec/06` section 2.4: phones do not get a native app (A19); the PWA is
used (Service Worker and Web Push included).

M2 is a skeleton: the UI is only the `web/ui` shell, **no pages, no routing, no
features**; data fetching (TanStack Query) and the API client arrive with the
milestone that generates `api/openapi.yaml`.

```sh
pnpm --filter @anybot/web build     # dist/ + sw.js + manifest.webmanifest
pnpm --filter @anybot/web preview   # view the PWA locally
```

The Service Worker uses `vite-plugin-pwa` (`registerType: "autoUpdate"`);
Web Push implementation is left for later (nothing is guessed before the spec says it).
