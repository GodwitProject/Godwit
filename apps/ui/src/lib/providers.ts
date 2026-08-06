export interface FallbackChain {
  id: string;
  primary: string;
  fallbacks: string[];
  triggered: number;
}

export interface Provider {
  id: string;
  name: string;
  status: 'healthy' | 'degraded' | 'down';
  modelCount: number;
  latencyP95: number;
  errorRate: number;
  baseUrl: string;
  apiKey: string;
  timeoutMs: number;
  enabledModels: string[];
  fallbackChain: string[];
  fallbackTriggered: number;
}

export type ProviderHealth = Pick<Provider, 'id' | 'name' | 'status'>;

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}

export async function fetchProviders(): Promise<Provider[]> {
  return getJson<Provider[]>('/providers');
}

export async function fetchProviderHealth(): Promise<ProviderHealth[]> {
  return getJson<ProviderHealth[]>('/providers/health');
}

export async function fetchProviderModels(providerId: string): Promise<string[]> {
  return getJson<string[]>(`/providers/${providerId}/models`);
}

export async function fetchProviderFallbacks(providerId: string): Promise<FallbackChain> {
  return getJson<FallbackChain>(`/providers/${providerId}/fallbacks`);
}
