"use client";

import { useI18n } from "@/lib/i18n/provider";
import { cn } from "@/lib/utils";
import { durationSeverity, firstResponseSeverity, formatOutputTokensPerSecond, formatRequestLatency } from "@/lib/utils/request-latency";
import type { RequestLog } from "@/types";

const PALETTE = {
  good: { text: "text-emerald-600 dark:text-emerald-400", bar: "bg-emerald-500", from: "from-emerald-500", to: "to-emerald-500" },
  warn: { text: "text-amber-600 dark:text-amber-400", bar: "bg-amber-400", from: "from-amber-400", to: "to-amber-400" },
  slow: { text: "text-orange-600 dark:text-orange-400", bar: "bg-orange-500", from: "from-orange-500", to: "to-orange-500" },
  critical: { text: "text-red-600 dark:text-red-400", bar: "bg-red-500", from: "from-red-500", to: "to-red-500" },
  unknown: { text: "text-muted-foreground", bar: "bg-muted-foreground/30", from: "from-muted-foreground/30", to: "to-muted-foreground/30" },
} as const;

export function RequestLatencyCell({ log }: { log: Pick<RequestLog, "durationMs" | "firstResponseMs" | "outputTokens" | "requestPath" | "originalPath"> }) {
  const { t } = useI18n();
  const first = firstResponseSeverity(log.firstResponseMs);
  const duration = durationSeverity(log.durationMs);
  // Image/audio endpoints may report modality token counts, not text generation speed.
  const path = log.originalPath || log.requestPath;
  const isTextRequest = !/\/(?:images|audio|videos)(?:\/|$)/i.test(path);
  const outputRate = isTextRequest ? formatOutputTokensPerSecond(log.outputTokens, log.durationMs) : "—";

  return (
    <div className="flex items-stretch gap-2 whitespace-nowrap" data-testid="request-latency">
      <span aria-hidden="true" className={cn("w-1 shrink-0 rounded-full", first === "unknown"
        ? PALETTE[duration].bar
        : ["bg-gradient-to-b from-40% to-60%", PALETTE[first].from, PALETTE[duration].to])} />
      <dl className="grid grid-cols-[max-content_max-content] items-baseline gap-x-2 gap-y-0.5 text-xs">
        <dt className="text-muted-foreground" title={t("首响表示从请求开始到首个上游响应片段的耗时")}>{t("首字")}</dt>
        <dd className={cn("m-0 font-medium tabular-nums", PALETTE[first].text)}>{formatRequestLatency(log.firstResponseMs)}</dd>
        <dt className="text-muted-foreground">{t("总耗时")}</dt>
        <dd className={cn("m-0 font-medium tabular-nums", PALETTE[duration].text)}>{formatRequestLatency(log.durationMs)}</dd>
        <dt className="text-muted-foreground" title={t("输出 TPS = 输出 Token 数 ÷ 总耗时（秒），包含已计入输出的推理 Token。")}>{t("输出 TPS")}</dt>
        <dd className="m-0 font-medium text-foreground tabular-nums" data-testid="output-tps">{outputRate}</dd>
      </dl>
    </div>
  );
}
