"""Carry workspace snapshots and request replies over one WebSocket."""

import json
from socket import SO_KEEPALIVE, SOL_SOCKET

from wsproto import ConnectionType, WSConnection
from wsproto.events import AcceptConnection, BytesMessage, CloseConnection, Ping, TextMessage
from wsproto.utilities import RemoteProtocolError

MAX_MESSAGE = 32_768


class MessageError(Exception):
    def __init__(self, message, data):
        super().__init__(message)
        self.data = data


def reply(handler, actions, message):
    request_id = None
    try:
        request = json.loads(message)
        if not isinstance(request, dict) or type(request.get("id")) is not int:
            raise ValueError("Expected a request with a numeric ID")
        request_id = request["id"]
        action, data = request.get("action"), request.get("data")
        if not isinstance(action, str) or action not in actions or not isinstance(data, dict):
            raise ValueError("Unknown or invalid workspace request")
        return {"id": request_id, "data": actions[action](data)}
    except MessageError as exc:
        return {"id": request_id, "error": str(exc), "data": exc.data}
    except (KeyError, ValueError, TypeError) as exc:
        return {"id": request_id, "error": str(exc)}
    except Exception as exc:
        handler.log_error("Workspace request: %s", exc)
        return {"id": request_id, "error": "Stackroom could not complete the request."}


def serve_websocket(handler, events, actions):
    ws = WSConnection(ConnectionType.SERVER)
    try:
        headers = [(name.encode("ascii"), value.encode("latin-1")) for name, value in handler.headers.raw_items()]
        ws.initiate_upgrade_connection(headers, handler.path)
        response = ws.send(AcceptConnection())
    except (RemoteProtocolError, ValueError):
        handler.json(400, {"error": "Expected a WebSocket upgrade"})
        return

    handler.close_connection = True
    connection = handler.connection
    message, size = [], 0

    def send(event):
        connection.sendall(ws.send(event))

    def receive():
        nonlocal message, size
        data = connection.recv(8192)
        if not data:
            return False
        ws.receive_data(data)
        for event in ws.events():
            if isinstance(event, CloseConnection):
                send(event.response())
                return False
            if isinstance(event, Ping):
                send(event.response())
            if isinstance(event, BytesMessage):
                send(CloseConnection(code=1003, reason="Expected text"))
                return False
            if isinstance(event, TextMessage):
                size += len(event.data)
                if size > MAX_MESSAGE:
                    send(CloseConnection(code=1009, reason="Workspace request too large"))
                    return False
                message.append(event.data)
                if event.message_finished:
                    send(TextMessage(data=json.dumps(reply(handler, actions, "".join(message)))))
                    message, size = [], 0
        return True

    def publish(name, data):
        send(TextMessage(data=f'{{"event":{json.dumps(name)},"data":{data}}}'))

    try:
        connection.setsockopt(SOL_SOCKET, SO_KEEPALIVE, 1)
        connection.sendall(response)
        events.stream_snapshots(connection, publish, receive)
    except (OSError, RemoteProtocolError):
        pass
