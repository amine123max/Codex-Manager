import { expect, test, type Page } from "@playwright/test";

async function setupLogs(page: Page, locale = "zh-CN", theme = "tech") {
  const rows = [
    { traceId: "latency-normal", firstResponseMs: 566, durationMs: 5550, outputTokens: 611, reasoningOutputTokens: 100 },
    { traceId: "latency-missing", firstResponseMs: null, durationMs: null, outputTokens: null },
    { traceId: "latency-slow", firstResponseMs: 35_000, durationMs: 200_000, outputTokens: 1000 },
    { traceId: "latency-image", firstResponseMs: null, durationMs: 2000, outputTokens: 1000, originalPath: "/v1/images/generations" },
    { traceId: "latency-no-first", firstResponseMs: null, durationMs: 2345, outputTokens: 34 },
  ].map(row => ({ model: row.traceId, requestPath: "/v1/responses", originalPath: "/v1/responses",
    method: "POST", statusCode: 200, createdAt: 1793678400, inputTokens: 1000, totalTokens: 1611, ...row }));
  await page.route("**/api/runtime**", route => route.fulfill({ json: { mode: "web-gateway", rpcBaseUrl: "/api/rpc", canManageService: false } }));
  await page.route("**/api/rpc", async route => {
    const { id, method } = route.request().postDataJSON();
    let result: unknown = { items: [] };
    if (method === "appSettings/get") result = { locale, theme, codexCliGuideDismissed: true,
      serviceAddr: "localhost:48760", backgroundTasks: { usagePollingEnabled: false } };
    if (method === "initialize") result = { userAgent: "codex_cli_rs/0.1.19", codexHome: "C:/Test", platformFamily: "windows", platformOs: "windows" };
    if (method === "accountManager/session/current") result = { mode: "none", role: "system_admin", currentUser: null, permissions: [] };
    if (method === "requestlog/list") result = { items: rows, total: rows.length, page: 1, pageSize: 20 };
    if (method === "requestlog/summary") result = { totalCount: rows.length, filteredCount: rows.length,
      successCount: rows.length, errorCount: 0, totalTokens: 8055, totalCostUsd: 0 };
    await route.fulfill({ json: { jsonrpc: "2.0", id, result } });
  });
}

test("latency column matches the reference with three rows and full-duration TPS", async ({ page }, testInfo) => {
  await setupLogs(page);
  await page.goto("/logs/");
  await expect(page.getByRole("columnheader", { name: "延迟", exact: true })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "用时 / 首响", exact: true })).toHaveCount(0);
  const normal = page.getByRole("row").filter({ hasText: "latency-normal" }).getByTestId("request-latency");
  await expect(normal).toContainText("首字566ms总耗时5.55s输出 TPS110.1 tok/s");
  await expect(normal.locator('[aria-hidden="true"]')).toHaveClass(/from-emerald-500.*to-emerald-500/);
  const missing = page.getByRole("row").filter({ hasText: "latency-missing" }).getByTestId("request-latency");
  await expect(missing.locator("dd")).toHaveText(["—", "—", "—"]);
  await expect(missing.locator('[aria-hidden="true"]')).toHaveClass(/bg-muted-foreground/);
  const slow = page.getByRole("row").filter({ hasText: "latency-slow" }).getByTestId("request-latency");
  await expect(slow.locator("dd")).toHaveText(["35.00s", "3m 20s", "5.0 tok/s"]);
  await expect(slow.locator("dd").first()).toHaveClass(/text-orange-600/);
  await expect(page.getByRole("row").filter({ hasText: "latency-image" }).getByTestId("output-tps")).toHaveText("—");
  await expect(page.getByRole("row").filter({ hasText: "latency-no-first" }).getByTestId("output-tps")).toHaveText("14.5 tok/s");
  await normal.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("latency-column.png") });
});

test("latency column supports English and dark themes", async ({ page }) => {
  await setupLogs(page, "en", "dark");
  await page.goto("/logs/");
  await expect(page.getByRole("columnheader", { name: "Latency", exact: true })).toBeVisible();
  const cell = page.getByRole("row").filter({ hasText: "latency-normal" }).getByTestId("request-latency");
  await expect(cell).toContainText("First token");
  await expect(cell).toContainText("Total time");
  await expect(cell.getByTestId("output-tps")).toHaveText("110.1 tok/s");
});
