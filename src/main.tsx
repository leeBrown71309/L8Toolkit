import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./styles/tokens.css";
import "./styles/base.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { resolveWindowKind, WindowRouter } from "./app/window-router";
import { isTauriRuntime } from "./shared/backend";

const container = document.getElementById("root");
if (!container) {
  throw new Error("The #root container is missing from index.html");
}

const windowKind = resolveWindowKind();
document.documentElement.dataset.window = windowKind;
if (!isTauriRuntime) document.documentElement.dataset.preview = "true";

createRoot(container).render(
  <StrictMode>
    <WindowRouter windowKind={windowKind} />
  </StrictMode>,
);
