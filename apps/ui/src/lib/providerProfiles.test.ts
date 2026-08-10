import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  listProviderProfiles,
  createProviderProfile,
  updateProviderProfile,
  deleteProviderProfile,
} from './providerProfiles';

const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('providerProfiles', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('lists profiles', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        data: [
          {
            id: '1',
            name: 'openai',
            protocol: 'openai',
            base_url: 'https://api.openai.com/v1',
            allow_wildcard: false,
            enabled: true,
            has_credentials: true,
            created_at: '2024-01-01T00:00:00Z',
          },
        ],
      }),
    } as Response);

    const profiles = await listProviderProfiles();
    expect(profiles).toHaveLength(1);
    expect(profiles[0].name).toBe('openai');
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/provider-profiles',
      expect.objectContaining({ credentials: 'include' })
    );
  });

  it('creates a profile', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        id: '1',
        name: 'openai',
        protocol: 'openai',
        base_url: 'https://api.openai.com/v1',
        allow_wildcard: false,
        enabled: true,
        has_credentials: true,
        created_at: '2024-01-01T00:00:00Z',
      }),
    } as Response);

    const profile = await createProviderProfile({
      name: 'openai',
      protocol: 'openai',
      base_url: 'https://api.openai.com/v1',
      api_key: 'sk-test',
      allow_wildcard: false,
    });

    expect(profile.name).toBe('openai');
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/provider-profiles',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        body: expect.stringContaining('"api_key":"sk-test"'),
      })
    );
  });

  it('normalizes empty base_url to null on create', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        id: '1',
        name: 'openai',
        protocol: 'openai',
        base_url: null,
        allow_wildcard: false,
        enabled: true,
        has_credentials: false,
        created_at: '2024-01-01T00:00:00Z',
      }),
    } as Response);

    await createProviderProfile({
      name: 'openai',
      protocol: 'openai',
      base_url: '',
      api_key: 'sk-test',
      allow_wildcard: false,
    });

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/provider-profiles',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('"base_url":null'),
      })
    );
  });

  it('omits empty api_key on create', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        id: '1',
        name: 'openai',
        protocol: 'openai',
        base_url: 'https://api.openai.com/v1',
        allow_wildcard: false,
        enabled: true,
        has_credentials: false,
        created_at: '2024-01-01T00:00:00Z',
      }),
    } as Response);

    await createProviderProfile({
      name: 'openai',
      protocol: 'openai',
      base_url: 'https://api.openai.com/v1',
      api_key: '',
      allow_wildcard: false,
    });

    const [, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(init.body).not.toContain('"api_key"');
  });

  it('throws on non-2xx response', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 400,
      json: async () => ({ message: 'Bad request' }),
    } as Response);

    await expect(
      createProviderProfile({
        name: 'openai',
        protocol: 'openai',
        base_url: 'https://api.openai.com/v1',
        api_key: 'sk-test',
        allow_wildcard: false,
      })
    ).rejects.toThrow('Request failed with status 400');
  });

  it('updates a profile', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        id: '1',
        name: 'openai',
        protocol: 'openai',
        base_url: 'https://api.new.com/v1',
        allow_wildcard: true,
        enabled: true,
        has_credentials: true,
        created_at: '2024-01-01T00:00:00Z',
      }),
    } as Response);

    const profile = await updateProviderProfile('1', {
      base_url: 'https://api.new.com/v1',
      api_key: 'new-key',
      allow_wildcard: true,
      enabled: true,
    });

    expect(profile.base_url).toBe('https://api.new.com/v1');
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/provider-profiles/1',
      expect.objectContaining({
        method: 'PATCH',
        credentials: 'include',
        body: expect.stringContaining('"api_key":"new-key"'),
      })
    );
  });

  it('deletes a profile', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({ deleted: true }) } as Response);
    await deleteProviderProfile('1');
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/provider-profiles/1',
      expect.objectContaining({ method: 'DELETE', credentials: 'include' })
    );
  });
});
