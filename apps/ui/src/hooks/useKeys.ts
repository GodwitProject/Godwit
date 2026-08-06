import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  fetchKeys,
  createKey,
  deleteKey,
  blockKey,
  unblockKey,
  type CreateKeyRequest,
} from '@/lib/keys';

export function useKeys() {
  return useQuery({
    queryKey: ['keys'],
    queryFn: fetchKeys,
  });
}

export function useCreateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (req: CreateKeyRequest) => createKey(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
    },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
    },
  });
}

export function useBlockKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => blockKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
    },
  });
}

export function useUnblockKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => unblockKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
    },
  });
}
