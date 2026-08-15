from collections.abc import Generator, Mapping
from contextlib import contextmanager
from queue import Empty, Queue
from subprocess import PIPE, STDOUT, Popen, TimeoutExpired
from threading import Thread
from time import monotonic
from typing import IO

POLL_INTERVAL = 0.1


@contextmanager
def run_server(command: str, env: Mapping[str, str]) -> Generator[None, None, None]:
    """Run a server until the end of the context, terminating it in case of failure as well."""

    with Popen(command.split(), stdout=PIPE, stderr=STDOUT, env=env) as p:
        try:
            wait_for_output(p, "Running on")
            yield
        finally:
            p.terminate()
            try:
                p.wait(timeout=10)
            except TimeoutExpired:
                p.kill()


def wait_for_output(p: Popen[bytes], expected: str, timeout: float = 30) -> None:
    def enqueue_output(out: IO[bytes], queue: Queue[bytes]) -> None:
        for line in iter(out.readline, b""):
            queue.put(line)
        out.close()

    assert p.stdout

    q: Queue[bytes] = Queue()
    t = Thread(target=enqueue_output, args=(p.stdout, q))
    t.daemon = True
    t.start()

    lines = []
    deadline = monotonic() + timeout

    while (remaining := deadline - monotonic()) > 0:
        try:
            line = q.get(timeout=min(remaining, POLL_INTERVAL)).decode("utf-8")
        except Empty:
            # The reader thread ends after the last output was read, so the queue is drained.
            if p.poll() is not None and not t.is_alive():
                break
            continue
        lines.append(line)
        if expected in line:
            return

    reason = (
        f"within {timeout} s"
        if (returncode := p.poll()) is None
        else f"before the server exited with status {returncode}"
    )
    output = "".join(lines) or "<no output>"
    raise RuntimeError(f'Expected output "{expected}" not found {reason} in:\n{output}')
