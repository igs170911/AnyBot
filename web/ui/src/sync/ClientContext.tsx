import type { AnyBotClient } from "@anybot/api-client";
import { createContext, type ReactNode, useContext } from "react";

const ClientContext = createContext<AnyBotClient | null>(null);

export const CLIENT_MISSING = "AnyBot client provider is missing";

export function ClientProvider(props: { client: AnyBotClient; children: ReactNode }) {
  return <ClientContext.Provider value={props.client}>{props.children}</ClientContext.Provider>;
}

export function useClient(): AnyBotClient {
  const client = useContext(ClientContext);
  if (client === null) {
    throw new Error(CLIENT_MISSING);
  }
  return client;
}
