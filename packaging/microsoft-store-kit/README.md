# Ascension Microsoft Store listing kit

Store ID `9PHHR2KQKVJ2`, package `K3D0.ascension-rts-platform`
(`packaging/msstore-identity.txt`, `packaging/windows/store-identity.json`).

**Import the folder `packaging/microsoft-store/` in Partner Center** (Store listings →
Import listings). It holds one CSV, `listingData-9PHHR2KQKVJ2-ascension.csv`, ten desktop
screenshots and four Store logos. The CSV has one column per language, the same 38 locale
columns as the Partner Center export, every one filled with Ascension's title, short and long
description, features, search terms, hardware notes and release notes; the `default` column
carries the US English copy. Screenshot paths are written as `microsoft-store/<file>.png`,
relative to the selected folder.

**Before importing, compare with a fresh Partner Center export.** The field rows (names, IDs
and types) and the language columns of `source/listingData-9PHHR2KQKVJ2-template.csv` come
from an export of another listing on the same account, with its values cleared. Export
Ascension's listing once from Partner Center; if its rows or columns differ, put the export's
rows in the template (values cleared) and rebuild.

The packages are not part of this kit: upload `ascension-X.Y.Z-windows-x64.msix` and
`ascension-X.Y.Z-windows-arm64.msix` from the GitHub release in the Packages section.

## Contents

- `source/`: the template CSV, the English copy (`en-us.json`, `en.json`), the other 36
  languages (`localized-copy.json`, drafts to be reviewed by native speakers), the
  screenshot list with captions (`screenshots.json`), Microsoft's supported-language codes,
  and the scripts and their tests.
- `locales/<locale>/`: the generated copy of each language as text files, for review.
- `artwork/`: the Store logos and promotional image, SVG and PNG, drawn from the game's
  emblem and HUD colours.
- `documentation/`: submission steps, certification notes, the privacy policy (Markdown and
  HTML), a quick start, the language table and a screenshot gallery.
- `screenshots-manifest.json`, `language-coverage.json`, `validation-report.json`: generated
  records of what was built and checked.

## Rebuild and check

```sh
python3 packaging/microsoft-store-kit/source/render_artwork.py     # needs cairosvg
python3 packaging/microsoft-store-kit/source/build_submission.py   # needs Pillow
python3 -m unittest discover -s packaging/microsoft-store-kit/source -p 'test_*.py'
```

The builder checks every field against Partner Center's limits (lengths, at most seven search
terms and 21 words, at most ten desktop screenshots), the image sizes, and the import folder
itself: exactly one CSV, a flat folder, every referenced file present and every file
referenced. The tests run in the release workflow. Nothing here talks to Partner Center:
import, age rating, pricing and submission are done by hand (`documentation/SUBMISSION.md`).
