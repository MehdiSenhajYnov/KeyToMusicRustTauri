import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

// Show window after first paint (double-rAF ensures paint is committed)
requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    document.getElementById("root")!.classList.add("loaded");
  });
});
