import socket
import sys

port = int(sys.argv[1])
soc = socket.socket()
soc.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
soc.bind(("0.0.0.0", port))
soc.listen(4)
print(f"Python Server Listening on port: {port}!")
(client, addr) = soc.accept()
while True:
    message = client.recv(1024)
    print(message)
    if not message:
        break
    client.sendall(message)
