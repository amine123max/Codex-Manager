import assert from "node:assert/strict";
import fs from "node:fs/promises";
import test from "node:test";
import ts from "../node_modules/typescript/lib/typescript.js";

const source = await fs.readFile(new URL("../src/lib/utils/reset-credits.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: {
  module: ts.ModuleKind.ES2022, target: ts.ScriptTarget.ES2022,
} });
const { getResetCreditExpirations, formatResetCreditExpiry } = await import(
  `data:text/javascript;base64,${Buffer.from(compiled.outputText).toString("base64")}`
);

test("reset-card expirations are sorted without confusing quota reset timestamps", () => {
  const rows = getResetCreditExpirations(JSON.stringify({
    resets_at: 123,
    rate_limit_reset_credits: { available_count: 2, credits: [
      { expires_at: "2026-12-01T00:00:00Z" },
      { expiresAt: "2026-11-01T00:00:00Z" },
      { expires_at: "2026-10-01T00:00:00Z", status: "consumed" },
      { reset_type: "other" }, null,
    ] },
  }));
  assert.equal(rows.length, 2);
  assert.equal(rows[0].expiresAt, "2026-11-01T00:00:00Z");
});

test("unknown or malformed metadata never invents expiration dates", () => {
  for (const value of [null, "", "broken", "null", "[]", '{"balance":10}']) {
    assert.deepEqual(getResetCreditExpirations(value), []);
  }
  const rows = getResetCreditExpirations('{"rateLimitResetCredits":{"credits":[{}, {"expiresAt":"invalid"}]}}');
  assert.equal(rows.length, 2);
  assert.equal(formatResetCreditExpiry(rows[0], "zh-CN", "未知"), "未知");
  assert.equal(formatResetCreditExpiry(rows[1], "zh-CN", "未知"), "invalid");
});

test("expiration formatting uses the selected language and retains seconds", () => {
  const [credit] = getResetCreditExpirations('{"credits":[{"expires_at":"2026-11-01T12:34:56Z"}]}');
  assert.match(formatResetCreditExpiry(credit, "zh-CN", "未知"), /2026/);
  assert.match(formatResetCreditExpiry(credit, "en", "Unknown"), /:56/);
});
