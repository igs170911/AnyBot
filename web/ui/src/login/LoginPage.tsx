import { type FormEvent, useState } from "react";

import { formatMessage } from "../i18n.ts";
import type { LoginInput } from "./form.ts";
import { LoginFields } from "./LoginFields.tsx";
import { useLogin } from "./useLogin.ts";
import { useServerInfo } from "./useServerInfo.ts";

function initialInput(): LoginInput {
  return {
    url: globalThis.location.origin,
    account: "",
    password: "",
    createAccount: false,
    invite: "",
  };
}

export function LoginPage() {
  const [input, setInput] = useState<LoginInput>(initialInput);
  const [acknowledged, setAcknowledged] = useState(false);
  const [ackMissing, setAckMissing] = useState(false);
  const login = useLogin();
  const info = useServerInfo(input.url);
  const privacyAdmin = info?.adminAccount ?? null;
  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    const blocked = input.createAccount && privacyAdmin !== null && !acknowledged;
    setAckMissing(blocked);
    if (!blocked) {
      void login.submit(input);
    }
  };
  return (
    <form className="anybot-login" data-route="login" onSubmit={onSubmit}>
      <LoginFields
        value={input}
        privacyAdmin={privacyAdmin}
        acknowledged={acknowledged}
        onChange={setInput}
        onAcknowledge={setAcknowledged}
      />
      {ackMissing ? <p data-role="login-error">{formatMessage("login-ack-required")}</p> : null}
      {login.failure === null ? null : (
        <p data-role="login-error">{formatMessage(login.failure.code, login.failure.params)}</p>
      )}
      {login.elsewhere ? (
        <p data-role="login-elsewhere">{formatMessage("login-other-origin")}</p>
      ) : null}
      <button type="submit" disabled={login.busy}>
        {formatMessage("login-submit")}
      </button>
    </form>
  );
}
