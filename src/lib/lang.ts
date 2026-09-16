export type Lang = "zh" | "en";
export type LangPref = "system" | Lang;

export function resolveLanguage(pref: LangPref): Lang {
  if (pref !== "system") return pref;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export function langOverride(): Lang | null {
  const param = new URLSearchParams(window.location.search).get("lang");
  return param === "zh" || param === "en" ? param : null;
}

export function initialLang(): Lang {
  return langOverride() ?? resolveLanguage(cachedLangPref());
}

const CACHE_KEY = "plico.lang";

export function cacheLangPref(pref: LangPref): void {
  try {
    localStorage.setItem(CACHE_KEY, pref);
  } catch {
  }
}

function cachedLangPref(): LangPref {
  try {
    const v = localStorage.getItem(CACHE_KEY);
    if (v === "zh" || v === "en" || v === "system") return v;
  } catch {
  }
  return "system";
}

export function hasLangOverride(): boolean {
  return langOverride() !== null;
}
