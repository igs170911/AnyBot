import { useEffect } from "react";

import { BotsRoute } from "../routes/BotsRoute.tsx";
import { ChatRoute } from "../routes/ChatRoute.tsx";
import { LoginRoute } from "../routes/LoginRoute.tsx";
import { useSession } from "../session/SessionProvider.tsx";
import { guardRoute, type Route } from "./route.ts";
import { replaceRoute, useRoute } from "./useRoute.ts";

function screenFor(route: Route) {
  switch (route.name) {
    case "login":
      return <LoginRoute />;
    case "chat":
      return <ChatRoute botId={route.botId} />;
    case "bots":
    case "missing":
      return <BotsRoute />;
  }
}

export function AppRoutes() {
  const session = useSession();
  const requested = useRoute();
  const signedIn = session.status === "signedIn";
  const effective = guardRoute(requested, signedIn);
  const known = session.status !== "unknown";
  useEffect(() => {
    if (known) {
      replaceRoute(effective);
    }
  }, [known, effective]);
  if (!known) {
    return null;
  }
  return screenFor(effective);
}
