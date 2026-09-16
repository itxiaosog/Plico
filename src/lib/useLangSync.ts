import { useEffect } from "react";

import { cacheLangPref, langOverride, resolveLanguage, type LangPref } from "./lang";
import { setLang } from "./i18n";

/**
 * 把设置里的语言偏好同步到 `<html lang>` 与 `t()` 的默认语言。
 *
 * 面板和设置窗口都要挂这个 hook —— 两个窗口是两个 webview，语言状态不共享。
 * 设置窗口改了语言后会广播 `plico://settings-changed`，面板收到后重新拉设置，
 * 这个 hook 的依赖随之变化，界面文案也就跟着换了。
 *
 * `pref` 为 undefined 表示设置还没加载完，此时不动手 —— 首屏已经由
 * `initialLang()` 用本地缓存顶上，等真实值到了再覆盖。
 */
export function useLangSync(pref: string | undefined): void {
  useEffect(() => {
    // `?lang=` 是预览用的强制覆盖：此时直接落地覆盖值，不跟随设置，也不污染缓存
    const override = langOverride();
    if (override) {
      setLang(override);
      document.documentElement.lang = override === "zh" ? "zh-CN" : "en";
      return;
    }
    if (!pref) return;

    const p: LangPref = pref === "zh" || pref === "en" || pref === "system" ? pref : "system";
    cacheLangPref(p);
    const lang = resolveLanguage(p);
    setLang(lang);
    document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  }, [pref]);
}
