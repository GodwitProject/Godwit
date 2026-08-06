import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { MetricsSocket } from './websocket';

class FakeWebSocket {
  static OPEN = 1;
  static instances: FakeWebSocket[] = [];

  url: string;
  readyState: number = 0;
  onopen: ((event?: unknown) => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  sent: string[] = [];

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = 3;
    this.onclose?.();
  }

  open() {
    this.readyState = 1;
    this.onopen?.();
  }

  receiveMessage(msg: string) {
    this.onmessage?.({ data: msg });
  }

  error() {
    this.onerror?.();
  }
}

describe('MetricsSocket', () => {
  let socket: MetricsSocket;
  let originalWS: typeof globalThis.WebSocket;

  beforeEach(() => {
    originalWS = globalThis.WebSocket;
    vi.stubGlobal('WebSocket', FakeWebSocket);
    FakeWebSocket.instances = [];
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
    socket?.close();
  });

  it('connects and subscribes to the metrics channel', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    socket.connect();

    const ws = FakeWebSocket.instances[0];
    expect(ws).toBeDefined();
    expect(ws.url).toBe('ws://test/api/v1/ws/metrics');

    ws.open();
    expect(ws.sent).toContain(JSON.stringify({ type: 'subscribe', channel: 'metrics' }));
  });

  it('dispatches metrics:update messages to listeners', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    socket.connect();
    const ws = FakeWebSocket.instances[0];
    ws.open();

    const listener = vi.fn();
    socket.subscribe(listener);

    ws.receiveMessage(
      JSON.stringify({
        type: 'metrics:update',
        data: { requestsTotal: 10, latencyP95Ms: 120, tokensTotal: 500, errorRate: 0.02, timestamp: 't' },
      })
    );

    expect(listener).toHaveBeenCalledWith({
      requestsTotal: 10,
      latencyP95Ms: 120,
      tokensTotal: 500,
      errorRate: 0.02,
      timestamp: 't',
    });
  });

  it('does not dispatch unknown or malformed messages', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    socket.connect();
    const ws = FakeWebSocket.instances[0];
    ws.open();

    const listener = vi.fn();
    socket.subscribe(listener);

    ws.receiveMessage(JSON.stringify({ type: 'other', data: { foo: 1 } }));
    ws.receiveMessage('not json');

    expect(listener).not.toHaveBeenCalled();
  });

  it('reconnects with backoff after close', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    socket.connect();

    expect(FakeWebSocket.instances.length).toBe(1);
    FakeWebSocket.instances[0].close();

    vi.advanceTimersByTime(2000);
    expect(FakeWebSocket.instances.length).toBe(2);

    FakeWebSocket.instances[1].close();
    vi.advanceTimersByTime(5000);
    expect(FakeWebSocket.instances.length).toBe(3);
  });

  it('signals wsFailed after exhausting retry delays', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    const onWsfailed = vi.fn();
    socket.onWsfailed = onWsfailed;
    socket.connect();

    // 3 retries after the initial attempt (2s, 5s, 10s) = 4 connections total
    const total = 1 + 3;
    for (let i = 1; i < total; i++) {
      FakeWebSocket.instances[i - 1].close();
      const delays = [2000, 5000, 10000];
      vi.advanceTimersByTime(delays[i - 1]);
    }

    FakeWebSocket.instances[total - 1].close();
    expect(onWsfailed).toHaveBeenCalled();
  });

  it('signals wsFailed on error', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    const onWsfailed = vi.fn();
    socket.onWsfailed = onWsfailed;
    socket.connect();

    // initial connection errors -> handleFailure -> schedule reconnect after 2000
    FakeWebSocket.instances[0].error();

    vi.advanceTimersByTime(2000);
    FakeWebSocket.instances[1].error();

    vi.advanceTimersByTime(5000);
    FakeWebSocket.instances[2].error();

    vi.advanceTimersByTime(10000);
    FakeWebSocket.instances[3].error();

    expect(onWsfailed).toHaveBeenCalled();
  });

  it('stops reconnecting after close is called', () => {
    socket = new MetricsSocket('ws://test/api/v1/ws/metrics');
    socket.connect();
    FakeWebSocket.instances[0].close();

    socket.close();
    vi.advanceTimersByTime(100000);
    expect(FakeWebSocket.instances.length).toBe(1);
  });
});
