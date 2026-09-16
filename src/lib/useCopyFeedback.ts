import { useEffect, useState } from "react";
import { copyText } from "../api";
import { t } from "./i18n";

export function useCopyFeedback() {
  const [status, setStatus] = useState<"idle" | "pending" | "success" | "error">("idle");
  useEffect(() => { if (status === "idle") return; const id = window.setTimeout(() => setStatus("idle"), 2400); return () => window.clearTimeout(id); }, [status]);
  async function run(makeText: () => string): Promise<void> {
    setStatus("pending");
    try { await copyText(makeText()); setStatus("success"); } catch { setStatus("error"); }
  }
  return { status, pending: status === "pending", message: status === "success" ? t("copySuccess") : status === "error" ? t("copyError") : "", run };
}
