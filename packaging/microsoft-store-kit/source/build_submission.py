#!/usr/bin/env python3
"""Build and verify the Partner Center folder import for Ascension (Store ID 9PHHR2KQKVJ2).

    python3 packaging/microsoft-store-kit/source/render_artwork.py     # artwork (cairosvg)
    python3 packaging/microsoft-store-kit/source/build_submission.py   # import folder (Pillow)

Run from any working directory. The template CSV (the field rows of a Partner Center
listing export) sets the fields and the locale columns, one column per language. English
copy is in en.json and en-us.json, the other languages in localized-copy.json. The import
folder `packaging/microsoft-store/` gets the generated CSV, the ten desktop screenshots
(1920 pixels wide, from app/docs/img) and the four Store logos; `locales/<locale>/` gets the
copy as text files for review.
"""
import csv
import hashlib
import html
import json
from pathlib import Path
import re
import shutil

BASE = Path(__file__).resolve().parents[1]
REPO = BASE.parents[1]
SOURCE = BASE / 'source'
STORE_ID = '9PHHR2KQKVJ2'
TEMPLATE = SOURCE / f'listingData-{STORE_ID}-template.csv'
IMPORT = BASE.parent / 'microsoft-store'
CSV_NAME = f'listingData-{STORE_ID}-ascension.csv'
MAX_DESKTOP_SCREENSHOTS = 10
SCREEN_WIDTH = 1920
SCREEN_MIN = (1366, 768)
LOGOS = ['StoreLogo300x300', 'StoreLogo1080x1080', 'StoreLogo720x1080', 'PromoImage1920x1080']
ARTWORK = {'StoreLogo300x300': (300, 300), 'StoreLogo1080x1080': (1080, 1080), 'StoreLogo720x1080': (720, 1080),
           'PromoImage1920x1080': (1920, 1080), 'StoreLogoOverride150x150': (150, 150),
           'StoreLogoOverride71x71': (71, 71)}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def identity():
    return json.loads((REPO / 'packaging/windows/store-identity.json').read_text())


def template_rows():
    with TEMPLATE.open(encoding='utf-8-sig', newline='') as stream:
        return [row for row in csv.reader(stream, strict=True) if any(row)]


def local_records(locales):
    records = json.loads((SOURCE / 'localized-copy.json').read_text(encoding='utf-8'))
    for locale in ('en', 'en-us'):
        records[locale] = json.loads((SOURCE / f'{locale}.json').read_text(encoding='utf-8'))
    title = identity()['displayName']
    for locale in locales:
        assert locale in records, f'Missing listing copy for {locale}'
        record = records[locale]
        assert record['locale'] == locale
        for field in ('Title', 'Description', 'ShortDescription', 'features', 'keywords',
                      'minimum_hardware', 'recommended_hardware', 'ReleaseNotes'):
            assert record.get(field), f'{locale}: missing {field}'
        assert record['Title'] == title, 'Use the reserved product name (store-identity.json displayName)'
    return [records[locale] for locale in locales]


def values(record, shots, folder):
    fields = {k: record.get(k, '') for k in ['Description', 'ReleaseNotes', 'Title', 'ShortTitle', 'SortTitle', 'VoiceTitle',
                                             'ShortDescription', 'DevStudio', 'CopyrightTrademarkInformation',
                                             'AdditionalLicenseTerms']}
    for prefix, key in [('Feature', 'features'), ('SearchTerm', 'keywords'), ('MinimumHardwareReq', 'minimum_hardware'),
                        ('RecommendedHardwareReq', 'recommended_hardware')]:
        for i, text in enumerate(record.get(key, []), 1):
            fields[f'{prefix}{i}'] = text
    for i, shot in enumerate(shots, 1):
        fields[f'DesktopScreenshot{i}'] = f'{folder}/{shot["file"]}'
        # Captions are in English; the other languages show the picture alone.
        if record['locale'] in ('en', 'en-us'):
            fields[f'DesktopScreenshotCaption{i}'] = shot['caption']
    for name in LOGOS:
        fields[name] = f'{folder}/{name}.png'
    return fields


def check_lengths(fields):
    limits = {'Title': 256, 'Description': 10000, 'ReleaseNotes': 1500, 'ShortDescription': 1000, 'ShortTitle': 50,
              'SortTitle': 255, 'VoiceTitle': 255, 'CopyrightTrademarkInformation': 200}
    for field, value in fields.items():
        limit = limits.get(field)
        if field.startswith(('Feature', 'MinimumHardwareReq', 'RecommendedHardwareReq', 'DesktopScreenshotCaption')):
            limit = 200
        if field.startswith('SearchTerm'):
            limit = 40
        if limit is not None:
            assert len(value) <= limit, f'{field}: {len(value)} > {limit}'
    keywords = [v for k, v in fields.items() if k.startswith('SearchTerm')]
    assert len(keywords) <= 7 and sum(len(k.split()) for k in keywords) <= 21, 'At most 7 search terms, 21 words'
    features = [k for k in fields if k.startswith('Feature')]
    assert len(features) <= 20
    assert fields.get('Description') and fields.get('DesktopScreenshot1')


