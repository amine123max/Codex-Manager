import { invoke, withAddr } from "@/lib/api/transport";
import type {
  BillingRule,
  BillingRuleUpsertParams,
  ModelPriceRule,
  ModelPriceRuleUpsertParams,
  QuotaApiKeyModelUsageItem,
  QuotaApiKeyUsageItem,
  QuotaCapacityConfigResult,
  QuotaModelPoolItem,
  QuotaModelPoolsResult,
  QuotaModelUsageItem,
  QuotaOverviewResult,
  QuotaPoolSourceBreakdown,
  QuotaRefreshSourceResult,
  QuotaRefreshSourcesParams,
  QuotaSourceSummary,
  QuotaSystemPoolResult,
} from "@/types/quota";

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function asBoolean(value: unknown): boolean {
  return typeof value === "boolean" ? value : Boolean(value);
}

function toNullableNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function readItems(payload: unknown): unknown[] {
  const source = asRecord(payload);
  return asArray(source.items ?? payload);
}

function normalizeModelUsageItem(payload: unknown): QuotaModelUsageItem {
  const source = asRecord(payload);
  return {
    model: asString(source.model) || "unknown",
    provider: asString(source.provider) || null,
    inputTokens: Math.max(0, toNullableNumber(source.inputTokens ?? source.input_tokens) ?? 0),
    cachedInputTokens: Math.max(
      0,
      toNullableNumber(source.cachedInputTokens ?? source.cached_input_tokens) ?? 0,
    ),
    outputTokens: Math.max(0, toNullableNumber(source.outputTokens ?? source.output_tokens) ?? 0),
    reasoningOutputTokens: Math.max(
      0,
      toNullableNumber(source.reasoningOutputTokens ?? source.reasoning_output_tokens) ?? 0,
    ),
    totalTokens: Math.max(0, toNullableNumber(source.totalTokens ?? source.total_tokens) ?? 0),
    estimatedCostUsd: toNullableNumber(source.estimatedCostUsd ?? source.estimated_cost_usd),
    priceStatus: asString(source.priceStatus ?? source.price_status) || "missing",
    apiKeyRemainingTokens: toNullableNumber(
      source.apiKeyRemainingTokens ?? source.api_key_remaining_tokens,
    ),
    aggregateEstimatedRemainingTokens: toNullableNumber(
      source.aggregateEstimatedRemainingTokens ??
        source.aggregate_estimated_remaining_tokens,
    ),
    aggregateBalanceUsd: toNullableNumber(
      source.aggregateBalanceUsd ?? source.aggregate_balance_usd,
    ),
    openaiAvailableAccountCount: Math.max(
      0,
      toNullableNumber(
        source.openaiAvailableAccountCount ?? source.openai_available_account_count,
      ) ?? 0,
    ),
    openaiPrimaryRemainPercent: toNullableNumber(
      source.openaiPrimaryRemainPercent ?? source.openai_primary_remain_percent,
    ),
    openaiSecondaryRemainPercent: toNullableNumber(
      source.openaiSecondaryRemainPercent ?? source.openai_secondary_remain_percent,
    ),
    openaiEstimatedRemainingTokens: toNullableNumber(
      source.openaiEstimatedRemainingTokens ??
        source.openai_estimated_remaining_tokens,
    ),
    openaiEstimateEnabled: asBoolean(
      source.openaiEstimateEnabled ?? source.openai_estimate_enabled,
    ),
  };
}

function normalizeApiKeyModelUsageItem(payload: unknown): QuotaApiKeyModelUsageItem {
  const source = asRecord(payload);
  return {
    model: asString(source.model) || "unknown",
    inputTokens: Math.max(0, toNullableNumber(source.inputTokens ?? source.input_tokens) ?? 0),
    cachedInputTokens: Math.max(
      0,
      toNullableNumber(source.cachedInputTokens ?? source.cached_input_tokens) ?? 0,
    ),
    outputTokens: Math.max(0, toNullableNumber(source.outputTokens ?? source.output_tokens) ?? 0),
    reasoningOutputTokens: Math.max(
      0,
      toNullableNumber(source.reasoningOutputTokens ?? source.reasoning_output_tokens) ?? 0,
    ),
    totalTokens: Math.max(0, toNullableNumber(source.totalTokens ?? source.total_tokens) ?? 0),
    estimatedCostUsd: toNullableNumber(source.estimatedCostUsd ?? source.estimated_cost_usd),
    priceStatus: asString(source.priceStatus ?? source.price_status) || "missing",
  };
}

