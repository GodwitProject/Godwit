import { useMutation, useQueryClient } from '@tanstack/react-query';
import { setProviderEnabled } from '@/lib/providers';

export function useSetProviderEnabled() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      setProviderEnabled(id, enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers'] });
    },
  });
}
