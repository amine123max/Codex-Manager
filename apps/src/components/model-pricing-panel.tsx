"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { DollarSign, PencilLine, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { quotaClient } from "@/lib/api/quota-client";
import { getAppErrorMessage } from "@/lib/api/transport";
import { useI18n } from "@/lib/i18n/provider";
import type { ModelPriceRule } from "@/types/quota";

const emptyDraft = {
  id: "", provider: "openai", modelPattern: "", matchType: "prefix",
  input: "", cached: "", output: "", reasoning: "", enabled: true,
};

export function ModelPricingPanel({ enabled }: { enabled: boolean }) {
  const { t } = useI18n();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState(emptyDraft);
  const query = useQuery({
    queryKey: ["quota", "model-price-rules"],
    queryFn: () => quotaClient.modelPriceRules(),
    enabled,
  });
  const save = useMutation({
    mutationFn: () => quotaClient.upsertModelPriceRule({
      id: draft.id || null,
      provider: draft.provider,
      modelPattern: draft.modelPattern,
      matchType: draft.matchType,
      inputPricePer1m: Number(draft.input),
      cachedInputPricePer1m: draft.cached ? Number(draft.cached) : null,
      outputPricePer1m: Number(draft.output),
      reasoningOutputPricePer1m: draft.reasoning ? Number(draft.reasoning) : null,
      enabled: draft.enabled,
    }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["quota"] });
      setOpen(false);
      toast.success(t("模型价格已保存"));
    },
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });
  const remove = useMutation({
    mutationFn: (id: string) => quotaClient.deleteModelPriceRule(id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["quota"] }),
    onError: (error) => toast.error(getAppErrorMessage(error)),
  });

  const edit = (item?: ModelPriceRule) => {
    setDraft(item ? {
      id: item.id, provider: item.provider, modelPattern: item.modelPattern,
      matchType: item.matchType, input: String(item.inputPricePer1m ?? ""),
      cached: String(item.cachedInputPricePer1m ?? ""), output: String(item.outputPricePer1m ?? ""),
      reasoning: String(item.reasoningOutputPricePer1m ?? ""), enabled: item.enabled,
    } : emptyDraft);
    setOpen(true);
  };
  const customRules = (query.data || []).filter((item) => item.source === "custom");

  return (
    <Card className="glass-card shadow-sm">
      <CardHeader className="flex flex-row items-center justify-between">
        <div>
          <CardTitle className="flex items-center gap-2 text-base"><DollarSign className="h-4 w-4" />{t("模型计费价格")}</CardTitle>
          <p className="mt-1 text-xs text-muted-foreground">{t("按模型分别设置每百万 Token 的输入、缓存、输出和推理输出价格。")}</p>
        </div>
        <Button size="sm" onClick={() => edit()}><Plus className="mr-1.5 h-4 w-4" />{t("新增价格")}</Button>
      </CardHeader>
      <CardContent>
        {customRules.length ? (
          <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
            {customRules.map((item) => (
              <div key={item.id} className="flex items-center gap-3 rounded-md border p-3">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold">{item.modelPattern}</div>
                  <div className="mt-1 text-[11px] text-muted-foreground tabular-nums">
                    IN ${item.inputPricePer1m ?? "--"} · CACHE ${item.cachedInputPricePer1m ?? "--"} · OUT ${item.outputPricePer1m ?? "--"}
                  </div>
                </div>
                <Button size="icon" variant="ghost" onClick={() => edit(item)}><PencilLine className="h-4 w-4" /></Button>
                <Button size="icon" variant="ghost" className="text-red-500" onClick={() => remove.mutate(item.id)}><Trash2 className="h-4 w-4" /></Button>
              </div>
            ))}
          </div>
        ) : <p className="text-sm text-muted-foreground">{t("当前使用内置官价，可新增自定义模型价格覆盖。")}</p>}
      </CardContent>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-xl">
          <DialogHeader><DialogTitle>{draft.id ? t("编辑模型价格") : t("新增模型价格")}</DialogTitle></DialogHeader>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5 sm:col-span-2"><Label>{t("模型匹配")}</Label><Input value={draft.modelPattern} onChange={(e) => setDraft({...draft, modelPattern:e.target.value})} placeholder="gpt-5.6-sol" /></div>
            <div className="space-y-1.5"><Label>{t("提供商")}</Label><Input value={draft.provider} onChange={(e) => setDraft({...draft, provider:e.target.value})} /></div>
            <div className="space-y-1.5"><Label>{t("匹配方式")}</Label><select className="h-9 w-full rounded-md border bg-background px-3 text-sm" value={draft.matchType} onChange={(e) => setDraft({...draft, matchType:e.target.value})}><option value="exact">exact</option><option value="prefix">prefix</option><option value="wildcard">wildcard</option></select></div>
            {[['input',t("输入 $/MTok")],['cached',t("缓存输入 $/MTok")],['output',t("输出 $/MTok")],['reasoning',t("推理输出 $/MTok")]] .map(([key,label]) => <div className="space-y-1.5" key={key}><Label>{label}</Label><Input type="number" min="0" step="any" value={draft[key as keyof typeof draft] as string} onChange={(e) => setDraft({...draft,[key]:e.target.value})} /></div>)}
            <label className="flex items-center gap-2 text-sm"><Checkbox checked={draft.enabled} onCheckedChange={(value) => setDraft({...draft,enabled:Boolean(value)})} />{t("启用")}</label>
          </div>
          <DialogFooter><Button onClick={() => save.mutate()} disabled={save.isPending || !draft.modelPattern.trim() || !draft.input || !draft.output}>{t("保存")}</Button></DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
