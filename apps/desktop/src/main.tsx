import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("找不到 #root：index.html 少了掛載點");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
