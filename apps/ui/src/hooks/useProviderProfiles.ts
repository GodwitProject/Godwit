import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  listProviderProfiles,
  createProviderProfile,
  updateProviderProfile,
  deleteProviderProfile,
  type UpdateProviderProfileInput,
} from '@/lib/providerProfiles';

const QUERY_KEY = ['providerProfiles'];

export function useProviderProfiles() {
  return useQuery({ queryKey: QUERY_KEY, queryFn: listProviderProfiles });
}

export function useCreateProviderProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: createProviderProfile,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY }),
  });
}

export function useUpdateProviderProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateProviderProfileInput }) =>
      updateProviderProfile(id, input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY }),
  });
}

export function useDeleteProviderProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: deleteProviderProfile,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY }),
  });
}
