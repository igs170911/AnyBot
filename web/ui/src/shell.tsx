/**
 * `web/ui` · 共用的畫面元件庫（React），桌面和手機都用（`spec/07` §2、`spec/06` §2.4）。
 *
 * M2 是骨架：這裡只有一個「外殼」元件，沒有任何頁面、路由、功能邏輯。
 * 之後的功能畫面都加進這個套件，桌面（apps/desktop）和 PWA（apps/web）共用同一套畫面。
 */
export function AppShell() {
  return (
    <div className="anybot-shell" data-testid="anybot-shell">
      <h1>AnyBot</h1>
      <p>畫面骨架：沒有功能（M2）。</p>
    </div>
  );
}
