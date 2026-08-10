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
      enabled: true,
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