function normalizeApiKeyUsageItem(payload: unknown): QuotaApiKeyUsageItem {
  const source = asRecord(payload);
  return {
    keyId: asString(source.keyId ?? source.key_id),
    name: asString(source.name) || null,
    modelSlug: asString(source.modelSlug ?? source.model_slug) || null,
    quotaLimitTokens: toNullableNumber(source.quotaLimitTokens ?? source.quota_limit_tokens),
    usedTokens: Math.max(0, toNullableNumber(source.usedTokens ?? source.used_tokens) ?? 0),
    remainingTokens: toNullableNumber(source.remainingTokens ?? source.remaining_tokens),
    estimatedCostUsd: Math.max(
      0,
      toNullableNumber(source.estimatedCostUsd ?? source.estimated_cost_usd) ?? 0,
    ),
    models: asArray(source.models).map(normalizeApiKeyModelUsageItem),
  };
}

function normalizeSourceSummary(payload: unknown): QuotaSourceSummary {
  const source = asRecord(payload);
  return {
    id: asString(source.id),
    kind: asString(source.kind),
    name: asString(source.name),
    status: asString(source.status) || "unknown",
    metricKind: asString(source.metricKind ?? source.metric_kind),
    remaining: toNullableNumber(source.remaining),
    total: toNullableNumber(source.total),
    used: toNullableNumber(source.used),
    unit: asString(source.unit) || null,
    models: asArray(source.models).map(asString).filter(Boolean),
    provider: asString(source.provider) || null,
    capturedAt: toNullableNumber(source.capturedAt ?? source.captured_at),
    error: asString(source.error) || null,
  };
}

function normalizeBillingRule(payload: unknown): BillingRule {
  const source = asRecord(payload);
  return {
    id: asString(source.id),
    name: asString(source.name),
    status: asString(source.status) || "active",
    priority: toNullableNumber(source.priority) ?? 0,
    multiplierMillis: toNullableNumber(
      source.multiplierMillis ?? source.multiplier_millis,
    ) ?? 1000,
    modelPattern: asString(source.modelPattern ?? source.model_pattern) || null,
    serviceTier: asString(source.serviceTier ?? source.service_tier) || null,
    userId: asString(source.userId ?? source.user_id) || null,
    projectId: asString(source.projectId ?? source.project_id) || null,
    apiKeyId: asString(source.apiKeyId ?? source.api_key_id) || null,
    startsAt: toNullableNumber(source.startsAt ?? source.starts_at),
    endsAt: toNullableNumber(source.endsAt ?? source.ends_at),
    createdAt: toNullableNumber(source.createdAt ?? source.created_at) ?? 0,
    updatedAt: toNullableNumber(source.updatedAt ?? source.updated_at) ?? 0,
  };
}

function normalizeModelPriceRule(payload: unknown): ModelPriceRule {
  const source = asRecord(payload);
  return {
    id: asString(source.id),
    provider: asString(source.provider) || "custom",
    modelPattern: asString(source.modelPattern ?? source.model_pattern),
    matchType: asString(source.matchType ?? source.match_type) || "prefix",
    inputPricePer1m: toNullableNumber(source.inputPricePer1m ?? source.input_price_per_1m),
    cachedInputPricePer1m: toNullableNumber(source.cachedInputPricePer1m ?? source.cached_input_price_per_1m),
    outputPricePer1m: toNullableNumber(source.outputPricePer1m ?? source.output_price_per_1m),
    reasoningOutputPricePer1m: toNullableNumber(source.reasoningOutputPricePer1m ?? source.reasoning_output_price_per_1m),
    source: asString(source.source),
    enabled: asBoolean(source.enabled),
    priority: Math.trunc(toNullableNumber(source.priority) ?? 0),
  };
}

