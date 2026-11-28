import { useEffect, useMemo, useState } from "react";

import { useClient } from "./ClientContext.tsx";
import { createFeedSource } from "./feedSource.ts";
import { EMPTY_FEED, type FeedState, RecordsFeed } from "./RecordsFeed.ts";
import { refOfIdentity, type StreamRef, streamIdentity } from "./records.ts";

export function useRecords(stream: StreamRef): FeedState {
  const client = useClient();
  const identity = streamIdentity(stream);
  const source = useMemo(() => createFeedSource(client), [client]);
  const [state, setState] = useState<FeedState>(EMPTY_FEED);
  useEffect(() => {
    setState(EMPTY_FEED);
    const feed = new RecordsFeed({ source, stream: refOfIdentity(identity), onChange: setState });
    feed.start();
    return () => feed.stop();
  }, [source, identity]);
  return state;
}
