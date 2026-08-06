import { useQuery } from '@tanstack/react-query';
import { fetchProviders } from '@/lib/providers';

export function useProviders() {
  return useQuery({
    queryKey: ['providers'],
    queryFn: fetchProviders,
  });
}
