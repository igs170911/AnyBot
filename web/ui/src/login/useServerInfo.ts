import { clientAt } from "@anybot/api-client";
import { useEffect, useState } from "react";

import { useClient } from "../sync/ClientContext.tsx";
import { checkServer, originOf, type ServerInfo } from "./serverCheck.ts";

export function useServerInfo(url: string): ServerInfo | null {
  const client = useClient();
  const [info, setInfo] = useState<ServerInfo | null>(null);
  useEffect(() => {
    const origin = originOf(url);
    if (origin === null) {
      setInfo(null);
      return;
    }
    let alive = true;
    void checkServer(clientAt(client, origin), url).then((check) => {
      if (alive) {
        setInfo(check.ok ? check.info : null);
      }
    });
    return () => {
      alive = false;
    };
  }, [client, url]);
  return info;
}
