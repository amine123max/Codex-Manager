import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import ts from '../node_modules/typescript/lib/typescript.js';

async function client(response) {
  const calls = [];
  const source = (await fs.readFile(new URL('../src/lib/api/quota-client.ts', import.meta.url), 'utf8'))
    .replace('import { invoke, withAddr } from "@/lib/api/transport";', `
      const invoke = async (...args) => { calls.push(args); if (response instanceof Error) throw response; return response; };
      const withAddr = () => ({ addr: "127.0.0.1:48760" });`);
  const compiled = ts.transpileModule(source, { compilerOptions: { module:ts.ModuleKind.ES2022, target:ts.ScriptTarget.ES2022 } });
  const { quotaClient } = new Function('calls', 'response', compiled.outputText.replace('export const quotaClient', 'const quotaClient') + '; return { quotaClient };')(calls, response);
  return { quotaClient, calls };
}

test('sync API uses typed transport with service address and enough timeout, without retries', async () => {
  const { quotaClient, calls } = await client({ modelCount:37, ruleCount:83, syncedAt:1793059200, sourceUrl:'https://developers.openai.com/api/docs/pricing' });
  const result = await quotaClient.syncOpenaiPrices();
  assert.equal(result.modelCount,37);
  assert.equal(result.ruleCount,83);
  assert.equal(result.syncedAt,1793059200);
  assert.deepEqual(calls, [['service_quota_model_prices_sync_openai', { addr:'127.0.0.1:48760' }, { timeoutMs:45000, retries:0 }]]);
});

test('sync API surfaces upstream errors to the caller', async () => {
  const { quotaClient } = await client(new Error('HTTP 503'));
  await assert.rejects(quotaClient.syncOpenaiPrices(), /HTTP 503/);
});

test('price list preserves synchronization status and long-context metadata', async () => {
  const { quotaClient } = await client([{ id:'synced-terra', modelPattern:'gpt-6.1-terra', source:'official_sync', billingMode:'standard',
    sourceUrl:'https://developers.openai.com/api/docs/pricing', updatedAt:1793059200, inputPricePer1m:1, outputPricePer1m:5,
    longContextThresholdTokens:272000, longContextInputPricePer1m:2, longContextOutputPricePer1m:7.5 }]);
  const [price] = await quotaClient.modelPriceRules();
  assert.equal(price.source,'official_sync');
  assert.equal(price.updatedAt,1793059200);
  assert.equal(price.longContextOutputPricePer1m,7.5);
});
