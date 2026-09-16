import { t } from "../lib/i18n";
import { GROUP_PALETTE, isColorActive } from "../lib/palette";

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
