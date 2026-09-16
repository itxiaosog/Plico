import { useState } from "react";

import { t } from "../lib/i18n";
import { groupDotColor } from "../lib/palette";
import type { Group } from "../types";

export function GroupDropBar({
  groups,
  count,
  onAssign,
}: {
  groups: Group[];
  count: number;
  onAssign(groupId: number | null): void;
}) {
  const [hover, setHover] = useState<number | "none" | null>(null);

  const chip = (key: number | "none", groupId: number | null, label: string, color: string | null) => (
    <button
      type="button"
      key={String(key)}
      className={`pl-drop-chip${hover === key ? " is-over" : ""}`}
      onDragOver={(e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        setHover(key);
      }}
      onDragLeave={() => setHover((h) => (h === key ? null : h))}
      onDrop={(e) => {
        e.preventDefault();
        setHover(null);
        onAssign(groupId);
      }}
    >
      {color && <span className="pl-group-dot" style={{ background: groupDotColor(color) }} />}
      {label}
    </button>
  );

  return (
    <div className="pl-drop-bar" data-count={count}>
      <span className="pl-drop-bar__label">
        {count > 1 ? `${count} ${t("unitItems")} → ` : ""}
        {t("dropHint")}
      </span>
      {chip("none", null, t("groupNone"), null)}
      {groups.map((g) => chip(g.id, g.id, g.name, g.color))}
    </div>
  );
}
