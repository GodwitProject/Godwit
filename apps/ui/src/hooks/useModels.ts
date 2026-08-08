import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { createModel, fetchModels, type CreateModelRequest } from '@/lib/models';

export function useModels() {
  return useQuery({
    queryKey: ['models'],
    queryFn: fetchModels,
  });
}

export function useCreateModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (req: CreateModelRequest) => createModel(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['models'] });
    },
  });
}
