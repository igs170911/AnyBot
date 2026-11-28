import { attempt } from "@anybot/api-client";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

import { useClient } from "../sync/ClientContext.tsx";
import { readSessionHint, writeSessionHint } from "./sessionHint.ts";

export type SessionStatus = "unknown" | "signedOut" | "signedIn";

export interface SessionValue {
  readonly status: SessionStatus;
  readonly account: string | null;
  markSignedIn(account: string | null): void;
  markSignedOut(): void;
}

interface SessionState {
  readonly status: SessionStatus;
  readonly account: string | null;
}

const SessionContext = createContext<SessionValue | null>(null);

export const SESSION_MISSING = "session provider is missing";

export function SessionProvider(props: { children: ReactNode }) {
  const client = useClient();
  const [state, setState] = useState<SessionState>({ status: "unknown", account: null });
  useEffect(() => {
    if (!readSessionHint()) {
      setState({ status: "signedOut", account: null });
      return;
    }
    let alive = true;
    void attempt(() => client.GET("/v1/settings")).then((result) => {
      if (alive) {
        writeSessionHint(result.ok);
        setState({ status: result.ok ? "signedIn" : "signedOut", account: null });
      }
    });
    return () => {
      alive = false;
    };
  }, [client]);
  const markSignedIn = useCallback((account: string | null) => {
    writeSessionHint(true);
    setState({ status: "signedIn", account });
  }, []);
  const markSignedOut = useCallback(() => {
    writeSessionHint(false);
    setState({ status: "signedOut", account: null });
  }, []);
  const value = useMemo(
    () => ({ ...state, markSignedIn, markSignedOut }),
    [state, markSignedIn, markSignedOut],
  );
  return <SessionContext.Provider value={value}>{props.children}</SessionContext.Provider>;
}

export function useSession(): SessionValue {
  const value = useContext(SessionContext);
  if (value === null) {
    throw new Error(SESSION_MISSING);
  }
  return value;
}
