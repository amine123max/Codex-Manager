import { expect, test, type Page } from "@playwright/test";

async function setup(page: Page, fail = false, member = false) {
  let syncCalls = 0;
  const custom = { id: "custom", modelPattern: "my-model", provider: "custom", source: "custom", inputPricePer1m: 9, outputPricePer1m: 18, billingMode: "standard" };
  const rules = [custom, { id: "seed-terra", modelPattern: "gpt-6.1-terra", provider: "openai", source: "official_seed", inputPricePer1m: 1, cachedInputPricePer1m: 0.05, outputPricePer1m: 5, billingMode: "standard" }];
  await page.route("**/api/runtime**", route => route.fulfill({ json: { mode: "web-gateway", rpcBaseUrl: "/api/rpc", canManageService: false } }));
  await page.route("**/api/rpc", async route => {
    const { method, id } = route.request().postDataJSON();
    let result: unknown = { items: [] };
    if (method === "appSettings/get") result = { locale: "zh-CN", theme: "tech", codexCliGuideDismissed: true, serviceAddr: "localhost:48760", backgroundTasks: { usagePollingEnabled: false } };
    if (method === "initialize") result = { userAgent: "codex_cli_rs/0.1.19", codexHome: "C:/Test", platformFamily: "windows", platformOs: "windows" };
    if (method === "accountManager/session/current") result = { mode: member ? "accounts" : "none", role: member ? "member" : "system_admin", permissions: [], currentUser: member ? { id: "member", role: "member", username: "member" } : null };
    if (method === "quota/modelPriceRules") result = rules;
    if (method === "quota/modelPriceRules/syncOpenai") {
      syncCalls++;
      await new Promise(resolve => setTimeout(resolve, 250));
      if (fail) {
        await route.fulfill({ json: { jsonrpc: "2.0", id, error: { code: -32000, message: "获取 OpenAI 官方价格失败：HTTP 503" } } });
        return;
      }
      rules.splice(1, 1, { ...rules[1], id: "synced-terra", source: "official_sync", updatedAt: 1793059200 } as typeof rules[number]);
      result = { modelCount: 37, ruleCount: 83, syncedAt: 1793059200, sourceUrl: "https://developers.openai.com/api/docs/pricing" };
    }
    await route.fulfill({ json: { jsonrpc: "2.0", id, result } });
  });
  return () => syncCalls;
}

test("one click synchronizes prices, shows status and preserves custom prices", async ({ page }, testInfo) => {
  const calls = await setup(page);
  await page.goto("/models/");
  const button = page.getByRole("button", { name: "同步 OpenAI 官价", exact: true });
  await expect(button).toBeEnabled();
  await button.click();
  await expect(page.getByRole("button", { name: "正在同步价格...", exact: true })).toBeDisabled();
  await expect(page.getByText("已同步 37 个 OpenAI 模型的官方价格", { exact: true })).toBeVisible();
  await expect(button).toBeEnabled();
  await expect(page.getByText("my-model", { exact: true })).toBeVisible();
  await expect(page.getByText(/最近同步/)).toBeVisible();
  await page.getByText(/查看官方价格/).click();
  await expect(page.getByRole("cell", { name: "官网同步", exact: true })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("price-sync.png") });
  expect(calls()).toBe(1);
  await page.reload();
  await expect(page.getByText(/最近同步/)).toBeVisible();
});

test("failed sync leaves existing prices and permits retry", async ({ page }) => {
  const calls = await setup(page, true);
  await page.goto("/models/");
  const button = page.getByRole("button", { name: "同步 OpenAI 官价", exact: true });
  await button.click();
  await expect(page.getByText(/获取 OpenAI 官方价格失败/)).toBeVisible();
  await expect(button).toBeEnabled();
  await expect(page.getByText("my-model", { exact: true })).toBeVisible();
  expect(calls()).toBe(1);
});

test("members do not see the price sync button", async ({ page }) => {
  const calls = await setup(page, false, true);
  await page.goto("/models/");
  await expect(page.getByRole("main").getByRole("heading", { name: "可用模型", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "同步 OpenAI 官价", exact: true })).toHaveCount(0);
  expect(calls()).toBe(0);
});
