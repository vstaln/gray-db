<div align="center">
  <img alt="gray-db" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-db</h1>
  <p><strong>SQLite tools for the agent — query, schema, and table listing over one file.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-db">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install db
```

`gray-db` gives the agent direct, scriptable SQLite access: run a SQL
batch against a file, list its tables, or dump a schema — all through the
sidecar tool protocol.

## Tools

| Tool | What it does |
|---|---|
| `db_query` | Run SQL against a SQLite file. Multiple statements execute in order (`CREATE`/`INSERT`/`UPDATE`/`DELETE` report affected rows; a trailing `SELECT` returns a pipe table). Params: `path`, `sql`. |
| `db_tables` | List tables and row counts. Params: `path`. |
| `db_schema` | Dump `sql` definitions from `sqlite_master`. Params: `path`, optional `table`. |

The SQL runner is quote/comment-aware — semicolons inside string literals
and `--`/`/* */` comments don't split statements. `PRAGMA`-style and
transactional scripts work as written.

## Example

```
db_query  path=./app.db  sql="CREATE TABLE t(a); INSERT INTO t VALUES (1); SELECT * FROM t;"
→ ok — 0 rows affected
  ok — 1 rows affected
  a
  1
  (1 rows)
```

No config, no network, no credentials — it opens the file you point at.

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/db
```

## License

MIT
