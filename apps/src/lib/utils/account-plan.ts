import type { Account } from "@/types";

const KNOWN_PLAN_TAGS = new Set([
  "free",
  "go",
  "plus",
  "pro",
  "team",
  "business",
  "enterprise",
  "edu",
  "education",
  "k12",
  "k-12",
]);

function normalizePlanValue(value: unknown): string {
  const normalized = String(value || "").trim().toLowerCase();
  return normalized === "k-12" ? "k12" : normalized;
}

export function resolveAccountPlanKey(
  account: Pick<Account, "planType" | "planTypeRaw" | "tags">,
): string {
  const normalized = normalizePlanValue(account.planType);
  if (normalized && normalized !== "unknown") {
    return normalized;
  }

  const raw = normalizePlanValue(account.planTypeRaw);
  if (raw && raw !== "unknown") {
    return raw;
  }

  const tags = account.tags.map(normalizePlanValue).filter(Boolean);
  const knownTag = tags.find((tag) => KNOWN_PLAN_TAGS.has(tag));
  if (knownTag) {
    return knownTag === "education" ? "edu" : knownTag;
  }
  if (tags.length === 1) {
    return tags[0];
  }

  return "unknown";
}
