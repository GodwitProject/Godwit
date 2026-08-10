import { z } from 'zod';
import { apiFetch } from './http';
import type { ProviderProfile } from '@/types';

const protocolEnum = z.enum(['openai', 'azure_openai', 'anthropic', 'google', 'custom']);

export const createProviderProfileSchema = z.object({
  name: z.string().min(1),
  protocol: protocolEnum,
  base_url: z.string().url().optional().or(z.literal('')),
  api_key: z.string().optional(),
  allow_wildcard: z.boolean().default(false),
});

export type CreateProviderProfileInput = z.infer<typeof createProviderProfileSchema>;

export const updateProviderProfileSchema = z.object({
  base_url: z.string().url().optional().or(z.literal('')),
  api_key: z.string().optional(),
  allow_wildcard: z.boolean().optional(),
  enabled: z.boolean().optional(),
});

export type UpdateProviderProfileInput = z.infer<typeof updateProviderProfileSchema>;

function normalizeInput(input: CreateProviderProfileInput | UpdateProviderProfileInput) {
  const body: Record<string, unknown> = { ...input };
  if (body.base_url === '') body.base_url = null;
  if (body.api_key === '') delete body.api_key;
  return body;
}

async function assertOk(res: Response): Promise<void> {
  if (!res.ok) {
    let message: string;
    try {
      const body = (await res.json()) as { message?: unknown };
      message = typeof body.message === 'string' ? body.message : res.statusText;
    } catch {
      message = res.statusText;
    }
    throw new Error(message || `Request failed with status ${res.status}`);
  }
}

export async function listProviderProfiles(): Promise<ProviderProfile[]> {
  const res = await apiFetch('/api/v1/provider-profiles');
  await assertOk(res);
  const json = (await res.json()) as { data: ProviderProfile[] };
  return json.data;
}

export async function createProviderProfile(input: CreateProviderProfileInput): Promise<ProviderProfile> {
  const res = await apiFetch('/api/v1/provider-profiles', {
    method: 'POST',
    body: JSON.stringify(normalizeInput(input)),
  });
  await assertOk(res);
  return (await res.json()) as ProviderProfile;
}

export async function updateProviderProfile(
  id: string,
  input: UpdateProviderProfileInput
): Promise<ProviderProfile> {
  const res = await apiFetch(`/api/v1/provider-profiles/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(normalizeInput(input)),
  });
  await assertOk(res);
  return (await res.json()) as ProviderProfile;
}

export async function deleteProviderProfile(id: string): Promise<void> {
  const res = await apiFetch(`/api/v1/provider-profiles/${id}`, { method: 'DELETE' });
  await assertOk(res);
}
