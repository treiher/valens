import os

HOST = "127.0.0.1"
PORT = 53535 + int(os.getenv("PYTEST_XDIST_WORKER", "gw0")[2:])