def verify_import(folder, source_rows, locales):
    """The folder Partner Center imports: one CSV, flat, every asset referenced and present."""
    files = list(folder.rglob('*.csv'))
    assert len(files) == 1, 'Import folder must contain exactly one CSV'
    with files[0].open(encoding='utf-8-sig', newline='') as stream:
        rows = list(csv.reader(stream))
    assert [r[:3] for r in rows] == [r[:3] for r in source_rows], 'Field/ID/Type metadata changed'
    assert rows[0] == source_rows[0][:4] + locales
    assert all(path.is_file() for path in folder.iterdir()), 'Import must be flat'
    uploaded = {f'{folder.name}/{path.name}': path for path in folder.iterdir()}
    for row in rows[1:]:
        suffix = row[0].removeprefix('DesktopScreenshot').removeprefix('Caption')
        if row[0].startswith('DesktopScreenshot') and suffix.isdecimal():
            assert int(suffix) <= MAX_DESKTOP_SCREENSHOTS or not any(row[3:]), (
                f'Partner Center supports at most {MAX_DESKTOP_SCREENSHOTS} desktop screenshots: {row[0]}'
            )
    refs = set()
    for row in rows[1:]:
        for value in row[3:]:
            if row[2].startswith('Relative path') and value:
                assert value in uploaded, f'Asset absent from uploaded file list: {value}'
                assert uploaded[value].suffix.lower() == '.png'
                refs.add(value)
    expected = {path.name for path in folder.iterdir() if path.suffix != '.csv'}
    assert {uploaded[ref].name for ref in refs} == expected, 'Unreferenced upload assets'
    return {'folder': folder.name, 'locales': locales, 'csv_rows': len(rows), 'asset_references': len(refs),
            'relative_upload_paths': sorted(uploaded), 'status': 'passed'}


def store_screenshot(source, dest):
    """Scale a capture to 1920 pixels wide, keeping its whole picture (Store minimum 1366 x 768)."""
    from PIL import Image
    with Image.open(source) as image:
        image = image.convert('RGB')
        w, h = image.size
        size = (SCREEN_WIDTH, round(h * SCREEN_WIDTH / w))
        image.resize(size, Image.Resampling.LANCZOS).save(dest, optimize=True)
        return size


def document(title, body):
    return f'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(title)}</title><style>
