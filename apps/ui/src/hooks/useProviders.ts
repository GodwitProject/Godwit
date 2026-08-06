import { useQuery } from '@tanstack/react-query';
import {
  fetchProviders,
  fetchProviderHealth,
  fetchProviderModels,
  fetchProviderFallbacks,
} from '@/lib/providers';

export function useProviders() {
  return useQuery({
    queryKey: ['providers'],
    queryFn: fetchProviders,
  });
}

export function useProviderHealth() {
  return useQuery({
    queryKey: ['providers', 'health'],
    queryFn: fetchProviderHealth,
    refetchInterval: 10000, // 10 seconds
  });
}

export function useProviderModels(providerId?: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'models'],
    queryFn: () => fetchProviderModels(providerId!),
    enabled: !!providerId,
  });
}

export function useProviderFallbacks(providerId?: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'fallbacks'],
    queryFn: () => fetchProviderFallbacks(providerId!),
    enabled: !!providerId,
  });
}
