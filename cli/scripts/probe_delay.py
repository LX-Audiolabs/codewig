import socket, struct, time
s = socket.create_connection(("127.0.0.1", 9470), timeout=3)
s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
time.sleep(0.1)  # wait for Bitwig to attach receive callback
body = b'{"id":1,"c":"ping"}'
frame = struct.pack(">I", len(body)) + body
s.sendall(frame)
hdr = s.recv(4)
print("hdr", hdr.hex() if hdr else None)
n = struct.unpack(">I", hdr)[0]
print("resp", s.recv(n))
s.close()
