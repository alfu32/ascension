# Submitting Ascension to the Microsoft Store

Store ID `9PHHR2KQKVJ2`, package identity `K3D0.ascension-rts-platform`, publisher
`CN=7F0CBC95-8035-4AF2-9B17-C38D58351CE5` (`packaging/msstore-identity.txt`).

1. **Packages.** Download `ascension-X.Y.Z-windows-x64.msix` and
   `ascension-X.Y.Z-windows-arm64.msix` from the GitHub release and upload both in the
   submission's Packages section. The Store re-signs them. The package version is the app
   version with 1 added to the major number (app 0.0.113 is package 1.0.113.0; app 0.149.0 is package 1.149.0.0).
2. **Store listings.** In Store listings choose *Import listings* and select the folder
   `packaging/microsoft-store/` (one CSV, ten screenshots, four logos). Before the first
   import, export the listing once from Partner Center and compare its field rows and
   language columns with `source/listingData-9PHHR2KQKVJ2-template.csv`; if they differ,
   replace the template's rows with the export's and run `build_submission.py` again.
3. **Privacy policy.** Enter `https://ascension-euw-1.duckdns.org/privacy`: the game's site
   (`site/`) publishes `PRIVACY.md` there. `documentation/privacy.html` is the same text as a
   single file, for any other host.
4. **Age rating.** Fill in the questionnaire: historical and futuristic combat between small
   figures, no blood; unrestricted text chat with other players only in optional network
   games that players set up themselves.
5. **Notes for certification**: paste `CERTIFICATION.md`.
6. **Product name.** The listing title and the package display name are `Ascension`
   (`packaging/windows/store-identity.json`, `displayName`). They must equal a name
   reserved for this product in Partner Center; if the reserved name differs, change
   `displayName` and the `Title` fields, rebuild, and release again.

Nothing in this kit talks to Partner Center; import and submission are done by hand.