function normalizeRefreshSource(payload: unknown): QuotaRefreshSourceResult {
  const source = asRecord(payload);
  return {
    id: asString(source.id),
    kind: asString(source.kind),
    ok: asBoolean(source.ok),
    error: asString(source.error) || null,
  };
}

function normalizePoolSource(payload: unknown): QuotaPoolSourceBreakdown {
  const source = asRecord(payload);
  return {
    sourceKind: asString(source.sourceKind ?? source.source_kind),
    sourceId: asString(source.sourceId ?? source.source_id),
    name: asString(source.name),
    status: asString(source.status) || "unknown",
    remainingTokens: toNullableNumber(source.remainingTokens ?? source.remaining_tokens),
    rawRemaining: toNullableNumber(source.rawRemaining ?? source.raw_remaining),
    rawUnit: asString(source.rawUnit ?? source.raw_unit) || null,
    models: asArray(source.models).map(asString).filter(Boolean),
    capturedAt: toNullableNumber(source.capturedAt ?? source.captured_at),
    priceStatus: asString(source.priceStatus ?? source.price_status) || "missing",
  };
}

function normalizeModelPoolItem(payload: unknown): QuotaModelPoolItem {
  const source = asRecord(payload);
  return {
    model: asString(source.model) || "unknown",
    provider: asString(source.provider) || null,
    totalRemainingTokens: toNullableNumber(
      source.totalRemainingTokens ?? source.total_remaining_tokens,
    ),
    aggregateRemainingTokens: toNullableNumber(
      source.aggregateRemainingTokens ?? source.aggregate_remaining_tokens,
    ),
    accountPrimaryRemainingTokens: toNullableNumber(
      source.accountPrimaryRemainingTokens ?? source.account_primary_remaining_tokens,
    ),
    accountSecondaryRemainingTokens: toNullableNumber(
      source.accountSecondaryRemainingTokens ?? source.account_secondary_remaining_tokens,
    ),
    accountEstimatedRemainingTokens: toNullableNumber(
      source.accountEstimatedRemainingTokens ?? source.account_estimated_remaining_tokens,
    ),
    sourceCount: Math.max(0, toNullableNumber(source.sourceCount ?? source.source_count) ?? 0),
    sources: asArray(source.sources).map(normalizePoolSource),
    priceStatus: asString(source.priceStatus ?? source.price_status) || "missing",
  };
}

function normalizeCapacityConfig(payload: unknown): QuotaCapacityConfigResult {
  const source = asRecord(payload);
  return {
    sourceAssignments: asArray(source.sourceAssignments ?? source.source_assignments).map((item) => {
      const record = asRecord(item);
      return {
        sourceKind: asString(record.sourceKind ?? record.source_kind),
        sourceId: asString(record.sourceId ?? record.source_id),
        modelSlugs: asArray(record.modelSlugs ?? record.model_slugs).map(asString).filter(Boolean),
      };
    }),
    templates: asArray(source.templates).map((item) => {
      const record = asRecord(item);
      return {
        planType: asString(record.planType ?? record.plan_type),
        primaryWindowTokens: toNullableNumber(
          record.primaryWindowTokens ?? record.primary_window_tokens,
        ),
        secondaryWindowTokens: toNullableNumber(
          record.secondaryWindowTokens ?? record.secondary_window_tokens,
        ),
      };
    }),
    accountOverrides: asArray(source.accountOverrides ?? source.account_overrides).map((item) => {
      const record = asRecord(item);
      return {
        accountId: asString(record.accountId ?? record.account_id),
        primaryWindowTokens: toNullableNumber(
          record.primaryWindowTokens ?? record.primary_window_tokens,
        ),
        secondaryWindowTokens: toNullableNumber(
          record.secondaryWindowTokens ?? record.secondary_window_tokens,
        ),
      };
    }),
  };
}

function normalizeModelPools(payload: unknown): QuotaModelPoolsResult {
  const source = asRecord(payload);
  return {
    items: readItems(payload).map(normalizeModelPoolItem),
    templates: normalizeCapacityConfig(source).templates,
    accountOverrides: normalizeCapacityConfig(source).accountOverrides,
  };
}

