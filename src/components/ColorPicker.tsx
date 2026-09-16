import { t } from "../lib/i18n";
import { GROUP_PALETTE, isColorActive } from "../lib/palette";

/**
 * F16 分组色板选择器。
 *
 * 8 个色点一行摆开，不做下拉 —— 8 个是「一眼能扫完」的量级，
 * 再套一层弹层反而多一次点击。规范第 7 节要求色标只做 8px 圆点、
 * 不做大面积填充，所以这里选中态用一圈描边表示，而不是把底色铺满。
 */
export function ColorPicker({
  value,
  disabled,
  onPick,
}: {
  value: string | null;
  disabled?: boolean;
  onPick(color: string): void;
}) {
  return (
    <div className="pl-color-row" role="radiogroup" aria-label={t("groupColor")}>
      {GROUP_PALETTE.map((entry) => {
        const active = isColorActive(value, entry.value);
        return (
          <button
            key={entry.value}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={t(entry.labelKey)}
            title={t(entry.labelKey)}
            disabled={disabled}
            className={`pl-color-dot${active ? " is-active" : ""}`}
            style={{ background: entry.swatch }}
            onClick={() => onPick(entry.value)}
          />
        );
      })}
    </div>
  );
}
