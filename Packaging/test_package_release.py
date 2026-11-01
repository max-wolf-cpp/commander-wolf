"""Regression checks for release archive contents and timestamp limits."""

from datetime import datetime, timezone
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch
import zipfile

from package_release import create_archive


class ReleaseArchiveTests(unittest.TestCase):
    def test_zip_bounds_notice_dates_without_changing_contents_or_source_dates(self):
        regular = datetime(2000, 1, 1, 12, tzinfo=timezone.utc).timestamp()
        future = datetime(2200, 1, 1, 12, tzinfo=timezone.utc).timestamp()
        cases = [
            (0, (1980, 1, 1)),
            (regular, time.localtime(regular)[:3]),
            (future, (2107, 12, 31)),
        ]

        for timestamp, expected_date in cases:
            with self.subTest(timestamp=timestamp), tempfile.TemporaryDirectory(prefix="cwolf-zip-test-") as temporary:
                root = Path(temporary)
                package = root / "package"
                package.mkdir()
                notice = package / "LICENSE"
                contents = b"Original notice contents\n"
                notice.write_bytes(contents)
                os.utime(notice, (timestamp, timestamp))
                source_date = notice.stat().st_mtime

                # Exercise the ZIP branch on every native CI platform.
                with patch("package_release.sys.platform", "win32"):
                    archive = create_archive(package, root)

                with zipfile.ZipFile(archive) as stream:
                    self.assertEqual(stream.namelist(), ["package/LICENSE"])
                    self.assertEqual(stream.read("package/LICENSE"), contents)
                    self.assertEqual(stream.getinfo("package/LICENSE").date_time[:3], expected_date)
                self.assertEqual(notice.read_bytes(), contents)
                self.assertEqual(notice.stat().st_mtime, source_date)


if __name__ == "__main__":
    unittest.main()
