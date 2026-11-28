import { useEffect, useState } from "react";

import { parseRoute, type Route, routeHref } from "./route.ts";

export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parseRoute(globalThis.location?.hash ?? ""));
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(globalThis.location.hash));
    globalThis.addEventListener("hashchange", onChange);
    onChange();
    return () => globalThis.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}

export function replaceRoute(route: Route): void {
  const target = routeHref(route);
  if (globalThis.location.hash !== target) {
    globalThis.location.replace(target);
  }
}
