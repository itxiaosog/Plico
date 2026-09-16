import { useState, useEffect } from "react";
import { openSettings, togglePanelPin, isPanelPinned } from "../api";
import { t } from "../lib/i18n";
import { PinIcon, TuneIcon } from "./TypeIcon";
import { useClipboardStore } from "../store/useClipboardStore";

export function StatusBar() {
  const total = useClipboardStore((s) => s.stats.total);
  const pinned = useClipboardStore((s) => s.stats.pinned);
  const error = useClipboardStore((s) => s.error);
  const editing = useClipboardStore((s) => s.editingId !== null);
  // 批量选择时 Delete 的语义变成「删一组」，提示文案要跟着变，
  // 否则用户以为只会删光标那一条。
  const multi = useClipboardStore((s) => s.selectedIds.length > 1);

  // 面板「钉住桌面」状态：会话级，不持久化。
  // 启动时读一次初始值（正常是 false），之后只靠点击切换。
  const [panelPinned, setPanelPinned] = useState(false);
  useEffect(() => {
    void isPanelPinned().then(setPanelPinned).catch(() => {});
  }, []);
  const togglePin = async () => {
    try {
      setPanelPinned(await togglePanelPin());
    } catch {
      // mock 环境不会有，真机出问题最多是钉不住，不报错
    }
  };

  if (error) {
    return (
      <div className="pl-status">
        <span style={{ color: "var(--pl-danger)" }}>{error}</span>
      </div>
    );
  }

  // F19 编辑态下的状态栏换成编辑器的快捷键提示
  if (editing) {
    return (
      <div className="pl-status">
        <span>Enter {t("statusPaste")}</span>
        <span>Ctrl+Enter {t("pastePlainText")}</span>
        <span>Esc {t("statusClose")}</span>
        <span className="pl-status__spacer" />
      </div>
    );
  }

  return (
    <div className="pl-status">
      <span>↑↓ {t("statusSelect")}</span>
      <span>Enter {t("statusPaste")}</span>
      <span>P {t("statusPin")}</span>
      <span>Delete {multi ? t("multiDelete") : t("statusDelete")}</span>
      <span>Tab {t("statusFilter")}</span>
      <span>Esc {t("statusClose")}</span>
      <span className="pl-status__spacer" />
      <span>
        {total} {t("unitItems")}
        {pinned > 0 ? ` · ${pinned} ${t("statusPin")}` : ""}
      </span>
      {/* 钉住桌面：点击切换。钉住时按钮高亮（强调色），表示当前处于「常驻」态。 */}
      <button
        type="button"
        className={`pl-icon-btn pl-panel-pin${panelPinned ? " is-pinned" : ""}`}
        title={panelPinned ? t("panelUnpin") : t("panelPin")}
        aria-label={panelPinned ? t("panelUnpin") : t("panelPin")}
        onClick={() => void togglePin()}
      >
        <PinIcon size={13} />
      </button>
      <button
        type="button"
        className="pl-icon-btn"
        title={t("statusSettings")}
        aria-label={t("statusSettings")}
        onClick={() => void openSettings()}
      >
        <TuneIcon size={13} />
      </button>
    </div>
  );
}
