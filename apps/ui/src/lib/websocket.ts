export interface MetricsUpdate {
  requestsTotal: number;
  latencyP95Ms: number;
  tokensTotal: number;
  errorRate: number;
  timestamp: string;
}

export type IncomingMessage =
  | { type: 'metrics:update'; data: MetricsUpdate }
  | { type: string; data?: unknown };

export interface SubscribeMessage {
  type: 'subscribe';
  channel: string;
}

const RETRY_DELAYS = [2000, 5000, 10000];

export type MetricsSocketStatus = 'connecting' | 'open' | 'failed';

type Listener = (update: MetricsUpdate) => void;

export class MetricsSocket {
  private ws: WebSocket | null = null;
  private url: string;
  private listeners = new Set<Listener>();
  private retryCount = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private connectTimer: ReturnType<typeof setTimeout> | null = null;
  private closed = false;
  status: MetricsSocketStatus = 'connecting';
  onWsfailed: (() => void) | null = null;

  constructor(url?: string) {
    this.url = url ?? `${process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3000/api/v1/ws'}/metrics`;
  }

  connect() {
    this.closed = false;
    this.retryCount = 0;
    this.openSocket();
  }

  private openSocket() {
    if (this.closed) return;
    this.status = 'connecting';
    const ws = new WebSocket(this.url);
    this.ws = ws;

    this.connectTimer = setTimeout(() => {
      this.handleFailure();
    }, 5000);

    ws.onopen = () => {
      if (this.connectTimer) clearTimeout(this.connectTimer);
      this.status = 'open';
      this.retryCount = 0;
      this.send({ type: 'subscribe', channel: 'metrics' });
    };

    ws.onmessage = (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data as string) as IncomingMessage;
        if (msg.type === 'metrics:update' && msg.data) {
          this.dispatch(msg.data as MetricsUpdate);
        }
      } catch {
        // ignore malformed messages
      }
    };

    ws.onclose = () => {
      this.handleFailure();
    };

    ws.onerror = () => {
      this.handleFailure();
    };
  }

  private handleFailure() {
    if (this.closed) return;
    if (this.connectTimer) clearTimeout(this.connectTimer);
    if (this.ws) {
      this.ws.onclose = null;
      this.ws.onerror = null;
      try {
        this.ws.close();
      } catch {
        // already closing
      }
    }

    if (this.retryCount < RETRY_DELAYS.length) {
      const delay = RETRY_DELAYS[this.retryCount];
      this.retryCount += 1;
      this.status = 'connecting';
      this.reconnectTimer = setTimeout(() => this.openSocket(), delay);
    } else {
      this.status = 'failed';
      this.onWsfailed?.();
    }
  }

  private send(message: SubscribeMessage) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private dispatch(update: MetricsUpdate) {
    this.listeners.forEach((listener) => listener(update));
  }

  close() {
    this.closed = true;
    if (this.connectTimer) clearTimeout(this.connectTimer);
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    if (this.ws) {
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.close();
      this.ws = null;
    }
    this.listeners.clear();
  }
}
