import { useEffect } from "react";

import { cacheLangPref, langOverride, resolveLanguage, type LangPref } from "./lang";
import { setLang } from "./i18n";

export function useLangSync(pref: string | undefined): void {
  useEffect(() => {
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
