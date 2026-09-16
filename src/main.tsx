import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import App from "./App";
import SettingsApp from "./settings/SettingsApp";
import { isTauri } from "./lib/bridge";
import { initialLang } from "./lib/lang";
import { setLang } from "./lib/i18n";
// pxlkit + Tailwind 基座必须最先引入（内含 @import 顺序约定与像素字体
// @font-face，字体文件内嵌在本包里，零网络）。
import "./styles/base.css";
import "./styles/tokens.css";
import "./styles/app.css";
import "./settings/settings.css";

/**
 * 面板和设置窗口共用同一份前端产物，靠窗口 label 决定渲染哪一屏。
 *
 * 这样比配两套 Vite 入口简单得多：多入口要动 `build.rollupOptions.input`，
 * 还要维护两份 index.html，而这里多出来的开销只是设置窗口也加载了面板的
 * 组件代码（几十 KB，且因为体积小没有拆包的必要）。
 */
type Surface = "panel" | "settings";

function resolveSurface(): Surface {
  if (isTauri) {
    try {
      return getCurrentWindow().label === "settings" ? "settings" : "panel";
    } catch {
      return "panel";
    }
  }
  // 浏览器预览：?window=settings 直接看设置界面
  return new URLSearchParams(window.location.search).get("window") === "settings"
    ? "settings"
    : "panel";
}

const surface = resolveSurface();

// 在浏览器里预览时给个中性背景，否则面板的圆角和阴影看不出来
if (!isTauri) {
  document.body.classList.add(surface === "settings" ? "pl-browser-set" : "pl-browser");
}

// 单一像素主题：不再有需要提前落地的主题偏好（主题系统已随 pxlkit 改版移除）。
// 语言：先用缓存/系统值让 t() 有默认语言，等设置加载完由 useLangSync 覆盖。
setLang(initialLang());

// 刻意不用 StrictMode：它会双调用 effect，导致 Tauri 事件监听被注册两次，
// 在开发模式下出现重复刷新。
const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(surface === "settings" ? <SettingsApp /> : <App />);
}
