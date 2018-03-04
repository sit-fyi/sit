# 0.2.0

## Breaking changes

* Some directories inside of SIT repository get renamed: `.web` becomes `web` and
  `.reducers` becomes `reducers` (6a5dfc4a-74f1-4410-b21e-7c60a0939890)
* Duktape reducers will no longer be searched under any sub-directory of
  `/path/to/repo/reducers`. Only files directly under it will be considered
    (53fee064-383d-4fbf-a189-40621c95e7b0)
* Duktape reducers should export their function using `module.export` now (e5e21640-383a-4e8f-9e98-996f7a20dbe8)

## Improvements

* sit: improvements to authorship discovery (efc6b084-db52-4d20-80b9-20112f679660)
* sit: Adds Windows 7 support (e573efdb-ae64-4ad2-bc5b-d9f6786a1a96)
* sit: Duktape reducers support for `require()` (ce9edc69-5b7c-4b3b-8ef0-9cc4ab46faad)
* sit: introduction of Merged record type (c23bdabc-0d25-4019-a7c8-56af4cb1e1ca)
* sit: Performance improvements in record listing (5aec551f-6d77-4da1-a3f8-cf96f13c7c82)
* sit-web: render only issues in the viewport (1fca1d34-7e0f-4a37-adeb-7784961e6135)
* sit-web: issues and comments now include time information (24083d29-bbe1-4067-ac12-fee78ce2ecba)
* sit-web: changed close/reopen buttons to text ones (47fd2dea-e057-4b42-a436-f7971d3d6bcb)
* sit-web: supplying custom repository (4daaf255-83f3-4cb2-8c3c-bf70647dbeda)
* sit-web: introduce loading spinner (7ffa58e7-462a-4f7a-91a0-1c3e742ded82)
* sit-web: add custom overlays (811c3b81-ab6d-4e28-9195-986353cf6e3c)
* sit-web: editing issue title and summary (a9d8e5af-696a-45f4-b7be-1353206c0311)
* sit-web: filter icon changed (f7c0886a-223b-4c44-b1ec-631ad7b42e89)
* sit-web: read-only instances (cac0109e-794f-4c16-9308-86d4d548a5fa)
* sit-web: new issue listing interface (cabb9ba2-e77f-404d-842c-57e269cf3b24)
* sit-web: embed documentation (4ad0f090-ad93-48d6-993b-d8cca001d955)
* sit-web: merge requests in comments styling (26eb23de-6d31-4ddf-990d-eda65a67f933)
* sit-web: improve comment styling (a1714e8a-7dc7-4332-aee2-941a74b35956)

## Bugfixes

* sit: Fixed handling of Unicode characters (emojis) in some cases (c9175308-8e82-4676-946f-4b84eb61c9ff)
* sit: `sit init` didn't respect `-r/--repository` argument (d23e95ad-1415-482d-b14a-56b0eb6e22fd)
* sit: calling .flatten() from itertools fails on nightly (fe5e68e5-22a1-4bc3-8ebf-36586460ba27)
* sit: don't fail if Duktape reducers return something unexpected (ffba2ba6-e4d9-47e0-98aa-9a7fd5412168)
* sit-web: don't fetch fonts from external parties (70d145c6-ee36-4be5-9767-2db4b72b0d94)
* sit-web: disallow empty comments (716979c3-4936-4c72-82d2-f2470e4de411)
* sit-web: handling of empty issue details (9d184a79-763c-4171-9230-cd46a3c2ee01)
* sit-web: complex filters don't work (a77c723b-ef47-495a-8346-ce61885a0687)

# 0.1.0

First public release. Hello, world!
