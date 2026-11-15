"""Check the installed macOS Homebrew payload and a short terminal session."""

import errno
import os
from pathlib import Path
import platform
import select
import signal
import struct
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from package_release import verify_checksums
from runtime_libraries import clean_environment, verify_runtime


def check_terminal_session(command: Path) -> None:
    # These modules and the controlling-terminal test are macOS-only.
    import fcntl
    import pty
    import termios

    with tempfile.TemporaryDirectory(prefix="cwolf-homebrew-") as temporary:
        root = Path(temporary)
        fixtures = root / "files"
        fixtures.mkdir()
        fixture_name = "homebrew-smoke.txt"
        (fixtures / fixture_name).write_text("Homebrew terminal smoke test.\n", encoding="utf-8")
        home = root / "home"
        home.mkdir()

        environment = clean_environment()
        environment.pop("NO_COLOR", None)
        environment.update(
            HOME=str(home),
            XDG_CONFIG_HOME=str(home / "config"),
            XDG_STATE_HOME=str(home / "state"),
            XDG_CACHE_HOME=str(home / "cache"),
            XDG_DATA_HOME=str(home / "data"),
            TERM="xterm-256color",
            COLORTERM="truecolor",
        )
        arguments = [str(command), "--clean", "--protocol", "hb-tc", str(fixtures)]
        child, terminal = pty.fork()
        if child == 0:
            try:
                fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", 28, 100, 800, 560))
                os.chdir(root)
                os.execve(command, arguments, environment)
            except OSError as error:
                os.write(2, f"Cannot start cwolf: {error}\n".encode())
                os._exit(127)

        reaped = False
        transcript = bytearray()
        try:
            deadline = time.monotonic() + 30
            saw_interface = False
            terminal_open = True
            while time.monotonic() < deadline:
                if terminal_open and select.select([terminal], [], [], 0.1)[0]:
                    try:
                        output = os.read(terminal, 65536)
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        output = b""
                    terminal_open = bool(output)
                    transcript.extend(output)
                    del transcript[:-1048576]
                elif not terminal_open:
                    time.sleep(0.05)

                if not saw_interface and b"NORMAL" in transcript and fixture_name.encode() in transcript:
                    saw_interface = True
                    os.write(terminal, b"q")

                finished, status = os.waitpid(child, os.WNOHANG)
                if finished:
                    reaped = True
                    exit_code = os.waitstatus_to_exitcode(status)
                    if exit_code != 0 or not saw_interface:
                        raise RuntimeError(
                            f"Terminal smoke test failed: exit={exit_code}, interface={saw_interface}"
                        )
                    print("Terminal interface rendered the fixture and exited normally.")
                    return

            raise RuntimeError("Terminal smoke test timed out after 30 seconds")
        except Exception:
            print("Last terminal output:", repr(bytes(transcript[-4096:])), file=sys.stderr)
            raise
        finally:
            if not reaped:
                os.kill(child, signal.SIGKILL)
                os.waitpid(child, 0)
            os.close(terminal)


def main() -> None:
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise RuntimeError("This check requires an Apple Silicon macOS runner")
    if len(sys.argv) != 2:
        raise RuntimeError("Usage: verify_installation.py HOMEBREW_FORMULA_PREFIX")

    prefix = Path(sys.argv[1]).resolve(strict=True)
    payload = prefix / "libexec"
    command = prefix / "bin/cwolf"
    if not command.is_file() or not os.access(command, os.X_OK):
        raise RuntimeError(f"Homebrew command is missing or not executable: {command}")

    verify_checksums(payload)
    verify_runtime(payload, payload / "cwolf")
    print("Release checksums, bundled libraries, signatures, and native loader passed.")
    check_terminal_session(command)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, ValueError) as error:
        print(f"Homebrew verification failed: {error}", file=sys.stderr)
        sys.exit(1)
