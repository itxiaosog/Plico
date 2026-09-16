import { useEffect, useState, type ReactElement } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauri, listen } from "../lib/bridge";
import { t } from "../lib/i18n";
import { useLangSync } from "../lib/useLangSync";
import { ErrorBanner } from "./controls";
import { AboutPage } from "./pages/AboutPage";
import { BehaviorPage } from "./pages/BehaviorPage";
import { GeneralPage } from "./pages/GeneralPage";
import { HotkeyPage } from "./pages/HotkeyPage";
import { PrivacyPage } from "./pages/PrivacyPage";
import { SnippetsPage } from "./pages/SnippetsPage";
import { TagsPage } from "./pages/TagsPage";
import { StoragePage } from "./pages/StoragePage";
import { useSettingsStore } from "./useSettingsStore";

type PageId = "general" | "hotkey" | "storage" | "privacy" | "behavior" | "snippets" | "tags" | "about";

const ICONS: Record<PageId, ReactElement> = {
  general: (
    <>
      <path d="M2.5 4.5h11M2.5 11.5h11" />
      <circle cx="6" cy="4.5" r="1.6" />
      <circle cx="10.5" cy="11.5" r="1.6" />
    </>
  ),
  hotkey: (
    <>
      <rect x="1.8" y="4" width="12.4" height="8" rx="1.6" />
      <path d="M4.4 6.6h.01M6.8 6.6h.01M9.2 6.6h.01M11.6 6.6h.01M5.6 9.6h4.8" />
    </>
  ),
  storage: (
    <>
      <ellipse cx="8" cy="4.2" rx="5.2" ry="2.2" />
      <path d="M2.8 4.2v7.6c0 1.2 2.3 2.2 5.2 2.2s5.2-1 5.2-2.2V4.2" />
      <path d="M2.8 8c0 1.2 2.3 2.2 5.2 2.2s5.2-1 5.2-2.2" />
    </>
  ),
  privacy: (
    <>
      <path d="M8 1.8l5 1.9v4.1c0 2.9-2 5.4-5 6.4-3-1-5-3.5-5-6.4V3.7z" />
      <path d="M6 8l1.5 1.5L10.2 6.8" />
    </>
  ),
  behavior: (
    <>
      <rect x="5.2" y="1.8" width="5.6" height="12.4" rx="2.8" />
      <path d="M8 4.6v2.4" />
    </>
  ),
  snippets: (<><rect x="2" y="2" width="12" height="12" rx="2" /><path d="M5 5h6M5 8h6M5 11h4" /></>),
  tags: (<><path d="M2 3h6l6 6-5 5-6-6z" /><path d="M5 6h.01" /></>),
  about: (    <>
      <circle cx="8" cy="8" r="6.2" />
      <path d="M8 7.2v4" />
      <path d="M8 5.1h.01" />
    </>
  ),
};

const PAGES: { id: PageId; label(): string; render(): ReactElement }[] = [
  { id: "general", label: () => t("navGeneral"), render: () => <GeneralPage /> },
  { id: "hotkey", label: () => t("navHotkey"), render: () => <HotkeyPage /> },
  { id: "storage", label: () => t("navStorage"), render: () => <StoragePage /> },
  { id: "privacy", label: () => t("navPrivacy"), render: () => <PrivacyPage /> },
  { id: "behavior", label: () => t("navBehavior"), render: () => <BehaviorPage /> },
  { id: "snippets", label: () => t("navSnippets"), render: () => <SnippetsPage /> },
  { id: "tags", label: () => t("navTags"), render: () => <TagsPage /> },
  { id: "about", label: () => t("navAbout"), render: () => <AboutPage /> },
];

export default function SettingsApp() {
  const [page, setPage] = useState<PageId>("general");

  const settings = useSettingsStore((s) => s.settings);
  const loading = useSettingsStore((s) => s.loading);
  const error = useSettingsStore((s) => s.error);
  const clearError = useSettingsStore((s) => s.clearError);
  const load = useSettingsStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const un = listen("plico://settings-changed", () => {
      void load();
    });
    return () => {
      void un.then((f) => f());
    };
  }, [load]);

  useLangSync(settings?.language);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || !isTauri) return;

      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;

      e.preventDefault();
      void getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const current = PAGES.find((p) => p.id === page) ?? PAGES[0];

  return (
    <div className="pl-set">
      <aside className="pl-set__nav">
        <div className="pl-set__brand">
          Plico <span>{t("brandSettings")}</span>
        </div>
        <nav className="pl-set__tabs">
          {PAGES.map((p) => (
            <button
              key={p.id}
              type="button"
              className={`pl-set__tab${p.id === page ? " is-active" : ""}`}
              data-page={p.id}
              onClick={() => setPage(p.id)}
            >
              <svg
                viewBox="0 0 16 16"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.8"
                strokeLinecap="square"
                strokeLinejoin="miter"
                aria-hidden="true"
              >
                {ICONS[p.id]}
              </svg>
              {p.label()}
            </button>
          ))}
        </nav>
      </aside>

      <main className="pl-set__main">
        {error && <ErrorBanner text={error} onClose={clearError} />}
        <div className="pl-set__scroll">
          {settings ? (
            current?.render()
          ) : (
            <div className="pl-set__loading">{loading ? t("loading") : t("settingsUnavailable")}</div>
          )}
        </div>
      </main>
    </div>
  );
}
