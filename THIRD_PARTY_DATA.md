# Third-Party Data Licenses and Attribution

This file tracks third-party lexical data used by the project pipeline.

## Why this file exists

The repository code is AGPL-3.0-or-later, but source datasets are licensed separately.
If a built artifact contains derived data from a third-party corpus, that artifact may carry additional obligations.

## Source 1: Polish Wiktionary Dump

- Name: Polish Wiktionary
- File pattern: plwiktionary-latest-pages-articles.xml.bz2
- Typical source URL: https://dumps.wikimedia.org/plwiktionary/
- License family: Wikimedia/Wiktionary content licenses (verify exact current terms when downloading)
- Expected obligations:
  - Attribution
  - Share-alike or copyleft terms for adapted content (depending on current dump terms)

Contributor requirements:
- Record the exact dump URL and retrieval date in PR notes.
- Preserve attribution in distributed artifacts.
- Do not claim Wiktionary data is re-licensed as AGPL.

## Source 2: PoliMorf

- Name: PoliMorf morphological dictionary
- File pattern: PoliMorf-*.tab.gz
- Local source path in this repository: sources/PoliMorf-0.6.7.tab.gz
- License note from upstream source page: BSD-2-Clause
- Required verification status: confirm the authoritative license text for the exact downloaded version before external release

Requested citation (from PoliMorf authors):

Marcin Wolinski, Marcin Milkowski, Maciej Ogrodniczuk, Adam Przepiorkowski,
and Lukasz Szalkiewicz. PoliMorf: A (not so) new open morphological dictionary
for Polish. In Proceedings of the Eighth International Conference on Language
Resources and Evaluation, LREC 2012, pages 860-864, Istanbul, Turkey, 2012.
European Language Resources Association (ELRA).

Contributor requirements:
- Add the authoritative license reference (URL or bundled license text) for the exact PoliMorf version used.
- If terms are missing or unclear, do not publish data-derived artifacts built from PoliMorf.
- Preserve all required attribution and notices.

## Practical Policy in This Repository

1. Keep third-party data provenance documented.
2. Keep code licensing and data licensing clearly separated.
3. Treat generated dictionary files as potentially license-encumbered by source data.
4. Require attribution and any downstream conditions when redistributing generated artifacts.

## Maintainer Review Gate for Data PRs

A data-affecting PR should include:
- Source URLs
- Version identifiers
- Retrieval dates
- License references
- Attribution text to be shipped
- Confirmation that redistribution terms are met

If any item above is missing, merge should be blocked until completed.

## Legal Note

This document is an engineering compliance checklist, not legal advice.
For legal interpretation, consult qualified counsel.
