import { Row, Section, Segmented, Switch } from "../controls";
import { t } from "../../lib/i18n";
import { useSettingsStore } from "../useSettingsStore";

export function GeneralPage() {
  const settings = useSettingsStore((s) => s.settings);
  const patch = useSettingsStore((s) => s.patch);
  if (!settings) return null;

  return (
    <>
      <Section title={t("sectionLanguage")}>
        <Row label={t("languageLabel")} hint={t("languageHint")}>
          <Segmented
            value={settings.language}
            options={[
              { value: "system", label: t("langSystem") },
              { value: "zh", label: t("langZh") },
              { value: "en", label: t("langEn") },
            ]}
            onChange={(language) => void patch({ language })}
          />
        </Row>
      </Section>

      <Section title={t("sectionStartup")}>
        <Row label={t("autostartLabel")} hint={t("autostartHint")}>
          <Switch
            checked={settings.autostart}
            onChange={(autostart) => void patch({ autostart })}
          />
        </Row>
      </Section>
    </>
  );
}
