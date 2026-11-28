import { useAccountBots } from "../sync/useAccountBots.ts";
import { useRecords } from "../sync/useRecords.ts";

export function ChatRoute(props: { botId: string }) {
  const bots = useAccountBots();
  const feed = useRecords({ kind: "bot", id: props.botId });
  const name = bots.find((bot) => bot.id === props.botId)?.name ?? "";
  return (
    <main className="anybot-chat" data-route="chat" data-record-count={feed.records.length}>
      <h2>{name}</h2>
    </main>
  );
}
