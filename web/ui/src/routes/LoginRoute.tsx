import { formatMessage } from "../i18n.ts";

export function LoginRoute() {
  return (
    <main className="anybot-login" data-route="login">
      <button type="submit">{formatMessage("login-submit")}</button>
    </main>
  );
}
