import { useEffect, useRef, useState } from "react";

import { getSettings, hidePanel, updateSettings } from "./api";
import { listen } from "./lib/bridge";
import { useLangSync } from "./lib/useLangSync";
import { ItemList } from "./components/ItemList";
import { SnippetList } from "./components/SnippetList";
import { EditPane } from "./components/EditPane";
import { PreviewPane } from "./components/PreviewPane";
import { ResizeGrip } from "./components/ResizeGrip";
import { SearchBar } from "./components/SearchBar";
import { StatusBar } from "./components/StatusBar";
import { ChevronIcon } from "./components/TypeIcon";
import { t } from "./lib/i18n";
import { useClipboardStore } from "./store/useClipboardStore";
import type { AppSettings } from "./types";

export default function App() {
  const refresh = useClipboardStore((s) => s.refresh);
  const reset = useClipboardStore((s) => s.reset);
  const editingId = useClipboardStore((s) => s.editingId);
  const query = useClipboardStore((s) => s.query);

  const [settings, setSettings] = useState<AppSettings | null>(null);
  useLangSync(settings?.language);

  const settingsRef = useRef<AppSettings | null>(null);

  const collapsed = settings?.previewCollapsed ?? false;
  const showPreview = !collapsed || editingId !== null;

  const togglePreview = () => {
    const next = !collapsed;
    if (settingsRef.current) settingsRef.current = { ...settingsRef.current, previewCollapsed: next };
    setSettings((s) => (s ? { ...s, previewCollapsed: next } : s));
    void updateSettings({ previewCollapsed: next })
      .then((s) => {
        settingsRef.current = s;
        setSettings(s);
      })
      .catch(() => {
        if (settingsRef.current) settingsRef.current = { ...settingsRef.current, previewCollapsed: collapsed };
        setSettings((s) => (s ? { ...s, previewCollapsed: collapsed } : s));
      });
  };

  const togglePreviewRef = useRef(togglePreview);
  togglePreviewRef.current = togglePreview;

  const rememberSize = (w: number, h: number) => {
    void updateSettings({ panelW: w, panelH: h })
      .then((s) => {
        settingsRef.current = s;
        setSettings(s);
      })
      .catch(() => {});
  };

  useEffect(() => {
    const syncSettings = () => {
      void getSettings()
        .then((s) => {
          settingsRef.current = s;
          setSettings(s);
        })
        .catch(() => {});
    };
    syncSettings();
    void refresh();

    const unItems = listen("plico://items-changed", () => {
      void refresh();
    });
    const unShown = listen("plico://panel-shown", () => {
      reset();
    });
    const unSettings = listen("plico://settings-changed", syncSettings);

    return () => {
      void unItems.then((f) => f());
      void unShown.then((f) => f());
      void unSettings.then((f) => f());
    };
  }, [refresh, reset]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const state = useClipboardStore.getState();

      if (e.isComposing || state.composing) return;

      const target = e.target as HTMLElement | null;
      const inInput = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA";
      const typing = inInput && (target as HTMLInputElement).value.length > 0;
      const vimMode = settingsRef.current?.vimMode ?? false;

      if (state.editingId !== null) {
        switch (e.key) {
          case "Enter":
            e.preventDefault();
            if (e.ctrlKey || e.metaKey) {
              void state.pasteText(state.editingDraft, true);
            } else {
              void state.pasteText(state.editingDraft, false);
            }
            break;
          case "Escape":
            e.preventDefault();
            state.setEditing(null);
            break;
          default:
            break;
        }
        return;
      }

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          state.moveSelection(1);
          break;

        case "ArrowUp":
          e.preventDefault();
          state.moveSelection(-1);
          break;

        case "Enter":
          e.preventDefault();
          if (query.startsWith("/")) {
            const s = state.snippets.find((x) => x.id === state.selectedSnippetId);
            if (s) void state.pasteSnippet(s.content);
          } else {
            void state.pasteSelected(e.ctrlKey || e.metaKey);
          }
          break;

        case "Tab":
          e.preventDefault();
          state.cycleFilter(e.shiftKey ? -1 : 1);
          break;

        case "Escape":
          e.preventDefault();
          if (state.selectedIds.length > 1) {
            state.clearMultiSelect();
            break;
          }
          if (!state.clearQuery()) {
            void hidePanel();
          }
          break;

        case "a":
        case "A":
          if (!e.ctrlKey && !e.metaKey) return;
          if (inInput) return;
          e.preventDefault();
          state.selectAllVisible();
          break;

        case "b":
        case "B":
          if (!e.ctrlKey && !e.metaKey) return;
          if (inInput) return;
          e.preventDefault();
          togglePreviewRef.current();
          break;

        case "e":
        case "E":
          if (!e.ctrlKey && !e.metaKey) return;
          e.preventDefault();
          state.beginEditSelected();
          break;

        case "Delete":
          if (typing) return;
          e.preventDefault();
          void state.deleteSelection();
          break;

        case "p":
        case "P":
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            state.moveSelection(-1);
            break;
          }
          if (typing || e.altKey) return;
          e.preventDefault();
          void state.togglePinSelected();
          break;

        case "n":
        case "N":
          if (!e.ctrlKey && !e.metaKey) return;
          e.preventDefault();
          state.moveSelection(1);
          break;

        case "j":
        case "J":
          if (!vimMode || typing || e.ctrlKey || e.metaKey || e.altKey) return;
          e.preventDefault();
          state.moveSelection(1);
          break;

        case "k":
        case "K":
          if (!vimMode || typing || e.ctrlKey || e.metaKey || e.altKey) return;
          e.preventDefault();
          state.moveSelection(-1);
          break;

        default:
          break;
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [query]);

  return (
    <div className="pl-panel">
      <SearchBar />
      <div className={`pl-body${showPreview ? "" : " is-preview-collapsed"}`}>
        {useClipboardStore.getState().query.startsWith("/") ? <SnippetList /> : <ItemList />}

        <div className={`pl-divider-v${showPreview ? "" : " is-collapsed"}`}>
          <button
            type="button"
            className="pl-divider-toggle"
            title={`${collapsed ? t("previewExpand") : t("previewCollapse")}（Ctrl+B）`}
            aria-label={collapsed ? t("previewExpand") : t("previewCollapse")}
            aria-expanded={showPreview}
            onClick={togglePreview}
          >
            <ChevronIcon dir={showPreview ? "right" : "left"} />
          </button>
        </div>

        {showPreview && (editingId !== null ? <EditPane /> : <PreviewPane />)}
      </div>
      <StatusBar />
      <ResizeGrip previewExpanded={showPreview} onResized={rememberSize} />
    </div>
  );
}
