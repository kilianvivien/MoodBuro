import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { desktop } from "./ipc";
import { installWindowChrome } from "./lib/window";
import "./styles.css";

installWindowChrome(desktop);
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
