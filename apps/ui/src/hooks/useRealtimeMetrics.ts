'use client';

import { useEffect, useRef, useState } from 'react';
import { create } from 'zustand';
import { MetricsSocket, type MetricsUpdate } from '@/lib/websocket';
import { fetchPrometheusMetrics } from '@/lib/api';

export type RealtimeStatus = 'connecting' | 'live' | 'polling' | 'error';

const useRealtimeMetricsStore = create<MetricsUpdate>(() => ({
  requestsTotal: 0,
  tokensTotal: 0,
  costUsdTotal: 0,
  activeRequests: 0,
  timestamp: '',
}));

interface RealTimeMetricsResult {
  data: MetricsUpdate;
  status: RealtimeStatus;
}

export function useRealtimeMetrics(): RealTimeMetricsResult {
  const [status, setStatus] = useState<RealtimeStatus>('connecting');
  const [data, setData] = useState<MetricsUpdate>(() => useRealtimeMetricsStore.getState());
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
      setData(update);
      setStatus('live');
    });

    function startPolling() {
      if (!active || fallbackTimer.current) return;
      fallbackTimer.current = setInterval(async () => {
        try {
          const metrics = await fetchPrometheusMetrics();
          if (!active) return;
          useRealtimeMetricsStore.setState(metrics);
          setData(metrics);
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
