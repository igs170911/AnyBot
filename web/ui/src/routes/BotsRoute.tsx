import { routeHref } from "../app/route.ts";
import { useAccountBots } from "../sync/useAccountBots.ts";

export function BotsRoute() {
  const bots = useAccountBots();
  return (
    <main className="anybot-bots" data-route="bots">
      <ul data-role="bot-list">
        {bots.map((bot) => (
          <li key={bot.id}>
            <a href={routeHref({ name: "chat", botId: bot.id })}>{bot.name}</a>
          </li>
        ))}
      </ul>
    </main>
  );
}
