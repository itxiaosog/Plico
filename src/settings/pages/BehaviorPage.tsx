import { NumberField, Row, Section, Segmented, Switch } from "../controls";
import { t } from "../../lib/i18n";
import { useSettingsStore } from "../useSettingsStore";
import type { PanelPosition } from "../../types";

export function BehaviorPage() {
  const settings = useSettingsStore((s) => s.settings);
  const patch = useSettingsStore((s) => s.patch);
  if (!settings) return null;

  return (
    <>
      <Section title={t("behaviorSection")}>
        <Row label={t("positionLabel")} hint={t("positionHint")}>
          <Segmented<PanelPosition>
            value={settings.panelPosition}
            options={[
              { value: "cursor", label: t("positionCursor") },
              { value: "remember", label: t("positionRemember") },
            ]}
            onChange={(panelPosition) => void patch({ panelPosition })}
          />
        </Row>

        <Row label={t("blurHideLabel")} hint={t("blurHideHint")}>
          <Switch
            checked={settings.hideOnBlur}
            onChange={(hideOnBlur) => void patch({ hideOnBlur })}
          />
        </Row>

        <Row label={t("hideDelayLabel")} hint={t("hideDelayHint")}>
          <NumberField
            value={settings.hideDelayMs}
            min={0}
            max={5_000}
            unit={t("unitMs")}
            disabled={!settings.hideOnBlur}
            onCommit={(hideDelayMs) => void patch({ hideDelayMs })}
          />
        </Row>
      </Section>

      <Section title={t("keyboardSection")}>
        <Row label={t("vimLabel")} hint={t("vimHint")}>
          <Switch
            checked={settings.vimMode}
            onChange={(vimMode) => void patch({ vimMode })}
          />
        </Row>
        <div className="pl-set-note">
          <p>
            {t("behaviorKeys1")}
          </p>
          <p>{t("behaviorKeys2")}</p>
        </div>
      </Section>

      <Section title={t("pasteSection")}>
        <Row
          label={t("restoreLabel")}
          hint={t("restoreHint")}
        >
          <Switch
            checked={settings.restoreClipboard}
            onChange={(restoreClipboard) => void patch({ restoreClipboard })}
          />
        </Row>
      </Section>
    </>
  );
}
