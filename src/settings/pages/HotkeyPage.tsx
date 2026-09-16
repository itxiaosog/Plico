import { useEffect, useState } from "react";

import { Button, Row, Section } from "../controls";
import { t } from "../../lib/i18n";
import { useSettingsStore } from "../useSettingsStore";
import { DEFAULT_HOTKEY, DEFAULT_PLAIN_HOTKEY, buildHotkey, formatHotkey, modifierCount } from "../hotkey";

type Which = "panel" | "plain";

export function HotkeyPage() {
  const settings = useSettingsStore((s) => s.settings);
  const patch = useSettingsStore((s) => s.patch);

  const [recording, setRecording] = useState<Which | null>(null);
  const [hint, setHint] = useState<string | null>(null);

  useEffect(() => {
    if (!recording) return;

    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setRecording(null);
        setHint(null);
        return;
      }
      if (e.repeat) return;

      const combo = buildHotkey(e);
      if (!combo) {
        setHint(t("hotkeyBadKey"));
        return;
      }
      if (modifierCount(combo) === 0) {
        setHint(t("hotkeyNeedModifier"));
        return;
      }

      const which = recording;
      setRecording(null);
      setHint(null);
      void patch(which === "panel" ? { hotkey: combo } : { plainHotkey: combo }).then((ok) => {
        if (!ok) setHint(t("hotkeyTaken"));
      });
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, patch]);

  if (!settings) return null;

  const renderSlot = (
    which: Which,
    value: string,
    fallback: string,
  ) => {
    const isRecording = recording === which;
    return (
      <div className="pl-set-hotkey">
        <kbd className={`pl-set-kbd${isRecording ? " is-recording" : ""}`}>
          {isRecording ? t("hotkeyRecording") : formatHotkey(value)}
        </kbd>
        <Button
          variant={isRecording ? "danger" : "default"}
          onClick={() => {
            setHint(null);
            setRecording(isRecording ? null : which);
          }}
        >
          {isRecording ? t("hotkeyCancel") : t("hotkeyRecord")}
        </Button>
        {value !== fallback && (
          <Button
            onClick={() => {
              setHint(null);
              setRecording(null);
              void patch(which === "panel" ? { hotkey: fallback } : { plainHotkey: fallback });
            }}
          >
            {t("hotkeyReset")}
          </Button>
        )}
      </div>
    );
  };

  return (
    <>
      <Section title={t("hotkeySection")}>
        <Row label={t("hotkeyLabel")} hint={t("hotkeyHint")}>
          {renderSlot("panel", settings.hotkey, DEFAULT_HOTKEY)}
        </Row>
        {hint && <div className="pl-set-warn">{hint}</div>}
      </Section>

      <Section title={t("plainHotkeySection")}>
        <Row label={t("plainHotkeyLabel")} hint={t("plainHotkeyHint")}>
          {renderSlot("plain", settings.plainHotkey, DEFAULT_PLAIN_HOTKEY)}
        </Row>
      </Section>
    </>
  );
}
