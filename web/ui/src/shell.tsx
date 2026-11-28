import type { ReactNode } from "react";

import { formatMessage } from "./i18n.ts";
import { UI_SHELL_SKELETON } from "./skeleton.ts";

export function AppShell(props: { children?: ReactNode }) {
  return (
    <div className="anybot-shell" data-testid="anybot-shell">
      <h1>AnyBot</h1>
      <p>{formatMessage(UI_SHELL_SKELETON)}</p>
      {props.children}
    </div>
  );
}
