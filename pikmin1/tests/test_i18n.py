"""Patcher messages in English and French."""

import io
import os
import re
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pikcraft_patcher import cli, i18n  # noqa: E402


class I18nTests(unittest.TestCase):
    def tearDown(self):
        i18n.set_language("en")

    def test_every_message_is_translated_with_the_same_placeholders(self):
        en, fr = i18n.MESSAGES["en"], i18n.MESSAGES["fr"]
        self.assertEqual(set(en), set(fr))
        fields = re.compile(r"\{(\w+)\}|%\((\w+)\)")
        for key in en:
            self.assertEqual(sorted(fields.findall(en[key])), sorted(fields.findall(fr[key])), key)

    def test_detects_french_systems(self):
        saved = {k: os.environ.get(k) for k in ("LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE")}
        try:
            for k in saved:
                os.environ.pop(k, None)
            os.environ["LANG"] = "fr_BE.UTF-8"
            self.assertEqual(i18n.detect(), "fr")
            os.environ["LC_ALL"] = "en_GB.UTF-8"  # LC_ALL wins
            self.assertEqual(i18n.detect(), "en")
        finally:
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v

    def test_numbers(self):
        i18n.set_language("fr")
        self.assertEqual(i18n.number(12.5), "12,5")
        i18n.set_language("en")
        self.assertEqual(i18n.number(12.5), "12.5")

    def test_cli_speaks_french(self):
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            self.assertEqual(cli.main(["--lang", "fr", "/nonexistent/pikmin.iso", "out.iso"]), 1)
        self.assertIn("erreur : /nonexistent/pikmin.iso n'existe pas", err.getvalue())
        with redirect_stdout(out), self.assertRaises(SystemExit):
            cli.main(["--lang", "fr", "--help"])
        self.assertIn("affiche cette aide", out.getvalue())
        self.assertIn("utilisation :", out.getvalue())


if __name__ == "__main__":
    unittest.main()
