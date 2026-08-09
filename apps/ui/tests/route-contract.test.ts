import { describe, it, expect, vi, afterEach } from 'vitest';
import * as auth from '@/lib/auth';
import * as keys from '@/lib/keys';
import * as models from '@/lib/models';
import * as providers from '@/lib/providers';
import * as logs from '@/lib/logs';
import * as api from '@/lib/api';
import * as http from '@/lib/http';
import { MetricsSocket } from '@/lib/websocket';

import contract from '../../../contract/routes.json';

const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

interface ContractEntry {
  id: string;
  method: string;
  path: string;
  frontend: { lib: string; fn: string } | null;
  scope: string;
}

type FetchCall = { url: string; method: string };

/** Strip the query string to compare against the contract path. */
function stripQuery(url: string): string {
  const i = url.indexOf('?');
  return i >= 0 ? url.slice(0, i) : url;
}

/** Replace every `{param}` segment with the sentinel id, matching what the UI passes. */
function concretePath(path: string): string {
  return path.replace(/\{[^}]+\}/g, ZERO_UUID);
}

/**
 * Invoke the FE lib function for a contract route with suitable args, returning the
 * fetch calls it made. A no-op (empty calls) means the route is exercised some other
 * way (e.g. the WebSocket), handled separately.
 */
async function invoke(entry: ContractEntry, mock: ReturnType<typeof vi.fn>): Promise<void> {
  switch (entry.frontend!.fn) {
    case 'login': await auth.login('admin@example.com', 'secret'); break;
    case 'logout': await auth.logout(); break;
    case 'fetchMe': await auth.fetchMe(); break;
    case 'doRefresh': await http.doRefresh(); break;
    case 'fetchKeys': await keys.fetchKeys(); break;
    case 'createKey': await keys.createKey({ name: 'k', scopes: [], allowed_models: [] }); break;
    case 'blockKey': await keys.blockKey(ZERO_UUID); break;
    case 'unblockKey': await keys.unblockKey(ZERO_UUID); break;
    case 'deleteKey': await keys.deleteKey(ZERO_UUID); break;
    case 'fetchModels': await models.fetchModels(); break;
    case 'createModel': await models.createModel({
      public_id: 'm', provider: 'openai', provider_profile_id: ZERO_UUID,
      provider_model_id: 'gpt-4', capabilities: 'chat', pricing: { input_price_per_million: 1, output_price_per_million: 2 },
    }); break;
    case 'fetchProviders': await providers.fetchProviders(); break;
    case 'setProviderEnabled': await providers.setProviderEnabled(ZERO_UUID, true); break;
    case 'fetchSpend': await api.fetchSpend(); break;
    case 'fetchLogs': await logs.fetchLogs(); break;
    case 'fetchStats': await api.fetchStats(); break;
    case 'fetchPrometheusMetrics': await api.fetchPrometheusMetrics(); break;
    default:
      throw new Error(`no invoke case for frontend.fn "${entry.frontend!.fn}"`);
  }
  void mock;
}

function getFetchCalls(mock: ReturnType<typeof vi.fn>): FetchCall[] {
  return mock.mock.calls.map((args: unknown[]) => {
    const [url, init] = args as [string, RequestInit | undefined];
    return { url: String(url), method: (init && init.method) || 'GET' };
  });
}

describe('route contract — every UI call matches the backend contract', () => {
  afterEach(() => vi.unstubAllGlobals());

  function getMock(data: unknown = {}) {
    const m = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => data,
      text: async () => '',
    });
    vi.stubGlobal('fetch', m);
    return m;
  }

  it('contains entries', () => {
    const entries = contract as ContractEntry[];
    expect(entries.length).toBeGreaterThan(0);
    expect(entries.filter((e) => e.scope === 'ui').length).toBeGreaterThan(0);
  });

  it('every UI lib function targets its contract path+method', async () => {
    const entries = (contract as ContractEntry[]).filter(
      (e) => e.scope === 'ui' && e.frontend
    );

    for (const entry of entries) {
      // The WebSocket is not fetch-based; verified separately below against the contract path.
      if (entry.frontend!.fn === 'MetricsSocket') continue;

      const m = getMock({ data: [] });
      await invoke(entry, m);
      const calls = getFetchCalls(m);

      expect(
        calls.length,
        `${entry.id} should have performed a fetch`
      ).toBeGreaterThan(0);

      const expectedPath = concretePath(entry.path);
      const matched = calls.some(
        (c) => c.method === entry.method && stripQuery(c.url) === expectedPath
      );
      expect(
        matched,
        `${entry.id} expected ${entry.method} ${expectedPath}; got ${JSON.stringify(calls)}`
      ).toBe(true);
    }
  });

  it('MetricsSocket defaults to the contract ws.metrics path', () => {
    class FakeWebSocket {
      url: string;
      static instances: FakeWebSocket[] = [];
      constructor(url: string) { this.url = url; FakeWebSocket.instances.push(this); }
      send() {}
      close() {}
    }
    vi.stubGlobal('WebSocket', FakeWebSocket as unknown as typeof WebSocket);

    const socket = new MetricsSocket();
    socket.connect();
    const inst = FakeWebSocket.instances[0];
    expect(new URL(inst.url).pathname).toBe('/api/v1/ws/metrics');
    socket.close();
  });
});
