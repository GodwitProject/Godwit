import { useQuery } from '@tanstack/react-query';
import { fetchMetrics, fetchLatency, fetchTokens } from '@/lib/api';

export function useMetrics() {
  return useQuery({
    queryKey: ['metrics'],
    queryFn: fetchMetrics,
    refetchInterval: 5000, // 5 seconds
  });
}

export function useLatency() {
  return useQuery({
    queryKey: ['latency'],
    queryFn: fetchLatency,
  });
}

export function useTokens() {
  return useQuery({
    queryKey: ['tokens'],
    queryFn: fetchTokens,
  });
}
