//! db — SQLite tools: db_query, db_tables, db_schema.

use std::io::{BufRead, Write};
use rusqlite::Connection;
use serde_json::{json, Value};

const NAME: &str = "db";
const MAX_ROWS: usize = 200;

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "DB",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "db_query", "description": "Run SQL against a SQLite file. SELECT returns a pipe table (capped at 200 rows); writes return rows affected.",
             "parameters": {"type": "object", "properties": {
                "path": {"type": "string", "description": "SQLite file path."},
                "sql": {"type": "string"},
                "max_rows": {"type": "integer"}}, "required": ["path","sql"]}},
            {"name": "db_tables", "description": "List tables in a SQLite file.",
             "parameters": {"type": "object", "properties": {
                "path": {"type": "string"}}, "required": ["path"]}},
            {"name": "db_schema", "description": "Show CREATE SQL for one table or all.",
             "parameters": {"type": "object", "properties": {
                "path": {"type": "string"}, "table": {"type": "string"}},
             "required": ["path"]}},
        ],
        "commands": ["/db"], "subcommands": ["db"],
    })
}

/// Split a script on `;` outside quotes/comments. Tolerant by design —
/// a weird tail just fails at prepare with a statement number.
fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' => {
                cur.push(c);
                while let Some(c2) = chars.next() {
                    cur.push(c2);
                    if c2 == c {
                        if chars.peek() == Some(&c) {
                            cur.push(chars.next().unwrap());
                            continue;
                        }
                        break;
                    }
                }
            }
            '-' if chars.peek() == Some(&'-') => {
                for c2 in chars.by_ref() { if c2 == '\n' { cur.push('\n'); break; } }
            }
            '/' if chars.peek() == Some(&'*') => {
                let mut prev = ' ';
                for c2 in chars.by_ref() {
                    if prev == '*' && c2 == '/' { break; }
                    prev = c2;
                }
            }
            ';' => {
                if !cur.trim().is_empty() { out.push(std::mem::take(&mut cur)); }
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() { out.push(cur); }
    out
}

fn open(path: &str) -> Result<Connection, String> {
    Connection::open(path).map_err(|e| format!("open {path}: {e}"))
}

fn db_query(args: &Value) -> Result<String, String> {
    let path = args["path"].as_str().ok_or("path required")?;
    let sql = args["sql"].as_str().ok_or("sql required")?;
    let cap = args["max_rows"].as_u64().unwrap_or(MAX_ROWS as u64) as usize;
    let conn = open(path)?;
    // prepare() compiles only the FIRST statement — split the script
    // ourselves (quote/comment aware) so multi-statement SQL runs; each
    // result-producing statement's rows go to the reply.
    let mut out = String::new();
    let mut stmt_no = 0usize;
    for stmt_sql in split_statements(sql) {
        let mut stmt = conn.prepare(&stmt_sql)
            .map_err(|e| format!("prepare stmt {}: {e}", stmt_no + 1))?;
        stmt_no += 1;
        let ncols = stmt.column_count();
        if ncols == 0 {
            let n = stmt.execute([]).map_err(|e| format!("stmt {stmt_no}: {e}"))?;
            out.push_str(&format!("ok — {n} rows affected\n"));
        } else {
            let headers: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            out.push_str(&headers.join(" | "));
            let mut rows = stmt.query([]).map_err(|e| format!("query: {e}"))?;
            let mut n = 0usize;
            while let Some(row) = rows.next().map_err(|e| format!("row: {e}"))? {
                if n >= cap { out.push_str(&format!("\n… ({cap}-row cap)")); break; }
                let mut cells = Vec::with_capacity(ncols);
                for i in 0..ncols {
                    let v = row.get_ref(i).map_err(|e| e.to_string())?;
                    cells.push(match v {
                        rusqlite::types::ValueRef::Null => "".into(),
                        rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                        rusqlite::types::ValueRef::Real(f) => f.to_string(),
                        rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                        rusqlite::types::ValueRef::Blob(b) => format!("<{}B blob>", b.len()),
                    });
                }
                out.push('\n');
                out.push_str(&cells.join(" | "));
                n += 1;
            }
            out.push_str(&format!("\n({n} rows)\n"));
        }
    }
    Ok(out.trim_end().to_string())
}

fn db_tables(args: &Value) -> Result<String, String> {
    let conn = open(args["path"].as_str().ok_or("path required")?)?;
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| e.to_string())?;
    let names: Vec<String> = stmt.query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?.filter_map(Result::ok).collect();
    Ok(if names.is_empty() { "(no tables)".into() } else { names.join("\n") })
}

fn db_schema(args: &Value) -> Result<String, String> {
    let conn = open(args["path"].as_str().ok_or("path required")?)?;
    let (sql, val) = if let Some(t) = args["table"].as_str() {
        ("SELECT sql FROM sqlite_master WHERE name = ?1", Some(t.to_string()))
    } else {
        ("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name", None)
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows: Vec<String> = match val {
        Some(t) => stmt.query_map([t], |r| r.get(0))
            .map_err(|e| e.to_string())?.filter_map(Result::ok).collect(),
        None => stmt.query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?.filter_map(Result::ok).collect(),
    };
    Ok(if rows.is_empty() { "(no schema)".into() } else { rows.join(";\n\n") + ";" })
}

fn tool(name: &str, args: &Value) -> Value {
    let r = match name {
        "db_query" => db_query(args),
        "db_tables" => db_tables(args),
        "db_schema" => db_schema(args),
        t => Err(format!("unknown tool {t}")),
    };
    match r {
        Ok(content) => json!({"result": {"content": content}}),
        Err(e) => json!({"result": {"content": e, "is_error": true}}),
    }
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(v["params"]["name"].as_str().unwrap_or(""), &v["params"]["args"]);
                    r["id"] = id; r
                } else if method == "command/run" {
                    json!({"id": id, "result": {"text": "db: db_query/db_tables/db_schema tools"}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?; out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("db") => args[1..].to_vec(), _ => args,
    };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => { let _ = sidecar(); }
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("db — SQLite tools (db_query, db_tables, db_schema)");
                    return;
                }
                let _ = sidecar(); return;
            }
            eprintln!("unknown verb"); std::process::exit(1);
        }
    }
}
