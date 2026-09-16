import { useEffect, useState } from "react";

import { useClipboardStore } from "../store/useClipboardStore";
import { baseName, cssColor, formatListTime, parseFileList, summarize } from "../lib/format";
import { imageUrl } from "../lib/bridge";
import { t } from "../lib/i18n";
import { highlight } from "../lib/highlight";
import type { Item } from "../types";
import { CheckIcon, PinIcon, TrashIcon, TypeIcon } from "./TypeIcon";

interface Props {
  item: Item;
  selected: boolean;
  multiSelected?: boolean;
  query: string;
  onSelect: () => void;
  onToggleMulti?: () => void;
  onRangeSelect?: () => void;
  draggable?: boolean;
  onPin: () => void;
  onDelete: () => void;
  onPastePlain?: () => void;
  onCopyMarkdown?: () => void;
  onAddTag?: () => void;
  onAssignGroup?: () => void;
}

function Highlighted({ text, query }: { text: string; query: string }) {
  return (
    <>
      {highlight(text, query).map((seg, i) =>
        seg.hit ? <mark key={i}>{seg.text}</mark> : <span key={i}>{seg.text}</span>,
      )}
    </>
  );
}

const CHIP_LIMIT = 2;

function Thumb({ item }: { item: Item }) {
  const src = imageUrl(item.thumb_path ?? item.image_path);
  if (!src) {
    return <span className="pl-thumb" />;
  }
  return <img className="pl-thumb" src={src} alt="" />;
}

export function ItemRow({ item, selected, multiSelected = false, query, onSelect, onToggleMulti, onRangeSelect, draggable = false, onPin, onDelete, onPastePlain, onCopyMarkdown, onAddTag, onAssignGroup }: Props) {
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);

  const isImage = item.type === "image";
  const isFiles = item.type === "files";
  const isColor = item.type === "color";
  const isTextual = !isImage && !isFiles;
  const isTall = isImage || isFiles;

  const color = isColor ? cssColor(item.content ?? item.plain_text ?? "") : null;
  const files = isFiles ? parseFileList(item.content ?? item.plain_text) : [];
  const textSummary = summarize(item.plain_text ?? item.content);
  const itemTags = useClipboardStore((s) => s.itemTags[item.id] ?? []);

  useEffect(() => {
    if (itemTags.length === 0) {
      void useClipboardStore.getState().loadItemTags(item.id);
    }
  }, [item.id, itemTags.length]);

  return (
    <div
      className={`pl-row${isTall ? " is-tall" : ""}${selected ? " is-selected" : ""}${multiSelected ? " is-multi" : ""}`}
      data-id={item.id}
      draggable={draggable}
      onMouseDown={(e) => {
        e.preventDefault();
        if (e.shiftKey && onRangeSelect) {
          onRangeSelect();
          return;
        }
        if ((e.ctrlKey || e.metaKey) && onToggleMulti) {
          onToggleMulti();
          return;
        }
        onSelect();
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        setMenu({ x: e.clientX, y: e.clientY });
      }}
    >
      <span className={`pl-badge${multiSelected ? " is-checked" : ""}`} data-type={item.type}>
        {multiSelected ? <CheckIcon /> : <TypeIcon type={item.type} />}
      </span>

      {isImage && <Thumb item={item} />}

      {isFiles ? (
        <span className="pl-chips">
          {files.slice(0, CHIP_LIMIT).map((p, i) => (
            <span className="pl-chip" key={i}>
              {baseName(p)}
            </span>
          ))}
          {files.length > CHIP_LIMIT && (
            <span className="pl-chip pl-chip--more">+{files.length - CHIP_LIMIT}</span>
          )}
          {files.length === 0 && <span className="pl-summary">{t("itemEmptyFiles")}</span>}
        </span>
      ) : (
        <>
          {color && <span className="pl-swatch" style={{ background: color }} />}
          <span className="pl-summary">
            <Highlighted text={textSummary || "（空）"} query={query} />
          </span>
        </>
      )}

      {itemTags.length > 0 && (
        <span className="pl-row__tags" title={itemTags.map((tag) => `#${tag.name}`).join(" ")}>
          {itemTags.slice(0, 2).map((tag) => `#${tag.name}`).join(" ")}
        </span>
      )}

      <span className="pl-time">{formatListTime(item.last_copied_at)}</span>

      {item.pinned ? (
        <>

          <button
            type="button"
            className="pl-icon-btn pl-pin"
            title={t("itemUnpin")}
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onPin();
            }}
          >
            <PinIcon />
          </button>
          <span className="pl-actions">
            <button
              type="button"
              className="pl-icon-btn is-danger"
              title={t("itemDelete")}
              onMouseDown={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onDelete();
              }}
            >
              <TrashIcon />
            </button>
          </span>
        </>
      ) : (
        <span className="pl-actions">
          <button
            type="button"
            className="pl-icon-btn"
            title={t("itemPin")}
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onPin();
            }}
          >
            <PinIcon />
          </button>
          <button
            type="button"
            className="pl-icon-btn is-danger"
            title={t("itemDelete")}
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onDelete();
            }}
          >
            <TrashIcon />
          </button>
        </span>
      )}

      {menu && (
        <ItemContextMenu
          pos={menu}
          isTextual={isTextual}
          onClose={() => setMenu(null)}
          onPastePlain={onPastePlain}
          onCopyMarkdown={onCopyMarkdown}
          onAddTag={onAddTag}
          onAssignGroup={onAssignGroup}
        />
      )}
    </div>
  );
}

function ItemContextMenu({
  pos,
  isTextual,
  onClose,
  onPastePlain,
  onCopyMarkdown,
  onAddTag,
  onAssignGroup,
}: {
  pos: { x: number; y: number };
  isTextual: boolean;
  onClose(): void;
  onPastePlain?: () => void;
  onCopyMarkdown?: () => void;
  onAddTag?: () => void;
  onAssignGroup?: () => void;
}) {
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (!(e.target as HTMLElement).closest(".pl-context-menu")) onClose();
    };
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("mousedown", onDown, true);
    window.addEventListener("keydown", onEsc, true);
    return () => {
      document.removeEventListener("mousedown", onDown, true);
      window.removeEventListener("keydown", onEsc, true);
    };
  }, [onClose]);

  if (!isTextual && !onCopyMarkdown && !onAddTag && !onAssignGroup) return null;

  return (
    <div className="pl-context-menu" style={{ left: Math.min(pos.x, window.innerWidth - 200), top: Math.min(pos.y, window.innerHeight - 190) }} onMouseDown={(e) => e.stopPropagation()}>
      {onAssignGroup && <button type="button" className="pl-menu__item" onClick={() => { onClose(); onAssignGroup(); }}>{t("groupAssign")}</button>}
      {isTextual && (
        <button type="button" className="pl-menu__item" onClick={() => { onClose(); onPastePlain?.(); }}>
          {t("pastePlainText")}
        </button>
      )}
      {onCopyMarkdown && (
        <button
          type="button"
          className="pl-menu__item"
          onClick={() => {
            onClose();
            onCopyMarkdown();
          }}
        >
          {t("copyMarkdown")}
        </button>
      )}
      {onAddTag && (
        <button
          type="button"
          className="pl-menu__item"
          onClick={() => {
            onClose();
            onAddTag();
          }}
        >
          {t("addTag")}
        </button>
      )}
    </div>
  );
}
