import { useState } from "react";

import { t } from "../lib/i18n";
import { groupDotColor } from "../lib/palette";
import type { Group } from "../types";

/**
 * F16 拖拽归类条。
 *
 * 只在拖拽期间出现（sticky 在列表顶部）：分组平时由搜索栏的下拉菜单切换，
 * 常驻一条分组栏会把 760×480 的面板再切掉一块，而拖拽是「手已经在鼠标上」
 * 的动作 —— 这时候给一排投放目标最自然，也不需要改动任何常驻布局。
 *
 * 「未分组」是第一个投放目标：它同时承担「从分组里拿出来」的语义，
 * 否则条目进了分组之后就只能靠右键菜单才能移出来。
 */
export function GroupDropBar({
  groups,
  count,
  onAssign,
}: {
  groups: Group[];
  /** 正在拖的条目数，显示在提示文案里。 */
  count: number;
  /** `groupId` 为 null 表示移出分组。 */
  onAssign(groupId: number | null): void;
}) {
  /** 当前悬停的投放目标，用来给用户一个「松手会落到这里」的反馈。 */
  const [hover, setHover] = useState<number | "none" | null>(null);

  const chip = (key: number | "none", groupId: number | null, label: string, color: string | null) => (
    <button
      type="button"
      key={String(key)}
      className={`pl-drop-chip${hover === key ? " is-over" : ""}`}
      // dragover 里必须 preventDefault，否则浏览器认为这里不接受投放，
      // drop 事件根本不会触发（这是最容易漏的一步）
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
