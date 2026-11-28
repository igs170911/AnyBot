import { registerSW } from "virtual:pwa-register";
import { createAnyBotClient } from "@anybot/api-client";
import { APP_ROOT_MOUNT_MISSING } from "@anybot/ui";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";
import "./shell.css";

void registerSW({ immediate: true });

const container = document.getElementById("root");
if (container === null) {
  throw new Error(APP_ROOT_MOUNT_MISSING);
}

const client = createAnyBotClient({ clientVersion: __APP_VERSION__ });

createRoot(container).render(
  <StrictMode>
    <App client={client} />
  </StrictMode>,
);
