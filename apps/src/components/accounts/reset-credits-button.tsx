"use client";

import { Clock, History } from "lucide-react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useI18n } from "@/lib/i18n/provider";
import { cn } from "@/lib/utils";
import { formatResetCreditExpiry, getResetCreditExpirations } from "@/lib/utils/reset-credits";
import { formatTsFromSeconds } from "@/lib/utils/usage";
import type { Account } from "@/types";

export function ResetCreditsButton({ account, disabled, loading, onQuery }: {
  account: Account;
  disabled: boolean;
  loading: boolean;
  onQuery: () => void;
}) {
  const { t, locale } = useI18n();
  const count = account.quotaResetAvailableCount;
  const expirations = count === 0 ? [] : getResetCreditExpirations(account.usage?.creditsJson);

  return (
    <Tooltip>
      <TooltipTrigger render={<span tabIndex={0} />} className="inline-flex">
        <button type="button"
          className="inline-flex items-center gap-1 text-primary hover:underline disabled:opacity-50"
          disabled={disabled || loading} onClick={onQuery}
          aria-label={t("查询剩余的额度重置次数")}>
          <History className={cn("h-3 w-3", loading && "animate-pulse")} />
          {t("次数")} {count ?? "--"}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right" sideOffset={10}
        className="max-w-[360px] rounded-lg border bg-popover p-3 text-popover-foreground shadow-md">
        <div className="min-w-[240px] space-y-2">
          <p className="text-sm font-semibold">{t("重置次数详情")}</p>
          <p className="text-xs">{t("剩余重置次数：{count}", { count: count ?? "--" })}</p>
          {expirations.length > 0 ? (
            <div className="max-h-64 overflow-y-auto space-y-2">
              {expirations.map((credit, index) => (
                <div key={`${credit.expiresAt}-${index}`} className="flex items-start gap-2 text-xs">
                  <Clock className="mt-0.5 h-3 w-3 shrink-0 text-primary" />
                  <div>
                    <p className="text-muted-foreground">{t("第 {index} 次重置 · 过期时间", { index: index + 1 })}</p>
                    <p className="tabular-nums">{formatResetCreditExpiry(credit, locale, t("未知"))}</p>
                    {credit.expiresAtMs != null && credit.expiresAtMs <= Date.now() ? (
                      <p className="text-amber-600">{t("已过期，请点击次数刷新")}</p>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-xs text-muted-foreground">{count === 0 ? t("暂无可用重置次数") : t("暂无过期时间，请点击次数查询")}</p>
          )}
          {count != null && count > expirations.length && expirations.length > 0 ? (
            <p className="text-xs text-muted-foreground">{t("部分重置次数未提供过期时间")}</p>
          ) : null}
          <p className="border-t pt-2 text-[10px] text-muted-foreground">
            {t("最近更新")}: {formatTsFromSeconds(account.usage?.capturedAt ?? null, t("未知"))}
          </p>
          <p className="text-[10px] text-muted-foreground">{t("点击次数可刷新，查询不会消耗重置次数。")}</p>
        </div>
      </TooltipContent>
    </Tooltip>
  );
}
