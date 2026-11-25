import { formatMessage } from "./i18n.ts";
import { UI_SHELL_SKELETON } from "./skeleton.ts";

export function AppShell() {
  return (
    <div className="anybot-shell" data-testid="anybot-shell">
      <h1>AnyBot</h1>
      <p>{formatMessage(UI_SHELL_SKELETON)}</p>
    </div>
  );
}
