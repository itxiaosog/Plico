/**
 * 界面语言（F22）。
 *
 * 用户偏好三态：system / zh / en。`system` 由这里按系统语言解析成两态之一，
 * 文案层永远只面对 `Lang`（"zh" | "en"），与主题层只面对 light/dark 是同一个模式。
 *
 * 文案表是「key → 两种语言」的扁平结构，不走 ICU MessageFormat ——
 * 目前所有文案都是静态字符串或简单拼接，没必要引入变量插值的开销。
 */

export type Lang = "zh" | "en";
export type LangPref = "system" | Lang;

/** 把三态偏好解析成实际语言。 */
export function resolveLanguage(pref: LangPref): Lang {
  if (pref !== "system") return pref;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

/** `?lang=` 覆盖，只在浏览器预览里用，方便不打开设置就验证英文。 */
export function langOverride(): Lang | null {
  const param = new URLSearchParams(window.location.search).get("lang");
  return param === "zh" || param === "en" ? param : null;
}

/** 面板/设置窗口启动时先写入 `<html lang>`，等设置加载完再由调用方以真实值覆盖。 */
export function initialLang(): Lang {
  return langOverride() ?? resolveLanguage(cachedLangPref());
}

const CACHE_KEY = "plico.lang";

export function cacheLangPref(pref: LangPref): void {
  try {
    localStorage.setItem(CACHE_KEY, pref);
  } catch {
    // 同 theme：隐私模式下 localStorage 可能不可写，语言只是观感，不报错
  }
}

function cachedLangPref(): LangPref {
  try {
    const v = localStorage.getItem(CACHE_KEY);
    if (v === "zh" || v === "en" || v === "system") return v;
  } catch {
    // 同上
  }
  return "system";
}

/** 是否存在 `?lang=` 覆盖。有覆盖时语言不跟随设置，否则设置一加载就把覆盖值冲掉。 */
export function hasLangOverride(): boolean {
  return langOverride() !== null;
}
