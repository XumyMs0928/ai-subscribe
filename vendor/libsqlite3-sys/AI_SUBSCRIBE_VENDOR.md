# ai-subscribe SQLite vendor provenance

- Rust wrapper: `libsqlite3-sys 0.38.2`, used by locked `rusqlite 0.40.1`.
- SQLite amalgamation: official `3.53.4`, downloaded from `https://sqlite.org/2026/sqlite-amalgamation-3530400.zip`.
- SQLite source ID: `2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc`.
- Official SHA3-256 for `sqlite3.c`: `67f423e9ebbbdc473cbc4772c872ee6b89f31fde4ed0279a5c25d5f65c043a16`.
- Local SHA-256 for vendored `sqlite3.c`: `b1dd5d74ec7f29055a6684fa06fb3c2f6821c87dd38f9a458dfd2e8a1db28189`.

Only the `sqlite3/` amalgamation files were replaced; the Rust wrapper remains the published 0.38.2 source. This vendor tree is project-local and must not be installed globally.
