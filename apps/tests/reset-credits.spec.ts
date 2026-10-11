import { expect, test } from "@playwright/test";

test.use({ timezoneId: "Asia/Shanghai" });

async function mockAccount(page: import("@playwright/test").Page, count: number | null, credits: unknown[]) {
  const methods: string[] = [];
  const patches: Record<string, unknown>[] = [];
  let settings = { locale: "zh-CN", theme: "tech", codexCliGuideDismissed: true, serviceAddr: "localhost:48760",
    backgroundTasks: { usagePollingEnabled: false, warmupCronEnabled: false, warmupCronExpression: "",
      warmupCronAccountIds: [] as string[], warmupIntervalSecs: 0, warmupRepeat: false, warmupNextRunAt: 0 } };
  let currentCredits = credits;
  const account = { id: "reset-demo", label: "reset-demo@example.com", status: "active", hasToken: true,
    planType: "plus", quotaResetAvailableCount: count };
  await page.route("**/api/runtime**", route => route.fulfill({ json: {
    mode: "web-gateway", rpcBaseUrl: "/api/rpc", canManageService: false,
    canUseBrowserFileImport: true, canUseBrowserDownloadExport: true,
  } }));
  await page.route("**/api/rpc", async route => {
    const { method, id, params } = route.request().postDataJSON();
    methods.push(method);
    let result: unknown = {};
    if (method === "appSettings/get") result = settings;
    if (method === "appSettings/set") {
      patches.push(params);
      settings = { ...settings, ...params, backgroundTasks: { ...settings.backgroundTasks, ...params.backgroundTasks } };
      if (settings.backgroundTasks.warmupCronEnabled && settings.backgroundTasks.warmupNextRunAt === 0) {
        settings.backgroundTasks.warmupNextRunAt = Math.floor(Date.now() / 1000) + settings.backgroundTasks.warmupIntervalSecs;
      }
      result = settings;
    }
    if (method === "initialize") result = { userAgent: "codex_cli_rs/test", codexHome: "C:/Test", platformFamily: "windows", platformOs: "windows" };
    if (method === "accountManager/session/current") result = { mode: "none", role: "system_admin", permissions: [], currentUser: null };
    if (method === "account/list") result = { items: [account], total: 1 };
    if (method === "account/usage/list") result = { items: [{ accountId: account.id, usedPercent: 5,
      windowMinutes: 300, resetsAt: 1800000000, secondaryUsedPercent: 20,
      secondaryWindowMinutes: 10080, secondaryResetsAt: 1800000000,
      capturedAt: 1791705600, creditsJson: JSON.stringify({ rate_limit_reset_credits: { available_count: count, credits: currentCredits } }),
    }] };
    if (method === "account/usage/resetCredits") {
      currentCredits = [{ expires_at: "2027-02-01T00:00:00Z" }];
      account.quotaResetAvailableCount = 1;
      result = { availableCount: 1 };
    }
    await route.fulfill({ json: { jsonrpc: "2.0", id, result } });
  });
  return { methods, patches };
}

test("hover shows each card's expiration and clicking only queries credits", async ({ page }) => {
  const { methods } = await mockAccount(page, 2, [
    { expires_at: "2027-01-01T00:00:00Z" }, { expires_at: "2026-12-01T00:00:00Z" },
  ]);
  await page.goto("/accounts/");
  const button = page.getByRole("button", { name: "查询剩余的额度重置次数", exact: true });
  await expect(button).toContainText("次数 2");
  await button.hover();
  const popup = page.locator('[data-slot="tooltip-content"]').filter({ hasText: "重置次数详情" });
  await expect(popup).toBeVisible();
  await expect(popup).toContainText("2026/12/01 08:00:00");
  await expect(popup).toContainText("2027/01/01 08:00:00");
  expect(methods).not.toContain("account/usage/resetCredits");
  await button.click();
  await expect(button).toContainText("次数 1");
  await page.mouse.move(0, 0);
  await button.hover();
  await expect(popup).toContainText("2027/02/01 08:00:00");
  expect(methods).toContain("account/usage/resetCredits");
  expect(methods).not.toContain("account/usage/resetQuota");
});

test("scheduled warmup saves fractional hours, repeat and selected accounts, then cancels", async ({ page }) => {
  const { methods, patches } = await mockAccount(page, 0, []);
  await page.goto("/accounts/");
  await page.getByRole("button", { name: "预热选项", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "即时预热" })).toBeVisible();
  await page.getByRole("menuitem", { name: "定时预热" }).click();
  const dialog = page.getByRole("dialog", { name: "定时预热", exact: true });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("多少小时后预热", { exact: true }).fill("0.5");
  await dialog.getByText("reset-demo@example.com", { exact: true }).click();
  await dialog.getByRole("switch", { name: "重复执行", exact: true }).click();
  await dialog.getByRole("button", { name: "保存计划", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  expect(patches[0].backgroundTasks).toMatchObject({ warmupCronEnabled: true, warmupIntervalSecs: 1800,
    warmupRepeat: true, warmupCronAccountIds: ["reset-demo"] });
  expect(methods).not.toContain("account/warmup");
  await page.reload();
  await page.getByRole("button", { name: "预热选项", exact: true }).click();
  await page.getByRole("menuitem", { name: "定时预热" }).click();
  await expect(dialog.getByLabel("多少小时后预热", { exact: true })).toHaveValue("0.5");
  await expect(dialog.getByRole("switch", { name: "重复执行", exact: true })).toBeChecked();
  await expect(dialog).toContainText("预热账号（已选 1 个）");
  await dialog.getByRole("button", { name: "取消定时预热", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  expect(patches[1].backgroundTasks).toMatchObject({ warmupCronEnabled: false });
});

test("keyboard focus shows the zero-credit state", async ({ page }) => {
  await mockAccount(page, 0, [{ expires_at: "2027-01-01T00:00:00Z" }]);
  await page.goto("/accounts/");
  const button = page.getByRole("button", { name: "查询剩余的额度重置次数", exact: true });
  await expect(button).toContainText("次数 0");
  await button.locator("..").focus();
  await expect(page.locator('[data-slot="tooltip-content"]').filter({ hasText: "重置次数详情" })).toContainText("暂无可用重置次数");
});

test("missing expiry data prompts a read-only query", async ({ page }) => {
  await mockAccount(page, null, []);
  await page.goto("/accounts/");
  const button = page.getByRole("button", { name: "查询剩余的额度重置次数", exact: true });
  await expect(button).toContainText("次数 --");
  await button.hover();
  await expect(page.locator('[data-slot="tooltip-content"]').filter({ hasText: "重置次数详情" })).toContainText("暂无过期时间，请点击次数查询");
});
