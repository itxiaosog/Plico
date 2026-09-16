import { useEffect, useState } from "react";

import { getAppInfo } from "../../api";
import { t } from "../../lib/i18n";
import { Row, Section } from "../controls";
import type { AppInfo } from "../../types";

export function AboutPage() {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    void getAppInfo().then(setInfo).catch(() => setInfo(null));
  }, []);

  return (
    <>
      <Section title="Plico">
        <Row label={t("aboutVersion")} hint={t("aboutTagline")}>
          <span className="pl-set-value">{info ? info.version : t("loading")}</span>
        </Row>
        <div className="pl-set-note">
          <p>{t("aboutPrivacy")}</p>
        </div>
      </Section>
    </>
  );
}
