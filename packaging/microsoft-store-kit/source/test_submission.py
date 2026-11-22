"""Store kit checks: python3 -m unittest discover -s packaging/microsoft-store-kit/source -p 'test_*.py'

The import checks need only the standard library; rebuilding the import needs Pillow."""
import csv
import json
from pathlib import Path
import struct
import tempfile
import unittest

import build_submission as kit


class DirectoryImportTests(unittest.TestCase):
    """Asset references resolve relative to the folder selected in Partner Center."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name) / 'microsoft-store'
        self.folder.mkdir()
        self.template = [
            ['Field', 'ID', 'Type (Type)', 'default', 'en-us'],
            ['DesktopScreenshot1', '100', 'Relative path (or URL to file in Partner Center)', '', ''],
        ]
        self.csv = self.folder / 'listingData.csv'
        (self.folder / 'shot.png').write_bytes(b'\x89PNG\r\n\x1a\n')
        self.write_reference('microsoft-store/shot.png')

    def write_reference(self, value, default=False):
        with self.csv.open('w', encoding='utf-8-sig', newline='') as stream:
            writer = csv.writer(stream)
            writer.writerow(self.template[0])
            writer.writerow(self.template[1][:3] + ([value, ''] if default else ['', value]))

    def check(self):
        return kit.verify_import(self.folder, self.template, ['en-us'])

    def test_folder_relative_path_and_default_column(self):
        for default in (False, True):
            with self.subTest(default=default):
                self.write_reference('microsoft-store/shot.png', default=default)
                result = self.check()
                self.assertEqual(result['asset_references'], 1)

    def test_rejects_wrong_root_missing_file_and_normalized_paths(self):
        for ref in ['./shot.png', 'microsoft-store/assets/shot.png', 'other-folder/shot.png', 'shot.png', './missing.png']:
            with self.subTest(reference=ref):
                self.write_reference(ref)
                with self.assertRaisesRegex(AssertionError, 'absent from uploaded file list'):
                    self.check()

    def test_a_second_csv_is_not_an_import(self):
        (self.folder / 'source.csv').write_text('original export')
        with self.assertRaisesRegex(AssertionError, 'exactly one CSV'):
            self.check()

    def test_nested_assets_are_rejected(self):
        (self.folder / 'assets').mkdir()
        with self.assertRaisesRegex(AssertionError, 'must be flat'):
            self.check()

    def test_rejects_screenshot_slots_above_ten(self):
        self.template.append(['DesktopScreenshot11', '110', 'Relative path (or URL to file in Partner Center)', '', ''])
        self.write_reference('microsoft-store/shot.png')
        with self.csv.open('a', encoding='utf-8', newline='') as stream:
            csv.writer(stream).writerow(self.template[-1][:3] + ['', 'microsoft-store/shot.png'])
        with self.assertRaisesRegex(AssertionError, 'at most 10'):
            self.check()

    def test_unreferenced_files_are_not_uploaded(self):
        (self.folder / 'extra.png').write_bytes(b'\x89PNG\r\n\x1a\n')
        with self.assertRaisesRegex(AssertionError, 'Unreferenced upload assets'):
            self.check()


def png_size(path):
    data = path.read_bytes()[:24]
    assert data[:8] == b'\x89PNG\r\n\x1a\n', path
    return struct.unpack('>II', data[16:24])


class GeneratedListingTests(unittest.TestCase):
    """The committed import folder matches the copy and passes the import checks."""

    @classmethod
    def setUpClass(cls):
        cls.template = kit.template_rows()
        cls.locales = cls.template[0][4:]
        path = kit.IMPORT / kit.CSV_NAME
        with path.open(encoding='utf-8-sig', newline='') as stream:
            cls.rows = {row[0]: row for row in csv.reader(stream)}
        cls.header = cls.rows['Field']

    def test_one_column_per_language_as_in_the_template(self):
        self.assertEqual(len(self.locales), 38)
        self.assertEqual(self.header, ['Field', 'ID', 'Type (Type)', 'default'] + self.locales)
        for locale in ('en-us', 'en', 'de', 'fr', 'ja', 'zh', 'ar', 'ru-ua', 'sr-latn', 'fil'):
            self.assertIn(locale, self.locales)

    def test_every_language_column_is_filled(self):
        records = {r['locale']: r for r in kit.local_records(self.locales)}
        for field in ('Title', 'ShortDescription', 'Description', 'Feature1', 'Feature7', 'SearchTerm1',
                      'MinimumHardwareReq1', 'RecommendedHardwareReq1', 'ReleaseNotes', 'DesktopScreenshot1',
                      'DesktopScreenshot10', 'StoreLogo300x300', 'PromoImage1920x1080'):
            row = self.rows[field]
            for index, locale in enumerate(self.locales, start=4):
                self.assertTrue(row[index], f'{field} is empty for {locale}')
        for index, locale in enumerate(self.locales, start=4):
            self.assertEqual(self.rows['Title'][index], 'Ascension RTS')
            want = ' '.join(line for line in records[locale]['ShortDescription'].splitlines() if line)
            self.assertEqual(self.rows['ShortDescription'][index], want, locale)
        self.assertEqual(self.rows['DesktopScreenshotCaption1'][self.header.index('en-us')],
                         json.loads((kit.SOURCE / 'screenshots.json').read_text())['screenshots'][0]['caption'])

    def test_lengths_and_search_terms_within_partner_center_limits(self):
        shots = json.loads((kit.SOURCE / 'screenshots.json').read_text())['screenshots']
        for record in kit.local_records(self.locales):
            kit.check_lengths(kit.values(record, shots, kit.IMPORT.name))

    def test_import_folder_passes_the_import_checks(self):
        result = kit.verify_import(kit.IMPORT, self.template, self.locales)
        self.assertEqual(result['asset_references'], 14)

    def test_screenshots_and_logos_have_store_sizes(self):
        for path in kit.IMPORT.glob('*.png'):
            name = path.stem
            if name in kit.ARTWORK:
                self.assertEqual(png_size(path), kit.ARTWORK[name])
            else:
                width, height = png_size(path)
                self.assertEqual(width, kit.SCREEN_WIDTH)
                self.assertGreaterEqual(height, kit.SCREEN_MIN[1])
        for name, size in kit.ARTWORK.items():
            self.assertEqual(png_size(kit.BASE / 'artwork' / f'{name}.png'), size)

    def test_title_is_the_package_display_name(self):
        self.assertEqual(kit.identity()['displayName'], 'Ascension RTS')
        self.assertEqual(kit.identity()['storeId'], kit.STORE_ID)

    def test_text_files_per_locale(self):
        for locale in self.locales:
            folder = kit.BASE / 'locales' / locale
            for name in ('title.txt', 'short-description.txt', 'description.txt', 'features.txt', 'keywords.txt',
                         'minimum-hardware.txt', 'recommended-hardware.txt', 'release-notes.txt', 'listing.json'):
                self.assertTrue((folder / name).is_file(), f'{locale}/{name}')


if __name__ == '__main__':
    unittest.main()
