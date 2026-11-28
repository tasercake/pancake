export class WorkspaceDisconnected extends Error {}

export class WorkspaceSocket {
  constructor(onEvent, onConnection) {
    this.onEvent = onEvent;
    this.onConnection = onConnection;
    this.requests = new Map();
    this.nextId = 0;
    this.delay = 500;
  }

  connect() {
    const target = new URL("/api/events", location.href);
    target.protocol = location.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(target);
    this.socket = socket;
    const timeout = setTimeout(() => this.disconnect(socket), 10000);
    socket.onopen = () => {
      if (socket !== this.socket) return;
      clearTimeout(timeout);
      this.delay = 500;
      this.onConnection(true);
    };
    socket.onmessage = ({ data }) => {
      if (socket !== this.socket) return;
      const message = JSON.parse(data);
      if (message.event) {
        this.onEvent(message.event, message.data);
        return;
      }
      const request = this.requests.get(message.id);
      if (!request) return;
      this.requests.delete(message.id);
      clearTimeout(request.timeout);
      if (message.error) {
        request.reject(Object.assign(new Error(message.error), { data: message.data }));
      } else {
        request.resolve(message.data);
      }
    };
    socket.onclose = () => {
      clearTimeout(timeout);
      this.disconnect(socket);
    };
  }

  disconnect(socket) {
    if (socket !== this.socket) return;
    this.socket = null;
    socket.close();
    for (const request of this.requests.values()) {
      clearTimeout(request.timeout);
      request.reject(new WorkspaceDisconnected());
    }
    this.requests.clear();
    this.onConnection(false);
    setTimeout(() => this.connect(), this.delay);
    this.delay = Math.min(this.delay * 2, 8000);
  }

  request(action, data) {
    const socket = this.socket;
    if (socket?.readyState !== WebSocket.OPEN) return Promise.reject(new WorkspaceDisconnected());
    const id = ++this.nextId;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => this.disconnect(socket), 10000);
      this.requests.set(id, { resolve, reject, timeout });
      try {
        socket.send(JSON.stringify({ id, action, data }));
      } catch {
        this.disconnect(socket);
      }
    });
  }
}
