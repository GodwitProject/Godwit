import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listModels, createModel, updateModel, deleteModel } from './models';

const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('models', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('lists models', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        data: [
          {
            id: '1',
            public_id: 'gpt-4o',
            provider: 'openai',
            provider_profile_id: 'p1',
            provider_model_id: 'gpt-4o',
            capabilities: ['chat'],
            pricing: { input_price_per_million: 5, output_price_per_million: 15 },
            config: {},
            created_at: '2024-01-01T00:00:00Z',
          },
        ],
      }),
    } as Response);

    const models = await listModels();
    expect(models).toHaveLength(1);
    expect(models[0].public_id).toBe('gpt-4o');
  });

  it('creates a model with provider, comma-separated capabilities and pricing JSON', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        id: '1',
        public_id: 'gpt-4o',
        provider: 'openai',
        provider_profile_id: 'p1',
        provider_model_id: 'gpt-4o',
        capabilities: ['chat', 'embedding'],
        pricing: { input_price_per_million: 5, output_price_per_million: 15 },
        config: {},
        created_at: '2024-01-01T00:00:00Z',
      }),
    } as Response);

    await createModel({
      public_id: 'gpt-4o',
      provider_profile_id: 'p1',
      provider_model_id: 'gpt-4o',
      provider: 'openai',
      capabilities: ['chat', 'embedding'],
      input_price_per_million: 5,
      output_price_per_million: 15,
    });

    const body = JSON.parse(mockFetch.mock.calls[0][1].body);
    expect(body.provider).toBe('openai');
    expect(body.capabilities).toBe('chat,embedding');
    expect(body.pricing).toEqual({ input_price_per_million: 5, output_price_per_million: 15 });
  });

  it('updates only public_id and capabilities', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) } as Response);
    await updateModel('1', { public_id: 'gpt-4o-renamed', capabilities: ['chat'] });
    const body = JSON.parse(mockFetch.mock.calls[0][1].body);
    expect(body).toEqual({ public_id: 'gpt-4o-renamed', capabilities: 'chat' });
  });

  it('deletes a model', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true } as Response);
    await deleteModel('1');
    expect(mockFetch.mock.calls[0][1].method).toBe('DELETE');
  });
});
