import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { desktop } from "./ipc";
import "./styles.css";

// The desktop window is transparent over native macOS materials; the browser
// preview keeps an opaque window background.
if (desktop && navigator.platform.includes("Mac"))
  document.documentElement.classList.add("desktop");
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
