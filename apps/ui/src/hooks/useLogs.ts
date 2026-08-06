import { useQuery } from '@tanstack/react-query';
import { fetchLogs, fetchLog, type LogFilters, type LogsQuery } from '@/lib/logs';

export function useLogs(filters: LogFilters, page: number, pageSize: number) {
  const query: LogsQuery = { page, pageSize, filters };
  return useQuery({
    queryKey: ['logs', query],
    queryFn: () => fetchLogs(query),
  });
}

export function useLog(id?: string) {
  return useQuery({
    queryKey: ['logs', id],
    queryFn: () => fetchLog(id!),
    enabled: !!id,
  });
}
