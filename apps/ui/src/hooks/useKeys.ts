import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  fetchKeys,
  createKey,
  updateKey,
  deleteKey,
  revokeKey,
  fetchKeyUsage,
  fetchKeyLogs,
  type CreateKeyRequest,
  type UpdateKeyRequest,
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

export function useUpdateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, req }: { id: string; req: UpdateKeyRequest }) => updateKey(id, req),
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

export function useRevokeKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => revokeKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
    },
  });
}

export function useKeyUsage(id?: string) {
  return useQuery({
    queryKey: ['keys', id, 'usage'],
    queryFn: () => fetchKeyUsage(id!),
    enabled: !!id,
  });
}

export function useKeyLogs(id?: string) {
  return useQuery({
    queryKey: ['keys', id, 'logs'],
    queryFn: () => fetchKeyLogs(id!),
    enabled: !!id,
  });
}
