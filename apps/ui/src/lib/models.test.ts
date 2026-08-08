import { describe, it, expect, vi, afterEach } from 'vitest';
import { createModel, fetchModels } from './models';

function mockFetch(data: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => data,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('models API', () => {
  it('posts to /api/v1/models when creating a model', async () => {
    const fetchMock = mockFetch({ data: { id: 'm1', public_id: 'gpt-4o' } });
    await createModel({
      public_id: 'gpt-4o',
      provider: 'openai',
      provider_profile_id: '11111111-1111-1111-1111-111111111111',
      provider_model_id: 'gpt-4o-2024-11-20',
      capabilities: 'chat',
      pricing: { input_price_per_million: 2.5, output_price_per_million: 10 },
    });
    const [url, init] = (fetchMock as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/models');
    expect(init.method).toBe('POST');
  });

  it('fetches models from /api/v1/models', async () => {
    const fetchMock = mockFetch({
      data: [{ id: 'm1', public_id: 'gpt-4o', provider_model_id: 'gpt-4o', capabilities: ['chat'] }],
    });
    await fetchModels();
    const [url] = (fetchMock as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/models');
  });
});
