import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { PopupBar } from "./components/PopupBar";
import { desktop } from "./ipc";
import { installWindowChrome } from "./lib/window";
import "./styles.css";

const popup = new URLSearchParams(window.location.search).has("popup");
if (!popup) installWindowChrome(desktop);
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{popup ? <PopupBar /> : <App />}</React.StrictMode>,
);
