import { clientAt } from "@anybot/api-client";
import { useCallback, useMemo, useState } from "react";

import { useSession } from "../session/SessionProvider.tsx";
import { useClient } from "../sync/ClientContext.tsx";
import { type LoginContext, runLogin } from "./flow.ts";
import type { LoginInput } from "./form.ts";
import type { Failure } from "./problemText.ts";

export interface LoginState {
  readonly busy: boolean;
  readonly failure: Failure | null;
  readonly elsewhere: boolean;
  submit(input: LoginInput): Promise<void>;
}

export function useLogin(): LoginState {
  const client = useClient();
  const session = useSession();
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [elsewhere, setElsewhere] = useState(false);
  const context = useMemo<LoginContext>(
    () => ({
      origin: globalThis.location.origin,
      clientAt: (url) => clientAt(client, url),
    }),
    [client],
  );
  const submit = useCallback(
    async (input: LoginInput) => {
      setBusy(true);
      setFailure(null);
      setElsewhere(false);
      const outcome = await runLogin(context, input);
      setBusy(false);
      if (outcome.kind === "signedIn") {
        session.markSignedIn(outcome.account);
      } else if (outcome.kind === "elsewhere") {
        setElsewhere(true);
      } else {
        setFailure(outcome.failure);
      }
    },
    [context, session],
  );
  return { busy, failure, elsewhere, submit };
}
