import { useQuery } from '@tanstack/react-query';
import { fetchModels } from '@/lib/models';

export function useModels() {
  return useQuery({
    queryKey: ['models'],
    queryFn: fetchModels,
  });
}
