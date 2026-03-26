# Changelog

## [0.1.4](https://github.com/raiderrobert/ownrs/compare/v0.1.3...v0.1.4) (2026-03-26)


### Features

* refine --suggest modes and add README docs ([#18](https://github.com/raiderrobert/ownrs/issues/18)) ([080b4dd](https://github.com/raiderrobert/ownrs/commit/080b4ddddbecf24391103e8f57ebfbc386c1b918))

## [0.1.3](https://github.com/raiderrobert/ownrs/compare/v0.1.2...v0.1.3) (2026-03-26)


### Features

* add --max-team-size and --exclude-team filters for suggestions ([873f53d](https://github.com/raiderrobert/ownrs/commit/873f53d8e316ad12aedf9a97a24511f8e4b5137a))
* add --suggest and --lookback-days CLI flags ([6284ced](https://github.com/raiderrobert/ownrs/commit/6284cedc7d5d5929415bc9de1d4d57111d2b2588))
* add --version flag to CLI ([1d2095e](https://github.com/raiderrobert/ownrs/commit/1d2095e8af4f33b679e9d5cf76e129b52862370f))
* add --version flag to CLI ([f17b11e](https://github.com/raiderrobert/ownrs/commit/f17b11e8a35584ea8d697e3d3318f86aa854829a))
* add commit and PR review activity fetching ([2490092](https://github.com/raiderrobert/ownrs/commit/2490092dfa483f5f08e3a2dcbe364ea27302fd29))
* add suggested owners to CSV output ([99f68dd](https://github.com/raiderrobert/ownrs/commit/99f68ddf33ed710f81cb23d72deff1ae216830cf))
* add suggestion types for ownership heuristic ([0aa391e](https://github.com/raiderrobert/ownrs/commit/0aa391e2767482905b473f3f5558f394bf087738))
* add team membership fetching with caching ([54ad63f](https://github.com/raiderrobert/ownrs/commit/54ad63f905104b13f96499100e75ea8599fa2c5a))
* add team scoring engine for ownership suggestions ([98e53c4](https://github.com/raiderrobert/ownrs/commit/98e53c497c0773754d40eaee323f6f23a60b3c6e))
* display suggested owners in single-repo table output ([90cfd61](https://github.com/raiderrobert/ownrs/commit/90cfd61d12eac15bb30bdb74657434dbd992e6a4))
* make --suggest accept mode (missing, stale, partial) with auto-trigger default ([6a61c94](https://github.com/raiderrobert/ownrs/commit/6a61c947d58eace463ee722376644780966ffb31))
* suggest likely owners for orphaned repos ([82efed6](https://github.com/raiderrobert/ownrs/commit/82efed6e6df1d46389aafce8409ef58a3d73e73e))
* wire ownership heuristic into run_repo ([e8636a5](https://github.com/raiderrobert/ownrs/commit/e8636a567d00e6a694f10149131530d5d0ca42f9))


### Bug Fixes

* format code for rustfmt ([4cf426b](https://github.com/raiderrobert/ownrs/commit/4cf426b13c9b7a21130af872b7a773b365b93f33))
* format code for rustfmt ([9f0067c](https://github.com/raiderrobert/ownrs/commit/9f0067cd3b98d2ace4a97bda4a4ae06e9a6469e6))
* group suggestion flags under help heading ([8021f64](https://github.com/raiderrobert/ownrs/commit/8021f64c803ceea1981f5e8ef320bc56d8ba7c9e))
* improve error reporting, cache keys, and PR pagination ([3e4ddab](https://github.com/raiderrobert/ownrs/commit/3e4ddab74b96aceb9efd7972c74735563ed1dd45))
* simplify team members cache key ([eed68ba](https://github.com/raiderrobert/ownrs/commit/eed68baf8aaea105fe513164add2c9d971c7765d))

## [0.1.2](https://github.com/raiderrobert/ownrs/compare/v0.1.1...v0.1.2) (2026-03-26)


### Features

* auto-detect GitHub token from gh CLI ([5371268](https://github.com/raiderrobert/ownrs/commit/53712688922a3355a6a9763c781d3ecc013e5a49))
* auto-detect GitHub token from gh CLI and hide token in help ([a304ba6](https://github.com/raiderrobert/ownrs/commit/a304ba6ad130587cb65c7d294c87b31b8ff62a0e))


### Bug Fixes

* format bail! to satisfy rustfmt ([a05ba31](https://github.com/raiderrobert/ownrs/commit/a05ba315c4f1062cd752a04691974e6e4218e4d4))
* format remaining bail! call for rustfmt ([0886cda](https://github.com/raiderrobert/ownrs/commit/0886cda471d44c810008a402b6ec6cc5ee72a6ab))

## [0.1.1](https://github.com/raiderrobert/ownrs/compare/v0.1.0...v0.1.1) (2026-03-18)


### Features

* add --strict flag to CLI and config ([998cddc](https://github.com/raiderrobert/ownrs/commit/998cddc754f4479dbf96e229b7a54c99de49d55a))
* add AdminOnly status, multi-team fields to RepoOwnership ([779e9c6](https://github.com/raiderrobert/ownrs/commit/779e9c66b475eaa652b42a8594bdf6463a7488eb))
* add install script and improve README ([528622f](https://github.com/raiderrobert/ownrs/commit/528622fbdf3c41e01f20e52b2e3037856af994a4))
* add install.sh and improve README ([0a9b50a](https://github.com/raiderrobert/ownrs/commit/0a9b50afc4a6f090657c51c5cdedef48e2047df4))
* add per-repo admin team fetcher with caching ([02e7b3b](https://github.com/raiderrobert/ownrs/commit/02e7b3beb2a1ddb5d321f280387e18e98f192233))
* add per-repo admin teams as third ownership signal ([c097a8f](https://github.com/raiderrobert/ownrs/commit/c097a8fe742c62b3f38e1074ad19086f67842f18))
* add percentage column to summary table ([2c19122](https://github.com/raiderrobert/ownrs/commit/2c1912250a5185ee196d1c4e8b187ac9cd5899a2))
* add README, restore LICENSE and future use cases ([7828a52](https://github.com/raiderrobert/ownrs/commit/7828a52af43a4bfc9eef03b98e70ddbe764b68d2))
* add release-please with macOS build pipeline ([3a3d213](https://github.com/raiderrobert/ownrs/commit/3a3d2137f7244a6eca1d08be09beea21f5a06cde))
* add release-please with macOS build pipeline ([e8bc73d](https://github.com/raiderrobert/ownrs/commit/e8bc73d770e19dbc958c850b1f55356b5eda494c))
* add Required Notice and Licensor Line of Business to LICENSE ([42d3d30](https://github.com/raiderrobert/ownrs/commit/42d3d30aaa54e46031b6a358363ee81ded2716be))
* add spinners and live progress for network fetches ([dfa3723](https://github.com/raiderrobert/ownrs/commit/dfa37235d717e77909d8c99aac8b9eda4ced038d))
* cache repo list with same 24h TTL ([9c949bc](https://github.com/raiderrobert/ownrs/commit/9c949bce3b3e9122aee5ab7d90007135bbd64142))
* extract_teams returns all CODEOWNERS teams with dedup ([d6bdc66](https://github.com/raiderrobert/ownrs/commit/d6bdc66fc902b3b75733eb8e32195934b7545b58))
* fetch admin teams alongside source files in fetcher ([31aabe7](https://github.com/raiderrobert/ownrs/commit/31aabe71408ee3f6e877b2bcb5ecbb22795f5a67))
* finalize v0 plan with BDD use cases, CLI design, and CI lifecycle ([570af64](https://github.com/raiderrobert/ownrs/commit/570af642092d17805ea9e397ee72177f0a74ba17))
* implement v0 CLI with full ownership reconciliation ([1511004](https://github.com/raiderrobert/ownrs/commit/1511004600e25d3f240b66cdc4e7695425be0d49))
* implement v0 ownership reconciliation CLI ([2bb8b75](https://github.com/raiderrobert/ownrs/commit/2bb8b7542e0978096b316a529a698c2a7e4b1fea))
* initial project scaffold with implementation plan ([e2986ac](https://github.com/raiderrobert/ownrs/commit/e2986acba77020a59ca597db60ee796eb1b2a457))
* three-source reconciliation with intersection and strict modes ([216589c](https://github.com/raiderrobert/ownrs/commit/216589ce2668c6b2ceb639b67d416a0c1a165c53))


### Bug Fixes

* apply cargo fmt formatting ([22377ba](https://github.com/raiderrobert/ownrs/commit/22377ba24ce1d98da7b079e26e9a19dc41ecedaf))
* broaden Licensor Line of Business scope ([eff89f3](https://github.com/raiderrobert/ownrs/commit/eff89f3b8e811cf6f76dd5c0559caef3fc67c5d3))
* convert LICENSE to plain text and add copyright notice ([8d166d3](https://github.com/raiderrobert/ownrs/commit/8d166d3789dd6dcd80004917ad5aa1d0ffe5c9d2))
* remove unused methods to silence warnings ([bee002a](https://github.com/raiderrobert/ownrs/commit/bee002ac9f3f1479a3c0dfd7274a879305428094))


### Miscellaneous

* remove implementation plans from repo ([be610a5](https://github.com/raiderrobert/ownrs/commit/be610a5b95796e6ffd2250a7a015c5de1d7fcdbc))
* remove superpowers docs from repo ([2a342d3](https://github.com/raiderrobert/ownrs/commit/2a342d30f7a5849c182d7641ef7d3678d7dc6d42))
* remove unused extract_team wrapper and repo_teams module ([e403c50](https://github.com/raiderrobert/ownrs/commit/e403c50f463551eac891314eb81c8343c1fd4745))
* update license required notice URL to personal site ([f46602f](https://github.com/raiderrobert/ownrs/commit/f46602fa57e698bf6b839dd50b73107caf254918))


### Documentation

* add admin teams design spec from diverge-critique-converge bakeoff ([4262dbd](https://github.com/raiderrobert/ownrs/commit/4262dbd27eba49dfce0d9589ecac5ef1c143a51d))
