import { createRoot } from "react-dom/client";
import { App } from "./app.tsx";
import { loadEdl } from "./edl/index.ts";
import { scriptURL } from "./trusted.ts";

// GitHub Pages cannot send frame-ancestors; refuse to run inside a frame.
if (window.top !== window.self) {
  document.documentElement.replaceChildren();
  throw new Error("verbatim does not run inside frames");
}

const root = document.getElementById("root");
// The cut and export engine is small and needed everywhere: load it before the first render.
void loadEdl().then(() => {
  if (root) createRoot(root).render(<App />);
});

const secure = location.protocol === "https:" || location.hostname === "localhost" || location.hostname === "127.0.0.1";
if ("serviceWorker" in navigator && secure) {
  void navigator.serviceWorker.register(scriptURL("./sw.js"), { scope: "./" }).catch(() => {});
}
