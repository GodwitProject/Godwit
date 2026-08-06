import { useInfiniteQuery } from '@tanstack/react-query';
import { fetchLogs, type LogFilters, type LogsPage } from '@/lib/logs';

export const LOGS_PAGE_SIZE = 50;

export function useLogs(filters: LogFilters) {
  return useInfiniteQuery<LogsPage>({
    queryKey: ['logs', filters],
    queryFn: ({ pageParam }) =>
      fetchLogs({ offset: pageParam as number, limit: LOGS_PAGE_SIZE, filters }),
    initialPageParam: 0,
    getNextPageParam: (lastPage, allPages) => {
      if (lastPage.items.length < LOGS_PAGE_SIZE) return undefined;
      return allPages.length * LOGS_PAGE_SIZE;
    },
  });
}
