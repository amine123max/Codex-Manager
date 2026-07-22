"use client";

import {
  Database,
  KeyRound,
  ShieldAlert,
  RefreshCw,
} from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button, buttonVariants } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";
import {
  formatAccountSubscriptionPlanLabel,
  formatAccountSubscriptionStatusLabel,
} from "@/app/accounts/accounts-page-helpers";
import {
  formatTsFromSeconds,
} from "@/lib/utils/usage";
import { Account } from "@/types";
import { useI18n } from "@/lib/i18n/provider";

interface UsageModalProps {
  account: Account | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRefresh: (id: string) => void;
  onRefreshRt: (id: string) => void;
  isRefreshing: boolean;
  isRefreshingRt: boolean;
}

export default function UsageModal({
  account,
  open,
  onOpenChange,
  onRefresh,
  onRefreshRt,
  isRefreshing,
  isRefreshingRt,
}: UsageModalProps) {
  const { t } = useI18n();
  if (!account) return null;
  const subscriptionStatusLabel = formatAccountSubscriptionStatusLabel(account, t);
  const subscriptionPlanLabel = formatAccountSubscriptionPlanLabel(account, t);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="glass-card grid-rows-[auto_minmax(0,1fr)_auto] p-6"
        style={{
          maxHeight: "calc(100vh - 2rem)",
          maxWidth: "none",
          width: "min(700px, calc(100vw - 2rem))",
        }}
      >
        <DialogHeader>
          <div className="mb-2 flex items-center gap-3">
            <div className="rounded-full bg-primary/10 p-2 text-primary">
              <Database className="h-5 w-5" />
            </div>
            <DialogTitle>{t("用量详情")}</DialogTitle>
          </div>
          <DialogDescription className="font-medium text-foreground/80">
            {t("账号:")} {account.name} ({account.id.slice(0, 8)}...)
          </DialogDescription>
        </DialogHeader>

        <div className="grid min-h-0 gap-4 overflow-y-auto py-4 pr-1">
          {!account.hasToken ? (
            <Alert variant="destructive">
              <ShieldAlert className="h-4 w-4" />
              <AlertTitle>{t("缺少授权 Token")}</AlertTitle>
              <AlertDescription>
                {t("该账号只有用量快照，当前不能参与模型刷新或网关转发。请重新登录或刷新 AT/RT 后再使用。")}
              </AlertDescription>
            </Alert>
          ) : null}

          <Card size="sm">
            <CardHeader>
              <CardTitle>{t("累计使用")}</CardTitle>
              <CardDescription>{t("独立累计统计，清空请求日志不会重置金额。")}</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="rounded-md border p-3">
                  <div className="text-[10px] text-muted-foreground">{t("请求次数")}</div>
                  <div className="text-sm font-semibold tabular-nums">{account.usageRequestCount}</div>
                </div>
                <div className="rounded-md border p-3">
                  <div className="text-[10px] text-muted-foreground">{t("累计 Token")}</div>
                  <div className="text-sm font-semibold tabular-nums">{account.usageTotalTokens.toLocaleString()}</div>
                </div>
                <div className="rounded-md border p-3">
                  <div className="text-[10px] text-muted-foreground">{t("累计金额")}</div>
                  <div className="text-sm font-semibold text-emerald-600 tabular-nums">${account.usageEstimatedCostUsd.toFixed(4)}</div>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card size="sm">
            <CardHeader>
              <CardTitle>{t("套餐信息")}</CardTitle>
              <CardDescription>
                {t("这里展示账号套餐接口同步回来的套餐状态与时间信息。")}
              </CardDescription>
            </CardHeader>

            <CardContent>
              <div className="grid gap-3 sm:grid-cols-2">
                <Card size="sm">
                  <CardContent>
                    <div className="text-[10px] text-muted-foreground">{t("订阅状态")}</div>
                    <div className="text-sm font-semibold">{subscriptionStatusLabel}</div>
                  </CardContent>
                </Card>
                <Card size="sm">
                  <CardContent>
                    <div className="text-[10px] text-muted-foreground">{t("订阅方案")}</div>
                    <div className="text-sm font-semibold">{subscriptionPlanLabel}</div>
                  </CardContent>
                </Card>
                <Card size="sm">
                  <CardContent>
                    <div className="text-[10px] text-muted-foreground">{t("到期时间")}</div>
                    <div className="text-sm font-semibold">
                      {formatTsFromSeconds(account.subscriptionExpiresAt, t("未知"))}
                    </div>
                  </CardContent>
                </Card>
                <Card size="sm">
                  <CardContent>
                    <div className="text-[10px] text-muted-foreground">{t("续费时间")}</div>
                    <div className="text-sm font-semibold">
                      {formatTsFromSeconds(account.subscriptionRenewsAt, t("未知"))}
                    </div>
                  </CardContent>
                </Card>
              </div>
            </CardContent>
          </Card>

          <div className="text-center">
            <p className="text-[10px] italic text-muted-foreground">
              {t("数据捕获于:")} {formatTsFromSeconds(account.lastRefreshAt, t("未知时间"))}
            </p>
          </div>
        </div>

        <DialogFooter className="-mx-6 -mb-6 min-h-16 px-10 py-3 sm:items-center sm:justify-between">
          <DialogClose
            className={buttonVariants({ variant: "ghost" })}
            type="button"
          >
            {t("关闭")}
          </DialogClose>
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button
              variant="outline"
              onClick={() => onRefreshRt(account.id)}
              disabled={isRefreshingRt}
              className="gap-2"
            >
              <KeyRound
                className={cn("h-4 w-4", isRefreshingRt && "animate-pulse")}
              />
              {isRefreshingRt ? t("AT/RT 刷新中...") : t("刷新 AT/RT")}
            </Button>
            <Button
              onClick={() => onRefresh(account.id)}
              disabled={isRefreshing}
              className="gap-2"
            >
              <RefreshCw
                className={cn("h-4 w-4", isRefreshing && "animate-spin")}
              />
              {isRefreshing ? t("正在刷新...") : t("立即刷新")}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
