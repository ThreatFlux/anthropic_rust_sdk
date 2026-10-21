# Stable dependency and security rollup

Reviewed 2026-10-21 against fetched main `ef64e62af1e7c8f5fbb891d6a4ec989f17ee37a2`.

## Dependencies and compatibility

All direct/dev crates were checked against the [crates.io sparse index](https://index.crates.io/config.json), excluding yanked/prerelease versions. Updated Tokio, serde_json and UUID; all others already named current stable releases. Builds now use stable Rust **1.99.1**, verified against the [official channel manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml). Consumer MSRV remains **1.99.0** with an independent CI check. The documentation checker now permits a newer build toolchain and checks the Docker builder against that toolchain. Positive and two negative controls (below-MSRV toolchain, mismatched builder) passed.

| Crate | Verified latest stable |
| --- | --- |
| anyhow | 1.0.104 |
| base64 | 0.23.1 |
| chrono | 0.4.45 |
| dotenvy | 0.15.7 |
| futures | 0.3.34 |
| futures-util | 0.3.34 |
| governor | 0.10.4 |
| http | 1.5.0 |
| mime | 0.3.17 |
| mime_guess | 2.0.5 |
| mockito | 1.7.2 |
| nonzero_ext | 0.3.0 |
| pretty_assertions | 1.4.1 |
| reqwest | 0.13.5 |
| serde | 1.0.229 |
| serde_json | 1.0.154 |
| tempfile | 3.27.0 |
| thiserror | 2.0.21 |
| tokio | 1.54.0 |
| tokio-stream | 0.1.19 |
| tokio-test | 0.4.6 |
| tracing | 0.1.44 |
| tracing-subscriber | 0.3.23 |
| url | 2.5.8 |
| uuid | 1.28.0 |
| wiremock | 0.6.5 |

Actions were checked against authoritative stable tags. No Action version bump was needed. The Rust builder advances to 1.99.1 with its current immutable index digest. This library intentionally does not track Cargo.lock; a fresh resolution was tested and scanned locally, while release workflows continue resolving and passing one graph to release assets.

## Alert inventory and runtime change

Paginated GitHub open-PR and open-alert inventories were empty. Existing Trivy analyses returned zero results because reporting was limited to fixable HIGH/CRITICAL issues. No alerts were dismissed or old PRs closed. New reporting includes all severities and unfixed findings. An empty filtered report is not proof of a safe image.

An independent all-severity scan of the **old immutable runtime base**, not the complete old application image, found 32 package records. Those records are assessed below; they are not fabricated GitHub alert IDs.

The helper runtime now uses the [official minimal GNU image](https://images.chainguard.dev/directory/image/glibc-dynamic/overview), pinned to `sha256:3f1283dc2a6a48c25cd2e1c7730a871b19b721975240e108e0d6a73b555a3a20`. This retains glibc/GNU ABI, CA roots, nonroot UID65532, tini, and both helper binaries. Exported ARM64 package inventory has glibc/loader/locale **2.44-r8**, libgcc/libstdc++ **16.2.0-r1**, CA bundle **20260909-r3**, and Wolfi baselayout (7 packages).

The new opt-in `vendored-native-tls` feature statically builds OpenSSL **3.6.3** via openssl-src300.6.1+3.6.3 for the container. Existing consumer platform-native TLS and Rustls choices remain available. The final filesystem has no libssl/libcrypto shared library and both binaries start successfully. No musl migration or disabled TLS functionality is used.

## Original-base finding decisions

TP means the affected component and package version are present; application reachability is a separate question. FP requires a component/architecture mismatch. No unproven finding is dismissed. Source-package version fanout alone does not establish that every binary package contains a bug.

| Finding | Assessment and reason | Implemented change / remaining plan |
| --- | --- | --- |
| [DSA-6572-1](https://security-tracker.debian.org/tracker/DSA-6572-1), ca-certificates | TP stale certificate package advisory; trust-store changes matter independently of SDK exploitability. | Replaced with current vendor CA bundle; retain TLS/certificate regression checks. |
| [CVE-2026-102010](https://security-tracker.debian.org/tracker/CVE-2026-102010), gcc-14-base | FP component mapping: findings concern libstdc++ aligned new or PBDS template header, not this binary component. | Old packages removed by runtime replacement; retain exact-component review for future compiler libraries. |
| [CVE-2026-95619](https://security-tracker.debian.org/tracker/CVE-2026-95619), gcc-14-base | FP component mapping: findings concern libstdc++ aligned new or PBDS template header, not this binary component. | Old packages removed by runtime replacement; retain exact-component review for future compiler libraries. |
| [CVE-2026-18374](https://security-tracker.debian.org/tracker/CVE-2026-18374), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-19499](https://security-tracker.debian.org/tracker/CVE-2026-19499), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-19542](https://security-tracker.debian.org/tracker/CVE-2026-19542), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-5435](https://security-tracker.debian.org/tracker/CVE-2026-5435), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-6238](https://security-tracker.debian.org/tracker/CVE-2026-6238), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-6368](https://security-tracker.debian.org/tracker/CVE-2026-6368), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-6791](https://security-tracker.debian.org/tracker/CVE-2026-6791), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-77117](https://security-tracker.debian.org/tracker/CVE-2026-77117), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-80489](https://security-tracker.debian.org/tracker/CVE-2026-80489), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-8674](https://security-tracker.debian.org/tracker/CVE-2026-8674), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-86805](https://security-tracker.debian.org/tracker/CVE-2026-86805), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2026-89092](https://security-tracker.debian.org/tracker/CVE-2026-89092), libc6 | FP component mapping for runtime without nscd. A source-package match does not install the separate daemon. | Both base layouts omit nscd; recheck daemon/socket absence in future deployments. |
| [CVE-2026-95818](https://security-tracker.debian.org/tracker/CVE-2026-95818), libc6 | TP vulnerable Debian library code; DNS search-domain initialization is plausibly reachable through reqwest. Other API-specific triggers are not proven SDK paths. | Replaced with patched2.44-r8 GNU runtime; modern fix source evidence below. Validate DNS/native-TLS and upstream regressions on each release architecture. |
| [CVE-2010-4756](https://security-tracker.debian.org/tracker/CVE-2010-4756), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2018-20796](https://security-tracker.debian.org/tracker/CVE-2018-20796), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2019-1010022](https://security-tracker.debian.org/tracker/CVE-2019-1010022), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2019-1010023](https://security-tracker.debian.org/tracker/CVE-2019-1010023), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2019-1010024](https://security-tracker.debian.org/tracker/CVE-2019-1010024), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2019-1010025](https://security-tracker.debian.org/tracker/CVE-2019-1010025), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2019-9192](https://security-tracker.debian.org/tracker/CVE-2019-9192), libc6 | TP/disputed source behavior in Debian; no demonstrated SDK exploit chain. Regex/glob/ldd paths and prerequisite memory corruption must be distinguished from thread/loader use. | No historical source fix is claimed from vendor scan0. Preserve literal paths/no untrusted ELF execution; bound untrusted processing. Review native imports and investigate any concrete leak/corruption/regex chain before claiming inapplicability. |
| [CVE-2026-97399](https://security-tracker.debian.org/tracker/CVE-2026-97399), libc6 | FP on supported amd64/arm64: vulnerable optimized implementation requires Power8. | Reassess if PowerPC is added; do not use a global architecture-independent suppression. |
| [CVE-2026-102010](https://security-tracker.debian.org/tracker/CVE-2026-102010), libgcc-s1 | FP component mapping: findings concern libstdc++ aligned new or PBDS template header, not this binary component. | Old packages removed by runtime replacement; retain exact-component review for future compiler libraries. |
| [CVE-2026-95619](https://security-tracker.debian.org/tracker/CVE-2026-95619), libgcc-s1 | FP component mapping: findings concern libstdc++ aligned new or PBDS template header, not this binary component. | Old packages removed by runtime replacement; retain exact-component review for future compiler libraries. |
| [CVE-2026-102010](https://security-tracker.debian.org/tracker/CVE-2026-102010), libgomp1 | FP component mapping: findings concern libstdc++ aligned new or PBDS template header, not this binary component. | Old packages removed by runtime replacement; retain exact-component review for future compiler libraries. |
| [CVE-2026-95619](https://security-tracker.debian.org/tracker/CVE-2026-95619), libgomp1 | FP component mapping: findings concern libstdc++ aligned new or PBDS template header, not this binary component. | Old packages removed by runtime replacement; retain exact-component review for future compiler libraries. |
| [CVE-2026-102010](https://security-tracker.debian.org/tracker/CVE-2026-102010), libstdc++6 | FP runtime-component mapping: PBDS fix is in a development template header, absent from runtime; precompiled consumers need independent review. | No C++ headers shipped; keep compiler-derived consumer exposure separate from shared-library version scans. |
| [CVE-2026-95619](https://security-tracker.debian.org/tracker/CVE-2026-95619), libstdc++6 | Undetermined for old application artifact: aligned-new build path needs disassembly; source-package match alone is insufficient. | Replaced GCC14 runtime with current vendor GCC16 runtime; verify affected aligned allocation path if evaluating original artifact. No old alert dismissal. |
| [CVE-2026-27171](https://security-tracker.debian.org/tracker/CVE-2026-27171), zlib1g | TP zlib1.3.1 CRC combine negative-length loop; application reachability unproven. | System libz removed; final package scan and native-library closure checked. For any retained library run bounded negative-length combine tests. |
| [CVE-2026-85091](https://security-tracker.debian.org/tracker/CVE-2026-85091), zlib1g | Undetermined version-range disagreement: shipped zlib1.3.1 precedes nonblocking writer machinery, but Debian labels it affected. | Runtime replacement removes system zlib; no source-level all-zlib safety claim. Verify transitive static compression dependencies with Cargo audit. |

Modern glibc fixes are checked separately from vendor database coverage. The [exact official Wolfi recipe](https://github.com/wolfi-dev/os/blob/b71c93b89f2aa676401abde131f262747739e740/glibc-2.44.yaml) pins source34106fee698c83eb0af71d4d0381e66b399be746. Against that exact extracted source, repeated upstream patch reverse dry-run checks for CVE-2026-19542,5435,6238,77117,80489,8674 all passed with no fuzz/offset. Source inspection also confirmed growable tilde buffers, empty-ccs guard and bounded strfmon written-width handling. [Independent source provenance and detailed fix evidence](https://github.com/ThreatFlux/file-scanner/blob/chore/consolidate-stable-dependencies-20261021/docs/security/glibc-source-fix-verification-2026-10-21.md) records upstream patch links and exact file hashes. This review did not build glibc or run its full upstream tests. Loader security-context bugs and historical vendor-disputed behaviors require their own trigger review; scan0 alone does not prove them fixed.

## Validation and limits

Built the actual ARM64 release helper image, ran the repository's image smoke script (usage helper, missing-key exit1, UID65532), exported its filesystem and scanned it with Trivy0.76.0 using the current shared vulnerability database, all severities and unfixed packages. Result: **0 package findings across7 OS packages**. The image scanner detected **no Rust package inventory**; a separate Trivy scan of the resolved Cargo.lock reported0 findings, and the full local gate includes RustSec audit. These are separate checks, not a claim of an embedded production SBOM.

Final ELF imports are only libc/libm/libgcc (usage helper: libc/libgcc; tini: libc), and the native filesystem contains no shared OpenSSL library or setuid/setgid file. The committed [resolver probe](../../scripts/fixtures/resolver_init_probe.c) was run network-disabled, read-only and nonroot against three upstream-style LOCALDOMAIN inputs: a244-character entry after example.com, a256-character entry before example.com, and a256-character entry alone. All three exit0 on the actual final image and abort133 on the old pinned runtime with the exact vulnerable resolv_conf_matches assertion. [Results](dns-regression-evidence-2026-10-21.json) record both positive and negative controls. No real API credentials or live billable API calls were used. The smoke test verifies startup and dependency resolution, not a complete live native-TLS transaction. ARM64 was built locally; AMD64 runtime/build behavior remains a hosted release check. Historical recursion/ASLR concerns remain explicit limitations. No source findings, alerts, ignores, Jira tickets, merges or deployments were manufactured.

The complete repository `make ci` passed, including all-feature tests (348 unit-suite tests and24 transport-suite tests), doctests, Clippy, build/examples/rustdoc, documentation contract, MSRV1.99.0, RustSec audit and license policy. Final all-target/all-feature Clippy, three toolchain-contract regression tests and actionlint passed as well.
