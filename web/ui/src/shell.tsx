import { UI_SHELL_SKELETON } from "./skeleton.ts";

export function AppShell() {
  return (
    <div className="anybot-shell" data-testid="anybot-shell">
      <h1>AnyBot</h1>
      <p data-message-code={UI_SHELL_SKELETON}>UI skeleton: no features (M2).</p>
    </div>
  );
}
