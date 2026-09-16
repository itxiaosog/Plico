import { useEffect, useState } from "react";

import { clearHistory, getStats } from "../../api";
import { t } from "../../lib/i18n";
import { Button, NumberField, Row, Section } from "../controls";
import { useSettingsStore } from "../useSettingsStore";
import type { Stats } from "../../types";

/**
 * 存储页。
 *
 * 这里只有「容量」和「清理」两组数字 —— 数据目录固定在安装目录下的 `data/`，
 * 不提供修改入口，所以没有「数据」那一栏（要看路径去关于页）。
 */
export function StoragePage() {
  const settings = useSettingsStore((s) => s.settings);
  const patch = useSettingsStore((s) => s.patch);

  const [stats, setStats] = useState<Stats | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  const refreshStats = () => {
    void getStats().then(setStats).catch(() => setStats(null));
  };

  useEffect(() => {
    refreshStats();
  }, []);

  if (!settings) return null;

  return (
    <>
      <Section title={t("storageSection")}>
        <Row
          label={t("maxItemsLabel")}
          hint={t("maxItemsHint")}
        >
          <NumberField
            value={settings.maxItems}
            min={100}
            max={10_000}
            unit={t("unitItems")}
            onCommit={(maxItems) => void patch({ maxItems })}
          />
        </Row>

        <Row label={t("retentionLabel")} hint={t("retentionHint")}>
          <NumberField
            value={settings.retentionDays}
            min={1}
            max={365}
            unit={t("unitDays")}
            onCommit={(retentionDays) => void patch({ retentionDays })}
          />
        </Row>

        <Row label={t("imageQuotaLabel")} hint={t("imageQuotaHint")}>
          <NumberField
            value={settings.imageQuotaMb}
            min={50}
            max={10_240}
            unit="MB"
            onCommit={(imageQuotaMb) => void patch({ imageQuotaMb })}
          />
        </Row>
      </Section>

      <Section title={t("cleanupSection")}>
        <Row
          label={t("historyLabel")}
          hint={
            stats
              ? `${stats.total} ${t("unitItems")}${stats.pinned > 0 ? ` · ${stats.pinned} ${t("statusPin")}` : ""}`
              : t("loading")
          }
        >
          <Button onClick={refreshStats}>{t("historyRefresh")}</Button>
        </Row>

        <Row label={t("clearLabel")} hint={t("clearHint")}>
          {confirming ? (
            <div className="pl-set-hotkey">
              <Button
                variant="danger"
                onClick={() => {
                  setConfirming(false);
                  void clearHistory(true).then((n) => {
                    setNote(`${n} ${t("unitItems")} ${t("clearDone")}`);
                    refreshStats();
                  });
                }}
              >
                {t("clearConfirm")}
              </Button>
              <Button onClick={() => setConfirming(false)}>{t("clearCancel")}</Button>
            </div>
          ) : (
            <Button
              onClick={() => {
                setNote(null);
                setConfirming(true);
              }}
            >
              {t("clearLabel")}…
            </Button>
          )}
        </Row>
        {note && <div className="pl-set-warn">{note}</div>}
      </Section>
    </>
  );
}
