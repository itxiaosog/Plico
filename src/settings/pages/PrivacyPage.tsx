import { useState } from "react";

import { PixelInput } from "@pxlkit/ui-kit";
import { Button, Section, Switch } from "../controls";
import { t } from "../../lib/i18n";
import { useSettingsStore } from "../useSettingsStore";
import type { Rule, RuleKind } from "../../types";

export function PrivacyPage() {
  const rules = useSettingsStore((s) => s.rules);
  const addRule = useSettingsStore((s) => s.addRule);
  const deleteRule = useSettingsStore((s) => s.deleteRule);
  const setRuleEnabled = useSettingsStore((s) => s.setRuleEnabled);

  return (
    <>
      <Section title={t("privacyAppTitle")}>
        <div className="pl-set-note">
          <p>{t("privacyAppNote")}</p>
        </div>
        <RuleList
          kind="app"
          rules={rules}
          placeholder={t("privacyAppPlaceholder")}
          addLabel={t("privacyAppAdd")}
          empty={t("privacyAppEmpty")}
          onAdd={addRule}
          onDelete={deleteRule}
          onToggle={setRuleEnabled}
        />
      </Section>

      <Section title={t("privacyContentTitle")}>
        <div className="pl-set-note">
          <p>{t("privacyContentNote")}</p>
        </div>
        <RuleList
          kind="content"
          rules={rules}
          placeholder={t("privacyContentPlaceholder")}
          addLabel={t("privacyContentAdd")}
          empty={t("privacyContentEmpty")}
          onAdd={addRule}
          onDelete={deleteRule}
          onToggle={setRuleEnabled}
        />
      </Section>

      <Section title={t("privacyBoundsTitle")}>
        <div className="pl-set-note">
          <p>{t("privacyBoundsNote")}</p>
        </div>
      </Section>
    </>
  );
}

function RuleList({
  kind,
  rules,
  placeholder,
  addLabel,
  empty,
  onAdd,
  onDelete,
  onToggle,
}: {
  kind: RuleKind;
  rules: Rule[];
  placeholder: string;
  addLabel: string;
  empty: string;
  onAdd(kind: RuleKind, value: string): Promise<boolean>;
  onDelete(id: number): Promise<boolean>;
  onToggle(id: number, enabled: boolean): Promise<boolean>;
}) {
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);

  const mine = rules.filter((r) => r.kind === kind);

  const submit = () => {
    const value = draft.trim();
    if (!value || busy) return;
    setBusy(true);
    void onAdd(kind, value).then((ok) => {
      setBusy(false);
      // 失败时保留输入，用户改一改正则就能重试
      if (ok) setDraft("");
    });
  };

  return (
    <div className="pl-set-rules">
      {mine.length === 0 ? (
        <div className="pl-set-rules__empty">{empty}</div>
      ) : (
        mine.map((rule) => (
          <div className="pl-set-rule" key={rule.id}>
            <span
              className={`pl-set-rule__value${kind === "content" ? " is-mono" : ""}${
                rule.enabled ? "" : " is-off"
              }`}
            >
              {rule.value}
            </span>
            <Switch
              checked={rule.enabled}
              onChange={(enabled) => void onToggle(rule.id, enabled)}
            />
            <button
              type="button"
              className="pl-set-rule__del"
              aria-label={`${t("itemDelete")} ${rule.value}`}
              onClick={() => void onDelete(rule.id)}
            >
              ×
            </button>
          </div>
        ))
      )}

      <div className="pl-set-rule is-new">
        <PixelInput
          size="sm"
          value={draft}
          placeholder={placeholder}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              submit();
            }
          }}
        />
        <Button onClick={submit} disabled={busy || draft.trim().length === 0}>
          {addLabel}
        </Button>
      </div>
    </div>
  );
}
