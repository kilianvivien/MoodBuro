import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { desktop } from "./ipc";
import "./styles.css";

// macOS overlays its native traffic lights; reserve matching shell chrome.
if (desktop && navigator.platform.includes("Mac"))
  document.documentElement.classList.add("desktop");
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
