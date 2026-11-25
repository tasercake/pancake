"""Push the latest review state to every connected browser."""

from contextlib import contextmanager
import json
from selectors import DefaultSelector, EVENT_READ
from socket import SHUT_WR, socketpair
from threading import Lock


class Events:
    def __init__(self):
        self.lock = Lock()
        self.latest = {}
        self.subscribers = {}
        self.closed = False

    def publish(self, name, snapshot):
        with self.lock:
            if self.closed:
                return
            data = json.dumps(snapshot(), ensure_ascii=True, allow_nan=False)
            payload = f"event: {name}\ndata: {data}\n\n".encode()
            if self.latest.get(name) == payload:
                return
            self.latest[name] = payload
            for writer, pending in self.subscribers.items():
                wake = not pending
                pending[name] = payload
                if wake:
                    writer.sendall(b"\0")

    @contextmanager
    def subscribe(self):
        # A single wake byte per client is enough. Each event name coalesces to
        # its latest snapshot, so a slow connection cannot build up a backlog.
        reader, writer = socketpair()
        with reader, writer:
            with self.lock:
                self.subscribers[writer] = self.latest.copy()
                if self.latest:
                    writer.sendall(b"\0")
                if self.closed:
                    writer.shutdown(SHUT_WR)
            try:
                yield reader, writer
            finally:
                with self.lock:
                    del self.subscribers[writer]

    def take(self, reader, writer):
        if not reader.recv(1):
            return ()
        with self.lock:
            if self.closed:
                return ()
            pending = self.subscribers[writer]
            # Preserve the initial event order even after coalescing updates.
            events = tuple(pending[name] for name in self.latest if name in pending)
            pending.clear()
            return events

    def stream(self, connection, write):
        with self.subscribe() as (reader, writer), DefaultSelector() as selector:
            selector.register(reader, EVENT_READ)
            selector.register(connection, EVENT_READ)
            while True:
                ready = {key.fileobj for key, _ in selector.select()}
                if connection in ready:
                    # Close when the peer disconnects or sends more data on
                    # this connection, which is dedicated to the response.
                    return
                if reader in ready:
                    events = self.take(reader, writer)
                    if not events:
                        return
                    for event in events:
                        write(event)

    def close(self):
        with self.lock:
            if not self.closed:
                self.closed = True
                for writer in self.subscribers:
                    writer.shutdown(SHUT_WR)
