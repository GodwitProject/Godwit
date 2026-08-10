import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  listModels,
  createModel,
  updateModel,
  deleteModel,
  type UpdateModelInput,
} from '@/lib/models';

const QUERY_KEY = ['models'];

export function useModels() {
  return useQuery({ queryKey: QUERY_KEY, queryFn: listModels });
}

export function useCreateModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: createModel,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY }),
  });
}

export function useUpdateModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateModelInput }) => updateModel(id, input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY }),
  });
}

export function useDeleteModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: deleteModel,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY }),
  });
}
