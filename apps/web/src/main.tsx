import { registerSW } from "virtual:pwa-register";
import { APP_ROOT_MOUNT_MISSING } from "@anybot/ui";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";

void registerSW({ immediate: true });

const container = document.getElementById("root");
if (container === null) {
  throw new Error(APP_ROOT_MOUNT_MISSING);
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