body{{font:16px/1.65 system-ui,sans-serif;max-width:1100px;margin:40px auto;padding:0 24px;background:#0c1118;color:#e6edf5}}
a{{color:#3fd0ff}}h1,h2{{line-height:1.2;color:#e8c98a}}figure{{margin:25px 0;padding:16px;background:#141b24;border:1px solid #6b4a1f;border-radius:12px}}img{{max-width:100%;height:auto}}
</style>{body}</html>'''


def inline(text):
    """Escapes a line of the policy and renders its **bold** and `code`."""
    text = html.escape(text)
    text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', text)
    return re.sub(r'`(.+?)`', r'<code>\1</code>', text)


def write_privacy():
    """documentation/privacy.html from PRIVACY.md (headings, paragraphs, lists whose items
    wrap over several lines)."""
    privacy = (BASE / 'documentation/PRIVACY.md').read_text()
    body = ''
    for paragraph in privacy.strip().split('\n\n'):
        if paragraph.startswith('# '):
            body += '<h1>' + inline(paragraph[2:]) + '</h1>'
        elif paragraph.startswith('## '):
            body += '<h2>' + inline(paragraph[3:]) + '</h2>'
        elif paragraph.startswith('- '):
            items = []
            for line in paragraph.splitlines():
                if line.startswith('- '):
                    items.append(line[2:])
                else:
                    items[-1] += ' ' + line.strip()
            body += '<ul>' + ''.join('<li>' + inline(item) + '</li>' for item in items) + '</ul>'
        else:
            body += '<p>' + inline(' '.join(paragraph.split())) + '</p>'
    (BASE / 'documentation/privacy.html').write_text(document('Ascension Privacy Policy', body), encoding='utf-8')


def main():
    from PIL import Image
    shots = json.loads((SOURCE / 'screenshots.json').read_text())['screenshots']
    assert 1 <= len(shots) <= MAX_DESKTOP_SCREENSHOTS and len({s['file'] for s in shots}) == len(shots)
    template = template_rows()
    locales = template[0][4:]
    records = local_records(locales)
    default_locale_index = template[0].index('en-us')
    for name, size in ARTWORK.items():
        with Image.open(BASE / 'artwork' / f'{name}.png') as image:
            assert image.size == size and image.format == 'PNG', name
    if IMPORT.exists():
        shutil.rmtree(IMPORT)
    IMPORT.mkdir(parents=True)
    manifest = []
    for slot, shot in enumerate(shots, 1):
        source = REPO / shot['source']
        dest = IMPORT / shot['file']
        width, height = store_screenshot(source, dest)
        with Image.open(dest) as image:
            assert image.format == 'PNG' and image.size == (width, height)
            assert width >= SCREEN_MIN[0] and height >= SCREEN_MIN[1], shot['file']
        assert dest.stat().st_size < 50 * 1024 * 1024
        manifest.append({'slot': slot, 'file': shot['file'], 'source': shot['source'], 'source_sha256': digest(source),
                         'width': width, 'height': height, 'sha256': digest(dest), 'caption': shot['caption'],
                         'interface_language': 'en-us', 'capture_platform': 'Linux'})
    for name in LOGOS:
        shutil.copy2(BASE / 'artwork' / f'{name}.png', IMPORT / f'{name}.png')
    localized = {r['locale']: values(r, shots, IMPORT.name) for r in records}
    for v in localized.values():
        check_lengths(v)
    with (IMPORT / CSV_NAME).open('w', encoding='utf-8-sig', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(template[0])
        for row in template[1:]:
            cells = row[:4]
            for index, locale in enumerate(locales, start=4):
                value = row[index] or localized[locale].get(row[0], '')
                # One line per cell, as in the Partner Center export.
                cells.append(' '.join(line for line in value.splitlines() if line))
            # Keep explicit defaults from the export; US English is the fallback.
            if not cells[3]:
                cells[3] = cells[default_locale_index]
            writer.writerow(cells)
    result = verify_import(IMPORT, template, locales)
    for record in records:
        dest = BASE / 'locales' / record['locale']
        if dest.exists():
            shutil.rmtree(dest)
        dest.mkdir(parents=True)
        write_json(dest / 'listing.json', localized[record['locale']])
        for key, name in [('Title', 'title.txt'), ('Description', 'description.txt'),
                          ('ShortDescription', 'short-description.txt'), ('ReleaseNotes', 'release-notes.txt')]:
            (dest / name).write_text(record[key] + '\n', encoding='utf-8')
        for key in ['features', 'keywords', 'minimum_hardware', 'recommended_hardware']:
            (dest / (key.replace('_', '-') + '.txt')).write_text('\n'.join(record[key]) + '\n', encoding='utf-8')
    body = (f'<h1>Ascension Store screenshots</h1><p>The {len(shots)} desktop screenshots of the import, '
            'in listing order. English interface, captured on Linux.</p>')
    for item in manifest:
        body += (f'<figure><img loading="lazy" src="../../microsoft-store/{html.escape(item["file"])}" '
                 f'alt="{html.escape(item["caption"])}"><figcaption><strong>{item["slot"]}. {html.escape(item["caption"])}'
                 f'</strong><br><code>{item["file"]}</code> from <code>{item["source"]}</code></figcaption></figure>')
    (BASE / 'documentation/screenshots.html').write_text(document('Ascension screenshots', body), encoding='utf-8')
    write_privacy()
    write_json(BASE / 'screenshots-manifest.json', manifest)
    coverage = [{key: record[key] for key in ('locale', 'language', 'translation_status')} for record in records]
    write_json(BASE / 'language-coverage.json', coverage)
    rows = ['# Listing-language coverage', '',
            f'The listing fills all {len(records)} locale columns of the template, one column per language.',
            'Every column carries the title, short and long description, features, search terms, hardware notes',
            'and release notes. Screenshot captions are in English. The game interface is in English.', '',
            '| Locale | Language | Copy |', '| --- | --- | --- |']
    rows += [f"| {r['locale']} | {r['language']} | {r['translation_status']} |" for r in coverage]
    (BASE / 'documentation/LANGUAGES.md').write_text('\n'.join(rows) + '\n', encoding='utf-8')
    write_json(BASE / 'validation-report.json', {
        'status': 'passed',
        'scope': 'local checks only; not Partner Center acceptance or Microsoft certification',
        'store_id': STORE_ID, 'import': result, 'screenshots': len(shots), 'locales': len(locales),
        'unresolved': ['Have a native speaker review the draft translations',
                       'Publish the privacy policy at a public URL and enter it in Partner Center',
                       'Compare the template rows with a fresh Partner Center export before importing',
                       'Complete age ratings, pricing and the other account-level fields in Partner Center',
                       'Upload the MSIX packages from the GitHub release in the Packages section']})
    print(json.dumps({'import': result['folder'], 'csv_rows': result['csv_rows'],
                      'asset_references': result['asset_references'], 'locales': len(locales)}, indent=2))


if __name__ == '__main__':
    main()
