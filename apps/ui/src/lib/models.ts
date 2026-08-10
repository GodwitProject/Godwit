import { z } from 'zod';
import { apiFetch } from './http';
import type { Model } from '@/types';

const capabilityEnum = z.enum(['chat', 'embedding', 'vision', 'tool_calling']);

export const createModelSchema = z.object({
  public_id: z.string().min(1).regex(/^[a-z0-9._-]+$/i),
  provider_profile_id: z.string().uuid(),
  provider_model_id: z.string().min(1),
  provider: z.string().min(1),
  capabilities: z.array(capabilityEnum),
  input_price_per_million: z.coerce.number().min(0),
  output_price_per_million: z.coerce.number().min(0),
});

export type CreateModelInput = z.infer<typeof createModelSchema>;

export const updateModelSchema = z.object({
  public_id: z.string().min(1).regex(/^[a-z0-9._-]+$/i),
  capabilities: z.array(capabilityEnum),
});

export type UpdateModelInput = z.infer<typeof updateModelSchema>;

function capabilitiesToString(caps: string[]) {
  if (caps.length === 0) return 'chat';
  return caps.join(',');
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

export async function listModels(): Promise<Model[]> {
  const res = await apiFetch('/api/v1/models');
  await assertOk(res);
  const json = (await res.json()) as { data: Model[] };
  return json.data;
}

export async function createModel(input: CreateModelInput): Promise<Model> {
  const res = await apiFetch('/api/v1/models', {
    method: 'POST',
    body: JSON.stringify({
      public_id: input.public_id,
      provider_profile_id: input.provider_profile_id,
      provider_model_id: input.provider_model_id,
      provider: input.provider,
      capabilities: capabilitiesToString(input.capabilities),
      pricing: {
        input_price_per_million: input.input_price_per_million,
        output_price_per_million: input.output_price_per_million,
      },
    }),
  });
  await assertOk(res);
  return (await res.json()) as Model;
}

export async function updateModel(id: string, input: UpdateModelInput): Promise<Model> {
  const res = await apiFetch(`/api/v1/models/${id}`, {
    method: 'PATCH',
    body: JSON.stringify({
      public_id: input.public_id,
      capabilities: capabilitiesToString(input.capabilities),
    }),
  });
  await assertOk(res);
  return (await res.json()) as Model;
}

export async function deleteModel(id: string): Promise<void> {
  const res = await apiFetch(`/api/v1/models/${id}`, { method: 'DELETE' });
  await assertOk(res);
}
