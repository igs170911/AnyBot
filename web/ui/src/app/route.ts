export type Route =
  | { readonly name: "login" }
  | { readonly name: "bots" }
  | { readonly name: "chat"; readonly botId: string }
  | { readonly name: "missing" };

const BOT_PREFIX = "/bots/";

export function parseRoute(hash: string): Route {
  const path = hash.replace(/^#/, "");
  if (path === "" || path === "/") {
    return { name: "bots" };
  }
  if (path === "/login") {
    return { name: "login" };
  }
  if (path.startsWith(BOT_PREFIX) && path.length > BOT_PREFIX.length) {
    return { name: "chat", botId: decodeURIComponent(path.slice(BOT_PREFIX.length)) };
  }
  return { name: "missing" };
}

export function routeHref(route: Route): string {
  switch (route.name) {
    case "login":
      return "#/login";
    case "chat":
      return `#${BOT_PREFIX}${encodeURIComponent(route.botId)}`;
    case "bots":
    case "missing":
      return "#/";
  }
}

export function guardRoute(route: Route, signedIn: boolean): Route {
  if (!signedIn) {
    return { name: "login" };
  }
  return route.name === "login" || route.name === "missing" ? { name: "bots" } : route;
}
