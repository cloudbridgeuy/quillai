export function installFakeEventSource() {
  type FakeControl = {
    sources: FakeEventSource[];
    ready: (index: number, pieceIds: number[]) => void;
    changed: (index: number, pieceId: number, rev: number) => void;
    resync: (index: number) => void;
    error: (index: number) => void;
    reconnect: (index: number, pieceIds: number[]) => void;
    close: (index: number) => void;
  };

  class FakeEventSource extends EventTarget {
    static readonly CONNECTING = 0;
    static readonly OPEN = 1;
    static readonly CLOSED = 2;
    readonly CONNECTING = 0;
    readonly OPEN = 1;
    readonly CLOSED = 2;
    readonly url: string;
    readonly withCredentials: boolean;
    readyState = FakeEventSource.CONNECTING;
    onopen: ((this: EventSource, ev: Event) => unknown) | null = null;
    onmessage: ((this: EventSource, ev: MessageEvent) => unknown) | null = null;
    onerror: ((this: EventSource, ev: Event) => unknown) | null = null;
    closeCount = 0;

    constructor(url: string | URL, options?: EventSourceInit) {
      super();
      this.url = String(url);
      this.withCredentials = options?.withCredentials ?? false;
      control.sources.push(this);
    }

    emit(type: string, data?: object) {
      if (this.readyState === FakeEventSource.CLOSED) return;
      const event = data === undefined ? new Event(type) : new MessageEvent(type, { data: JSON.stringify(data) });
      this.dispatchEvent(event);
      const handler = type === "open" ? this.onopen : type === "error" ? this.onerror : type === "message" ? this.onmessage : null;
      handler?.call(this as unknown as EventSource, event as MessageEvent);
    }

    close() {
      this.closeCount++;
      this.readyState = FakeEventSource.CLOSED;
    }
  }

  const control: FakeControl = {
    sources: [],
    ready(index, pieceIds) {
      const source = this.sources[index]!;
      if (source.readyState === FakeEventSource.CLOSED) return;
      source.readyState = FakeEventSource.OPEN;
      source.emit("open");
      source.emit("ready", { pieceIds });
    },
    changed(index, pieceId, rev) { this.sources[index]!.emit("changed", { pieceId, rev }); },
    resync(index) { this.sources[index]!.emit("resync", {}); },
    error(index) {
      const source = this.sources[index]!;
      if (source.readyState === FakeEventSource.CLOSED) return;
      source.readyState = FakeEventSource.CONNECTING;
      source.emit("error");
    },
    reconnect(index, pieceIds) { this.ready(index, pieceIds); },
    close(index) { this.sources[index]!.close(); },
  };
  Object.assign(window, { EventSource: FakeEventSource, fakeEventSource: control });
}