function normalizeSystemPool(payload: unknown): QuotaSystemPoolResult {
  const source = asRecord(payload);
  return {
    referenceModel: asString(source.referenceModel ?? source.reference_model) || "unknown",
    provider: asString(source.provider) || null,
    totalRemainingTokens: toNullableNumber(
      source.totalRemainingTokens ?? source.total_remaining_tokens,
    ),
    aggregateRemainingTokens: toNullableNumber(
      source.aggregateRemainingTokens ?? source.aggregate_remaining_tokens,
    ),
    accountPrimaryRemainingTokens: toNullableNumber(
      source.accountPrimaryRemainingTokens ?? source.account_primary_remaining_tokens,
    ),
    accountSecondaryRemainingTokens: toNullableNumber(
      source.accountSecondaryRemainingTokens ?? source.account_secondary_remaining_tokens,
    ),
    accountEstimatedRemainingTokens: toNullableNumber(
      source.accountEstimatedRemainingTokens ?? source.account_estimated_remaining_tokens,
    ),
    aggregateSourceCount: Math.max(
      0,
      toNullableNumber(source.aggregateSourceCount ?? source.aggregate_source_count) ?? 0,
    ),
    accountSourceCount: Math.max(
      0,
      toNullableNumber(source.accountSourceCount ?? source.account_source_count) ?? 0,
    ),
    unknownSourceCount: Math.max(
      0,
      toNullableNumber(source.unknownSourceCount ?? source.unknown_source_count) ?? 0,
    ),
    priceStatus: asString(source.priceStatus ?? source.price_status) || "missing",
    sources: asArray(source.sources).map(normalizePoolSource),
  };
}

function normalizeOverview(payload: unknown): QuotaOverviewResult {
  const source = asRecord(payload);
  const apiKey = asRecord(source.apiKey ?? source.api_key);
  const aggregateApi = asRecord(source.aggregateApi ?? source.aggregate_api);
  const openaiAccount = asRecord(source.openaiAccount ?? source.openai_account);
  const todayUsage = asRecord(source.todayUsage ?? source.today_usage);
  return {
    apiKey: {
      keyCount: Math.max(0, toNullableNumber(apiKey.keyCount ?? apiKey.key_count) ?? 0),
      limitedKeyCount: Math.max(
        0,
        toNullableNumber(apiKey.limitedKeyCount ?? apiKey.limited_key_count) ?? 0,
      ),
      totalLimitTokens: toNullableNumber(
        apiKey.totalLimitTokens ?? apiKey.total_limit_tokens,
      ),
      totalUsedTokens: Math.max(
        0,
        toNullableNumber(apiKey.totalUsedTokens ?? apiKey.total_used_tokens) ?? 0,
      ),
      totalRemainingTokens: toNullableNumber(
        apiKey.totalRemainingTokens ?? apiKey.total_remaining_tokens,
      ),
      estimatedCostUsd: Math.max(
        0,
        toNullableNumber(apiKey.estimatedCostUsd ?? apiKey.estimated_cost_usd) ?? 0,
      ),
    },
    aggregateApi: {
      sourceCount: Math.max(
        0,
        toNullableNumber(aggregateApi.sourceCount ?? aggregateApi.source_count) ?? 0,
      ),
      enabledBalanceQueryCount: Math.max(
        0,
        toNullableNumber(
          aggregateApi.enabledBalanceQueryCount ??
            aggregateApi.enabled_balance_query_count,
        ) ?? 0,
      ),
      okCount: Math.max(0, toNullableNumber(aggregateApi.okCount ?? aggregateApi.ok_count) ?? 0),
      errorCount: Math.max(
        0,
        toNullableNumber(aggregateApi.errorCount ?? aggregateApi.error_count) ?? 0,
      ),
      totalBalanceUsd: toNullableNumber(
        aggregateApi.totalBalanceUsd ?? aggregateApi.total_balance_usd,
      ),
      lastRefreshedAt: toNullableNumber(
        aggregateApi.lastRefreshedAt ?? aggregateApi.last_refreshed_at,
      ),
    },
    openaiAccount: {
      accountCount: Math.max(
        0,
        toNullableNumber(openaiAccount.accountCount ?? openaiAccount.account_count) ?? 0,
      ),
      availableCount: Math.max(
        0,
        toNullableNumber(openaiAccount.availableCount ?? openaiAccount.available_count) ?? 0,
      ),
      lowQuotaCount: Math.max(
        0,
        toNullableNumber(openaiAccount.lowQuotaCount ?? openaiAccount.low_quota_count) ?? 0,
      ),
      primaryRemainPercent: toNullableNumber(
        openaiAccount.primaryRemainPercent ?? openaiAccount.primary_remain_percent,
      ),
      secondaryRemainPercent: toNullableNumber(
        openaiAccount.secondaryRemainPercent ?? openaiAccount.secondary_remain_percent,
      ),
      lastRefreshedAt: toNullableNumber(
        openaiAccount.lastRefreshedAt ?? openaiAccount.last_refreshed_at,
      ),
    },
    todayUsage: {
      inputTokens: Math.max(
        0,
        toNullableNumber(todayUsage.inputTokens ?? todayUsage.input_tokens) ?? 0,
      ),
      cachedInputTokens: Math.max(
        0,
        toNullableNumber(todayUsage.cachedInputTokens ?? todayUsage.cached_input_tokens) ?? 0,
      ),
      outputTokens: Math.max(
        0,
        toNullableNumber(todayUsage.outputTokens ?? todayUsage.output_tokens) ?? 0,
      ),
      reasoningOutputTokens: Math.max(
        0,
        toNullableNumber(
          todayUsage.reasoningOutputTokens ?? todayUsage.reasoning_output_tokens,
        ) ?? 0,
      ),
      totalTokens: Math.max(
        0,
        toNullableNumber(todayUsage.totalTokens ?? todayUsage.total_tokens) ?? 0,
      ),
      estimatedCostUsd: Math.max(
        0,
        toNullableNumber(todayUsage.estimatedCostUsd ?? todayUsage.estimated_cost_usd) ?? 0,
      ),
    },
  };
}

