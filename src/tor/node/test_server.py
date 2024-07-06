import socket
import time
import sys

port = int(sys.argv[1])
soc = socket.socket()
soc.bind(("0.0.0.0", port))
soc.listen(4)
print(f"Python Server Listening on port: {port}!")
(client, addr) = soc.accept()
message = client.recv(1024)
print(message)
