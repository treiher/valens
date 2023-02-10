from queue import Empty, Queue
from threading import Thread
from typing import IO


def wait_for_output(out: IO[bytes], expected: str) -> None:
    def enqueue_output(out: IO[bytes], queue: Queue[bytes]) -> None:
        for line in iter(out.readline, b""):
            queue.put(line)
        out.close()

    q: Queue[bytes] = Queue()
    t = Thread(target=enqueue_output, args=(out, q))
    t.daemon = True
    t.start()

    for _ in range(100):
        try:
            line = q.get(timeout=0.1).decode("utf-8")
        except Empty:
            pass
        else:
            print(line)
            if expected in line:
                break
    else:
        raise RuntimeError("expected output not found")