export const quotaClient = {
  async overview(): Promise<QuotaOverviewResult> {
    return normalizeOverview(await invoke<unknown>("service_quota_overview", withAddr()));
  },
  async modelUsage(params?: {
    startTs?: number | null;
    endTs?: number | null;
  }): Promise<QuotaModelUsageItem[]> {
    const result = await invoke<unknown>(
      "service_quota_model_usage",
      withAddr({
        startTs: params?.startTs ?? null,
        endTs: params?.endTs ?? null,
      }),
    );
    return readItems(result).map(normalizeModelUsageItem);
  },
  async apiKeyUsage(): Promise<QuotaApiKeyUsageItem[]> {
    const result = await invoke<unknown>("service_quota_api_key_usage", withAddr());
    return readItems(result).map(normalizeApiKeyUsageItem);
  },
  async sourceList(): Promise<QuotaSourceSummary[]> {
    const result = await invoke<unknown>("service_quota_source_list", withAddr());
    return readItems(result).map(normalizeSourceSummary);
  },
  async modelPools(): Promise<QuotaModelPoolsResult> {
    return normalizeModelPools(await invoke<unknown>("service_quota_model_pools", withAddr()));
  },
  async systemPool(params?: {
    referenceModel?: string | null;
  }): Promise<QuotaSystemPoolResult> {
    return normalizeSystemPool(
      await invoke<unknown>(
        "service_quota_system_pool",
        withAddr({
          referenceModel: params?.referenceModel ?? null,
        }),
      ),
    );
  },
  async capacityConfig(): Promise<QuotaCapacityConfigResult> {
    return normalizeCapacityConfig(
      await invoke<unknown>("service_quota_capacity_config", withAddr()),
    );
  },
  async billingRules(): Promise<BillingRule[]> {
    const result = await invoke<unknown>("service_quota_billing_rules", withAddr());
    return readItems(result).map(normalizeBillingRule);
  },
  async modelPriceRules(): Promise<ModelPriceRule[]> {
    const result = await invoke<unknown>("service_quota_model_price_rules", withAddr());
    return readItems(result).map(normalizeModelPriceRule);
  },
  async upsertModelPriceRule(params: ModelPriceRuleUpsertParams): Promise<ModelPriceRule[]> {
    const result = await invoke<unknown>(
      "service_quota_model_price_rule_upsert",
      withAddr({
        id: params.id ?? null,
        provider: params.provider,
        modelPattern: params.modelPattern,
        matchType: params.matchType,
        inputPricePer1m: params.inputPricePer1m,
        cachedInputPricePer1m: params.cachedInputPricePer1m ?? null,
        outputPricePer1m: params.outputPricePer1m,
        reasoningOutputPricePer1m: params.reasoningOutputPricePer1m ?? null,
        enabled: params.enabled ?? true,
        priority: params.priority ?? 20_000,
      }),
    );
    return readItems(result).map(normalizeModelPriceRule);
  },
  async deleteModelPriceRule(id: string): Promise<ModelPriceRule[]> {
    const result = await invoke<unknown>(
      "service_quota_model_price_rule_delete",
      withAddr({ id }),
    );
    return readItems(result).map(normalizeModelPriceRule);
  },
  async upsertBillingRule(params: BillingRuleUpsertParams): Promise<BillingRule[]> {
    const result = await invoke<unknown>(
      "service_quota_billing_rule_upsert",
      withAddr({
        id: params.id ?? null,
        name: params.name,
        status: params.status ?? "active",
        priority: params.priority ?? 0,
        multiplierMillis: params.multiplierMillis,
        modelPattern: params.modelPattern ?? null,
        serviceTier: params.serviceTier ?? null,
        userId: params.userId ?? null,
        apiKeyId: params.apiKeyId ?? null,
        startsAt: params.startsAt ?? null,
        endsAt: params.endsAt ?? null,
      }),
    );
    return readItems(result).map(normalizeBillingRule);
  },
  async deleteBillingRule(id: string): Promise<BillingRule[]> {
    const result = await invoke<unknown>(
      "service_quota_billing_rule_delete",
      withAddr({ id }),
    );
    return readItems(result).map(normalizeBillingRule);
  },
  async setSourceModels(params: {
    sourceKind: string;
    sourceId: string;
    modelSlugs: string[];
  }): Promise<QuotaCapacityConfigResult> {
    return normalizeCapacityConfig(
      await invoke<unknown>(
        "service_quota_source_models_set",
        withAddr({
          sourceKind: params.sourceKind,
          sourceId: params.sourceId,
          modelSlugs: params.modelSlugs,
        }),
      ),
    );
  },
  async updateCapacityTemplate(params: {
    planType: string;
    primaryWindowTokens?: number | null;
    secondaryWindowTokens?: number | null;
  }): Promise<QuotaCapacityConfigResult> {
    return normalizeCapacityConfig(
      await invoke<unknown>(
        "service_quota_capacity_template_update",
        withAddr({
          planType: params.planType,
          primaryWindowTokens: params.primaryWindowTokens ?? null,
          secondaryWindowTokens: params.secondaryWindowTokens ?? null,
        }),
      ),
    );
  },
  async updateAccountCapacityOverride(params: {
    accountId: string;
    primaryWindowTokens?: number | null;
    secondaryWindowTokens?: number | null;
  }): Promise<QuotaCapacityConfigResult> {
    return normalizeCapacityConfig(
      await invoke<unknown>(
        "service_quota_account_capacity_override_update",
        withAddr({
          accountId: params.accountId,
          primaryWindowTokens: params.primaryWindowTokens ?? null,
          secondaryWindowTokens: params.secondaryWindowTokens ?? null,
        }),
      ),
    );
  },
  async refreshSources(
    params: QuotaRefreshSourcesParams = {},
  ): Promise<QuotaRefreshSourceResult[]> {
    const result = await invoke<unknown>(
      "service_quota_refresh_sources",
      withAddr({
        kinds: params.kinds ?? [],
        sourceIds: params.sourceIds ?? [],
        source_ids: params.sourceIds ?? [],
      }),
    );
    return readItems(result).map(normalizeRefreshSource);
  },
};
