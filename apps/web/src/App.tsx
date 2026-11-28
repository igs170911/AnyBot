import type { AnyBotClient } from "@anybot/api-client";
import { AppRoutes, AppShell, ClientProvider, SessionProvider } from "@anybot/ui";

export function App(props: { client: AnyBotClient }) {
  return (
    <ClientProvider client={props.client}>
      <SessionProvider>
        <AppShell>
          <AppRoutes />
        </AppShell>
      </SessionProvider>
    </ClientProvider>
  );
}
