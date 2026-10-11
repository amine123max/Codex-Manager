"use client";

import { useState } from "react";
import { ChevronDown, Clock, Loader2, Zap } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { appClient } from "@/lib/api/app-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { useI18n } from "@/lib/i18n/provider";
import { useAppStore } from "@/lib/store/useAppStore";
import { formatTsFromSeconds } from "@/lib/utils/usage";
import type { Account } from "@/types";

export function WarmupControls({ accounts, selectedIds, ready, warming, onImmediate }: {
  accounts: Account[];
  selectedIds: string[];
  ready: boolean;
  warming: boolean;
  onImmediate: () => void;
}) {
  const { t } = useI18n();
  const background = useAppStore(state => state.appSettings.backgroundTasks);
  const setAppSettings = useAppStore(state => state.setAppSettings);
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);
  const [hours, setHours] = useState("1");
  const [repeat, setRepeat] = useState(false);
  const [targets, setTargets] = useState<string[]>([]);

  async function openSchedule() {
    setOpen(true);
    setLoading(true);
    try {
      const latest = await appClient.getSettings();
      setAppSettings(latest);
      const schedule = latest.backgroundTasks;
      setHours(String(schedule.warmupIntervalSecs > 0 ? schedule.warmupIntervalSecs / 3600 : 1));
      setRepeat(schedule.warmupRepeat);
      setTargets((selectedIds.length ? selectedIds : schedule.warmupCronAccountIds)
        .filter(id => accounts.some(account => account.id === id)));
    } catch (error) {
      toast.error(`${t("读取预热计划失败")}: ${getAppErrorMessage(error)}`);
      setOpen(false);
    } finally {
      setLoading(false);
    }
  }

  async function saveSchedule(cancel = false) {
    const delayHours = Number(hours);
    if (!cancel && (!Number.isFinite(delayHours) || delayHours < 0.01 || delayHours > 8760)) {
      toast.error(t("请输入 0.01 到 8760 之间的小时数"));
      return;
    }
    if (!cancel && targets.length === 0) {
      toast.error(t("请至少选择一个预热账号"));
      return;
    }
    setSaving(true);
    try {
      const result = await appClient.setSettings({ backgroundTasks: {
        warmupCronEnabled: !cancel,
        ...(cancel ? { warmupIntervalSecs: 0, warmupRepeat: false, warmupNextRunAt: 0, warmupCronAccountIds: [] } : {
          warmupIntervalSecs: Math.round(delayHours * 3600),
          warmupRepeat: repeat,
          warmupNextRunAt: 0,
          warmupCronAccountIds: targets,
        }),
      } });
      setAppSettings(result);
      toast.success(cancel ? t("定时预热已取消") : t("定时预热已保存"));
      setOpen(false);
    } catch (error) {
      toast.error(`${t("保存预热计划失败")}: ${getAppErrorMessage(error)}`);
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <div className="inline-flex items-center">
        <Tooltip>
          <TooltipTrigger render={<span />} className="inline-flex">
            <Button variant="outline"
              className="glass-card h-10 min-w-[88px] gap-2 rounded-l-xl rounded-r-none px-3"
              disabled={!ready || warming || accounts.length === 0} onClick={onImmediate}>
              {warming ? <Loader2 className="h-4 w-4 animate-spin" /> : <Zap className="h-4 w-4" />}
              <span className="text-sm font-medium">{warming ? t("预热中...") : t("预热")}</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("向选中账号发送 hi 进行预热；如果未选中账号，则默认预热全部账号。")}</TooltipContent>
        </Tooltip>
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button variant="outline" />}
            aria-label={t("预热选项")} disabled={!ready}
            className="glass-card h-10 rounded-l-none rounded-r-xl border-l-0 px-2">
            <ChevronDown className="h-4 w-4" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48 rounded-xl border border-border/70 bg-popover/95 p-2 shadow-sm">
            <DropdownMenuItem disabled={warming || accounts.length === 0} onClick={onImmediate}>
              <Zap className="mr-2 h-4 w-4" />{t("即时预热")}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => void openSchedule()}>
              <Clock className="mr-2 h-4 w-4" />{t("定时预热")}
              {background.warmupCronEnabled ? <span className="ml-auto h-2 w-2 rounded-full bg-emerald-500" /> : null}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <Dialog open={open} onOpenChange={value => { if (!saving) setOpen(value); }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t("定时预热")}</DialogTitle>
            <DialogDescription>{t("设置多少小时后对选定账号预热，可选择仅执行一次或按相同间隔重复。")}</DialogDescription>
          </DialogHeader>
          {loading ? <div className="flex justify-center py-8"><Loader2 className="h-5 w-5 animate-spin" /></div> : (
            <div className="space-y-4">
              {background.warmupCronEnabled ? (
                <div className="rounded-lg bg-primary/10 p-3 text-xs">
                  {t("当前计划已启用")}
                  {background.warmupIntervalSecs > 0 ? <p className="mt-1">{t("下次预热")}: {formatTsFromSeconds(background.warmupNextRunAt, t("未知"))}</p> : null}
                </div>
              ) : null}
              <div className="space-y-2">
                <Label htmlFor="warmup-delay-hours">{t("多少小时后预热")}</Label>
                <Input id="warmup-delay-hours" type="number" min="0.01" max="8760" step="0.01"
                  value={hours} disabled={saving} onChange={event => setHours(event.target.value)} />
                <p className="text-xs text-muted-foreground">{t("支持小数，例如 0.5 小时为 30 分钟；保存后开始计时。")}</p>
              </div>
              <div className="flex items-center justify-between gap-3">
                <Label htmlFor="warmup-repeat">{t("重复执行")}</Label>
                <Switch id="warmup-repeat" checked={repeat} disabled={saving} onCheckedChange={setRepeat} />
              </div>
              <p className="text-xs text-muted-foreground">{repeat ? t("每隔 {hours} 小时预热一次", { hours }) : t("只执行一次，完成后自动关闭计划。")}</p>
              <div className="space-y-2">
                <Label>{t("预热账号（已选 {count} 个）", { count: targets.length })}</Label>
                <div className="max-h-44 space-y-1 overflow-y-auto rounded-lg border p-2">
                  {accounts.map(account => (
                    <label key={account.id} className="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-xs hover:bg-muted">
                      <Checkbox checked={targets.includes(account.id)} disabled={saving}
                        onCheckedChange={checked => setTargets(current => checked ? [...current, account.id] : current.filter(id => id !== account.id))} />
                      <span className="min-w-0 truncate">{account.name}</span>
                    </label>
                  ))}
                </div>
              </div>
              <p className="text-xs text-muted-foreground">{t("定时预热由服务执行，服务需保持运行；重启后保留计划，错过的执行会在服务启动后补一次。不可用账号会跳过。")}</p>
            </div>
          )}
          <DialogFooter>
            {background.warmupCronEnabled || background.warmupIntervalSecs > 0 ? <Button variant="outline" disabled={saving || loading} onClick={() => void saveSchedule(true)}>{t("取消定时预热")}</Button> : null}
            <Button variant="outline" disabled={saving} onClick={() => setOpen(false)}>{t("关闭")}</Button>
            <Button disabled={saving || loading || !ready} onClick={() => void saveSchedule()}>
              {saving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}{t("保存计划")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
