export interface ResetCreditExpiry {
  expiresAt: string | null;
  expiresAtMs: number | null;
}

/** Read reset-card metadata, independently of the 5h/7d quota windows. */
export function getResetCreditExpirations(creditsJson: string | null | undefined): ResetCreditExpiry[] {
  if (!creditsJson) return [];
  try {
    const payload = JSON.parse(creditsJson);
    const reset = payload?.rate_limit_reset_credits ?? payload?.rateLimitResetCredits ?? payload;
    if (!Array.isArray(reset?.credits)) return [];
    return reset.credits
      .filter((credit: unknown) => {
        if (!credit || typeof credit !== "object" || Array.isArray(credit)) return false;
        const item = credit as Record<string, unknown>;
        const status = String(item.status ?? "").trim().toLowerCase();
        const resetType = String(item.reset_type ?? item.resetType ?? "").trim().toLowerCase();
        return (!status || status === "available") && (!resetType || resetType === "codex_rate_limits");
      })
      .map((credit: Record<string, unknown>) => {
        const raw = credit.expires_at ?? credit.expiresAt;
        const expiresAt = typeof raw === "string" && raw.trim() ? raw.trim() : null;
        const time = expiresAt ? Date.parse(expiresAt) : NaN;
        return { expiresAt, expiresAtMs: Number.isFinite(time) ? time : null };
      })
      .sort((a: ResetCreditExpiry, b: ResetCreditExpiry) =>
        (a.expiresAtMs ?? Infinity) - (b.expiresAtMs ?? Infinity));
  } catch {
    return [];
  }
}

export function formatResetCreditExpiry(credit: ResetCreditExpiry, locale: string, unknownText: string): string {
  if (credit.expiresAtMs == null) return credit.expiresAt ?? unknownText;
  return new Intl.DateTimeFormat(locale, {
    year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false,
  }).format(new Date(credit.expiresAtMs));
}
