import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import App from "./App";
import SettingsApp from "./settings/SettingsApp";
import { isTauri } from "./lib/bridge";
import { initialLang } from "./lib/lang";
import { setLang } from "./lib/i18n";
import "./styles/base.css";
import "./styles/tokens.css";
import "./styles/app.css";
import "./settings/settings.css";

type Surface = "panel" | "settings";

function resolveSurface(): Surface {
  if (isTauri) {
    try {
      return getCurrentWindow().label === "settings" ? "settings" : "panel";
    } catch {
      return "panel";
    }
  }
  return new URLSearchParams(window.location.search).get("window") === "settings"
    ? "settings"
    : "panel";
}

const surface = resolveSurface();

if (!isTauri) {
  document.body.classList.add(surface === "settings" ? "pl-browser-set" : "pl-browser");
}

setLang(initialLang());

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(surface === "settings" ? <SettingsApp /> : <App />);
}
