# 0.1.3

## Breaking changes

* Duktape reducers now must export their function through `module.export`
  (e5e21640-383a-4e8f-9e98-996f7a20dbe8)

# 0.1.2

## Breaking changes:

* Duktape reducers will no longer be searched under any sub-directory of
  `/path/to/repo/reducers`. Only files directly under it will be considered
  (53fee064-383d-4fbf-a189-40621c95e7b0)

## Bugfixes:

* Fixed handling of Unicode characters (emojis) in some cases (c9175308-8e82-4676-946f-4b84eb61c9ff)
* `sit init` didn't respect `-r/--repository` argument (d23e95ad-1415-482d-b14a-56b0eb6e22fd)

## Improvements

* Performance improvements in record listing (5aec551f-6d77-4da1-a3f8-cf96f13c7c82)
* `sit-web` now accepts `-r/--repository` argument (4daaf255-83f3-4cb2-8c3c-bf70647dbeda)

# 0.1.1

## Breaking changes:

* Some directories inside of SIT repository get renamed: `.web` becomes `web` and
  `.reducers` becomes `reducers` (6a5dfc4a-74f1-4410-b21e-7c60a0939890)

## Improvements:

* Adds (minimally tested) Windows 7 support (e573efdb-ae64-4ad2-bc5b-d9f6786a1a96)
* Improves styling of comments (a1714e8a-7dc7-4332-aee2-941a74b35956)

# 0.1.0

First public release. Hello, world!
