import { describe, expect, it } from "vitest";

import { guardRoute, parseRoute, routeHref } from "./route.ts";

describe("route", () => {
  it("parses hashes into routes", () => {
    expect(parseRoute("")).toEqual({ name: "bots" });
    expect(parseRoute("#/login")).toEqual({ name: "login" });
    expect(parseRoute("#/bots/a%20b")).toEqual({ name: "chat", botId: "a b" });
    expect(parseRoute("#/nowhere")).toEqual({ name: "missing" });
  });

  it("round-trips chat routes", () => {
    const route = { name: "chat", botId: "x/y" } as const;
    expect(parseRoute(routeHref(route))).toEqual(route);
  });

  it("signed out visitors are sent to login and signed in ones away from it", () => {
    expect(guardRoute({ name: "bots" }, false)).toEqual({ name: "login" });
    expect(guardRoute({ name: "login" }, true)).toEqual({ name: "bots" });
    expect(guardRoute({ name: "chat", botId: "1" }, true)).toEqual({ name: "chat", botId: "1" });
  });
});
