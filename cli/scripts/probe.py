import socket
import struct

s = socket.create_connection(("127.0.0.1", 9470), timeout=3)
s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
body = b'{"id":1,"c":"ping"}'
frame = struct.pack(">I", len(body)) + body
print("send", len(frame), frame)
s.sendall(frame)
hdr = s.recv(4)
print("hdr", hdr.hex() if hdr else None, hdr)
if len(hdr) == 4:
    n = struct.unpack(">I", hdr)[0]
    data = s.recv(n)
    print("resp", data)
s.close()
