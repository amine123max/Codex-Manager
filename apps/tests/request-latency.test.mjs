import assert from "node:assert/strict";
import fs from "node:fs/promises";
import test from "node:test";
import ts from "../node_modules/typescript/lib/typescript.js";

const source = await fs.readFile(new URL("../src/lib/utils/request-latency.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: {
  module: ts.ModuleKind.ES2022, target: ts.ScriptTarget.ES2022,
} });
const { formatOutputTokensPerSecond, formatRequestLatency, firstResponseSeverity, durationSeverity } = await import(
  `data:text/javascript;base64,${Buffer.from(compiled.outputText).toString("base64")}`
);

test("output TPS matches sub2api's full-duration calculation and one-decimal precision", () => {
  assert.equal(formatOutputTokensPerSecond(611, 5550), "110.1 tok/s");
  assert.equal(formatOutputTokensPerSecond(1000, 10_000), "100.0 tok/s");
  assert.equal(formatOutputTokensPerSecond(1, 100_000), "0.0 tok/s");
  assert.equal(formatOutputTokensPerSecond(34, 2345), "14.5 tok/s");
});

test("unknown, nonpositive, nonfinite and overflow values never invent TPS", () => {
  for (const value of [null, 0, -1, NaN, Infinity, -Infinity]) {
    assert.equal(formatOutputTokensPerSecond(value, 1000), "—");
    assert.equal(formatOutputTokensPerSecond(611, value), "—");
  }
  assert.equal(formatOutputTokensPerSecond(Number.MAX_VALUE, Number.MIN_VALUE), "—");
});

test("latency formatting retains milliseconds, two decimal seconds and long durations", () => {
  for (const [ms, expected] of [[0, "0ms"], [566, "566ms"], [999, "999ms"], [1000, "1.00s"],
    [5550, "5.55s"], [46_000, "46.00s"], [60_000, "1m 0s"], [90_000, "1m 30s"], [3_600_000, "1h 0m"]]) {
    assert.equal(formatRequestLatency(ms), expected);
  }
  for (const value of [null, -1, NaN, Infinity]) assert.equal(formatRequestLatency(value), "—");
});

test("latency colors follow sub2api thresholds and keep missing values neutral", () => {
  for (const [value, expected] of [[0, "good"], [9999, "good"], [10000, "warn"], [30000, "slow"], [60000, "critical"]]) {
    assert.equal(firstResponseSeverity(value), expected);
  }
  for (const [value, expected] of [[59999, "good"], [60000, "warn"], [180000, "slow"], [300000, "critical"]]) {
    assert.equal(durationSeverity(value), expected);
  }
  assert.equal(firstResponseSeverity(null), "unknown");
  assert.equal(durationSeverity(-1), "unknown");
});
