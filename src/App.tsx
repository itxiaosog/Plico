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

  // 面板要用到的设置：语言（即时生效）+ Vim 模式（F5）。
  const [settings, setSettings] = useState<AppSettings | null>(null);
  useLangSync(settings?.language);

  // 键盘处理器的依赖数组是空的（它每次从 store 现取状态），所以设置也得走 ref，
  // 否则处理器会一直看到挂载时那个 null。
  const settingsRef = useRef<AppSettings | null>(null);

  const collapsed = settings?.previewCollapsed ?? false;
  // 编辑模式下预览区换成了编辑器，这时候不能折叠 —— 否则用户看不见自己在改什么
  const showPreview = !collapsed || editingId !== null;

  /**
   * 折叠 / 展开预览区（F5）。
   *
   * 先改本地状态再发请求：折叠是个即时反馈的交互，等一次 IPC 往返会顿一下。
   * 后端返回的是权威设置，回来之后再覆盖一次（如果钳制了什么）。
   */
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
        // 落库失败就把本地状态退回去，别让界面和库里说的不一样
        if (settingsRef.current) settingsRef.current = { ...settingsRef.current, previewCollapsed: collapsed };
        setSettings((s) => (s ? { ...s, previewCollapsed: collapsed } : s));
      });
  };

  // 键盘处理器的依赖数组是空的（它每次从 store 现取状态），所以这里也得走 ref，
  // 否则 Ctrl+B 会一直调到挂载时那个 `collapsed = false` 的闭包。
  const togglePreviewRef = useRef(togglePreview);
  togglePreviewRef.current = togglePreview;

  /** 浏览器预览里拖握把改尺寸，拖完落库（真机由后端在隐藏时统一记忆）。 */
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

    // 后端每次入库都会广播，前端据此刷新
    const unItems = listen("plico://items-changed", () => {
      void refresh();
    });
    // 面板每次唤起都清空搜索词，回到干净状态
    const unShown = listen("plico://panel-shown", () => {
      reset();
    });
    // 设置变更（主题、Vim 模式等）在两个窗口之间同步
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

      // 输入法组合期间一切快捷键都让路，否则选词会被当成命令
      if (e.isComposing || state.composing) return;

      const target = e.target as HTMLElement | null;
      const inInput = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA";
      // 搜索框里已经有内容时，单键快捷键让位给输入 ——
      // 否则用户想打字母 "p" 会被当成置顶命令
      const typing = inInput && (target as HTMLInputElement).value.length > 0;
      const vimMode = settingsRef.current?.vimMode ?? false;

      // F19 编辑模式：编辑器开着的期间接管键盘，不走面板快捷键。
      // Enter 提交粘贴，Esc 退回列表。
      if (state.editingId !== null) {
        switch (e.key) {
          case "Enter":
            e.preventDefault();
            // 编辑模式只提交当前草稿；片段模式不会进入编辑模式。
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
            // F18：{{cursor}} 占位符由后端解析，粘贴后光标停在占位符处
            if (s) void state.pasteSnippet(s.content);
          } else {
            // Ctrl+Enter = 粘贴为纯文本（F15），普通 Enter 原样粘
            void state.pasteSelected(e.ctrlKey || e.metaKey);
          }
          break;

        case "Tab":
          e.preventDefault();
          state.cycleFilter(e.shiftKey ? -1 : 1);
          break;

        case "Escape":
          e.preventDefault();
          // 三段式：先把批量选择收缩回单项，再清搜索词，最后关面板。
          // 批量选择排在最先 —— 它是用户最近一次操作，也该最先被撤销。
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
          // Ctrl+A = 全选当前列表（F16 批量入口）。
          // 焦点在输入框里时让路：那里 Ctrl+A 应该是「选中搜索词」。
          if (!e.ctrlKey && !e.metaKey) return;
          if (inInput) return;
          e.preventDefault();
          state.selectAllVisible();
          break;

        case "b":
        case "B":
          // Ctrl+B = 折叠 / 展开预览区（F5）。不带修饰键时让路给输入。
          if (!e.ctrlKey && !e.metaKey) return;
          if (inInput) return;
          e.preventDefault();
          togglePreviewRef.current();
          break;

        case "e":
        case "E":
          // Ctrl+E = 编辑选中项（F19）。纯 E 不触发 —— 那会把输入挡掉。
          if (!e.ctrlKey && !e.metaKey) return;
          e.preventDefault();
          state.beginEditSelected();
          break;

        case "Delete":
          if (typing) return;
          e.preventDefault();
          // 走 deleteSelection 而不是 removeSelected：它按 selectedIds 走，
          // 单条选中时两者等价，批量选中时删掉一整组。
          void state.deleteSelection();
          break;

        case "p":
        case "P":
          // Ctrl+P = 上移（Emacs 传统，F5）。必须放在置顶判断**之前** ——
          // 否则会被下面那条「带修饰键就让路」的规则吃掉，变成静默空操作。
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
          // Ctrl+N = 下移（F5）。不带修饰键时什么都不做，避免吞掉正常输入。
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
        {/* 分隔线上挂着折叠开关：折叠动作就发生在这条线上，按钮放这里最直观。
            折叠后分隔线仍然留着，否则就没有「展开」的入口了。 */}
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
        {/* F19：编辑模式时预览区换成编辑器 */}
        {showPreview && (editingId !== null ? <EditPane /> : <PreviewPane />)}
      </div>
      <StatusBar />
      <ResizeGrip previewExpanded={showPreview} onResized={rememberSize} />
    </div>
  );
}
