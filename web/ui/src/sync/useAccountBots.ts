import { useMemo } from "react";

import { type BotEntry, botsFromRecords } from "./accountBots.ts";
import { useRecords } from "./useRecords.ts";

const ACCOUNT_STREAM = { kind: "account" } as const;

export function useAccountBots(): BotEntry[] {
  const feed = useRecords(ACCOUNT_STREAM);
  return useMemo(() => botsFromRecords(feed.records), [feed.records]);
}
