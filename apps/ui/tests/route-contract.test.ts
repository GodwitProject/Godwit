import { describe, it, expect, vi, afterEach } from 'vitest';
import * as auth from '@/lib/auth';
import * as models from '@/lib/models';
import * as providerProfiles from '@/lib/providerProfiles';

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

function stripQuery(url: string): string {
  const i = url.indexOf('?');
  return i >= 0 ? url.slice(0, i) : url;
}

function concretePath(path: string): string {
  return path.replace(/\{[^}]+\}/g, ZERO_UUID);
}

async function invoke(entry: ContractEntry): Promise<void> {
  switch (entry.frontend!.fn) {
    case 'login':
      await auth.login('admin@example.com', 'secret');
      break;
    case 'logout':
      await auth.logout();
      break;
    case 'fetchMe':
      await auth.fetchMe();
      break;
    case 'listModels':
      await models.listModels();
      break;
    case 'createModel':
      await models.createModel({
        public_id: 'm',
        provider_profile_id: ZERO_UUID,
        provider_model_id: 'gpt-4',
        provider: 'openai',
        capabilities: ['chat'],
        input_price_per_million: 1,
        output_price_per_million: 2,
      });
      break;
    case 'updateModel':
      await models.updateModel(ZERO_UUID, { public_id: 'm', capabilities: ['chat'] });
      break;
    case 'deleteModel':
      await models.deleteModel(ZERO_UUID);
      break;
    case 'listProviderProfiles':
      await providerProfiles.listProviderProfiles();
      break;
    case 'createProviderProfile':
      await providerProfiles.createProviderProfile({
        name: 'p',
        protocol: 'openai',
        allow_wildcard: false,
      });
      break;
    case 'updateProviderProfile':
      await providerProfiles.updateProviderProfile(ZERO_UUID, {});
      break;
    case 'deleteProviderProfile':
      await providerProfiles.deleteProviderProfile(ZERO_UUID);
      break;
    default:
      throw new Error(`no invoke case for frontend.fn "${entry.frontend!.fn}"`);
  }
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
    const entries = contract as unknown as ContractEntry[];
    expect(entries.length).toBeGreaterThan(0);
    expect(entries.filter((e) => e.scope === 'ui').length).toBeGreaterThan(0);
  });

  it('every UI lib function targets its contract path+method', async () => {
    const entries = (contract as unknown as ContractEntry[]).filter(
      (e) => e.scope === 'ui' && e.frontend
    );

    for (const entry of entries) {
      const m = getMock({ data: [] });
      await invoke(entry);
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
});
