import { refreshKeyboardLayout } from "./utils/keyboardLayout";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Resolve labels before rendering any profile keys.
void refreshKeyboardLayout().then(() => {
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
});
