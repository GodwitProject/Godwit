'use client';

import { useEffect, useRef, useState } from 'react';
import { create } from 'zustand';
import { MetricsSocket, type MetricsUpdate, type MetricsSocketStatus } from '@/lib/websocket';
import { fetchMetrics } from '@/lib/api';

export type RealtimeStatus = 'connecting' | 'live' | 'polling' | 'error';

interface RealtimeMetricsState {
  requestsTotal: number | null;
  latencyP95Ms: number | null;
  tokensTotal: number | null;
  errorRate: number | null;
  timestamp: string | null;
}

const useRealtimeMetricsStore = create<RealtimeMetricsState>(() => ({
  requestsTotal: null,
  latencyP95Ms: null,
  tokensTotal: null,
  errorRate: null,
  timestamp: null,
}));

interface PollingMetrics {
  totalRequests?: number;
  errorRate?: number;
  [key: string]: unknown;
}

interface RealTimeMetricsResult {
  data: RealtimeMetricsState;
  status: RealtimeStatus;
}

export function useRealtimeMetrics(): RealTimeMetricsResult {
  const [status, setStatus] = useState<RealtimeStatus>('connecting');
  const [data, setData] = useState<RealtimeMetricsState>(() => useRealtimeMetricsStore.getState());
  const socketRef = useRef<MetricsSocket | null>(null);
  const fallbackTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    let active = true;
    const socket = new MetricsSocket();
    socketRef.current = socket;

    socket.onWsfailed = () => {
      if (!active) return;
      setStatus('polling');
      startPolling();
    };

    socket.subscribe((update: MetricsUpdate) => {
      if (!active) return;
      useRealtimeMetricsStore.setState(update);
      setData(useRealtimeMetricsStore.getState());
      setStatus('live');
    });

    function startPolling() {
      if (!active || fallbackTimer.current) return;
      fallbackTimer.current = setInterval(async () => {
        try {
          const metrics = (await fetchMetrics()) as PollingMetrics;
          if (!active) return;
          const mapped: RealtimeMetricsState = {
            requestsTotal: typeof metrics.totalRequests === 'number' ? metrics.totalRequests : null,
            latencyP95Ms: typeof metrics.p95LatencyMs === 'number' ? metrics.p95LatencyMs : null,
            tokensTotal: typeof metrics.totalTokens === 'number' ? metrics.totalTokens : null,
            errorRate: typeof metrics.errorRate === 'number' ? metrics.errorRate : null,
            timestamp: new Date().toISOString(),
          };
          useRealtimeMetricsStore.setState(mapped);
          setData(mapped);
        } catch {
          if (active) setStatus('error');
        }
      }, 5000);
    }

    socket.connect();

    return () => {
      active = false;
      if (fallbackTimer.current) clearInterval(fallbackTimer.current);
      socket.close();
      socketRef.current = null;
    };
  }, []);

  return { data, status };
}
