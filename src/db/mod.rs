use std::fs;

use rusqlite::{params, Connection};

use crate::config::SkillMeta;
use crate::error::SkmError;
use crate::store::{discover_skill_ids, skills, StorePaths};
use crate::util::{hash_directory, is_skill_dir, skill_spec};

const SCHEMA: &str = include_str!("schema.sql");

pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub source_path: Option<String>,
    pub kind: String,
    pub sha: Option<String>,
    pub hash: String,
    pub description: String,
    pub enabled: bool,
}

pub fn open_index(store: &StorePaths) -> Result<Connection, SkmError> {
    let conn = Connection::open(store.index_db())?;
    conn.execute_batch(SCHEMA)?;
    ensure_column(
        &conn,
        "description",
        "ALTER TABLE skills ADD COLUMN description TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "enabled",
        "ALTER TABLE skills ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1",
    )?;
    Ok(conn)
}

fn ensure_column(conn: &Connection, name: &str, migration: &str) -> Result<(), SkmError> {
    let mut stmt = conn.prepare("PRAGMA table_info(skills)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    if !columns.iter().any(|column| column == name) {
        conn.execute(migration, [])?;
    }
    Ok(())
}

pub fn rebuild_from_store(store: &StorePaths) -> Result<(), SkmError> {
    fs::create_dir_all(store.meta_dir())?;
    fs::create_dir_all(store.profiles_dir())?;

    let conn = open_index(store)?;
    conn.execute("DELETE FROM skills", [])?;

    let disabled = skills::read_disabled_ids(store)?;
    for id in discover_skill_ids(store)? {
        let skill_path = store.skill_dir(&id);
        if !is_skill_dir(&skill_path) {
            continue;
        }

        let meta_path = store.meta_file(&id);
        let (source_type, source_path, hash) = if meta_path.is_file() {
            let content = fs::read_to_string(&meta_path)?;
            let meta: SkillMeta = toml::from_str(&content)?;
            (meta.source_type, Some(meta.path), meta.hash)
        } else if let Some(bundle_id) = crate::store::bundle_meta_for_skill(store, &id) {
            let bundle_meta_path = store.meta_file(&bundle_id);
            if bundle_meta_path.is_file() {
                let content = fs::read_to_string(&bundle_meta_path)?;
                let meta: SkillMeta = toml::from_str(&content)?;
                let hash = hash_directory(&skill_path)?;
                (meta.source_type, Some(meta.path), hash)
            } else {
                let hash = hash_directory(&skill_path)?;
                ("unknown".to_string(), None, hash)
            }
        } else {
            let hash = hash_directory(&skill_path)?;
            ("unknown".to_string(), None, hash)
        };

        let leaf_name = id.rsplit('/').next().unwrap_or(&id).to_string();
        let description = skill_spec::read_description(&skill_path).unwrap_or_default();
        let enabled = !disabled.contains(&id);

        conn.execute(
            "INSERT INTO skills (id, name, source_type, source_path, kind, sha, hash, description, enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                leaf_name,
                source_type,
                source_path,
                "pool",
                None::<String>,
                hash,
                description,
                enabled
            ],
        )?;
    }

    Ok(())
}

/// Adopt on-disk skills missing meta, then rebuild `index.db`.
pub fn refresh_store_index(store: &StorePaths) -> Result<(), SkmError> {
    crate::store::ensure_meta_for_discovered_skills(store)?;
    rebuild_from_store(store)
}

const LIST_SKILLS_SQL: &str = "SELECT id, name, source_type, source_path, kind, sha, hash, description, enabled FROM skills WHERE enabled = 1 ORDER BY id";
const SEARCH_SKILLS_SQL: &str = "SELECT id, name, source_type, source_path, kind, sha, hash, description, enabled FROM skills ORDER BY id";

fn skill_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillRow> {
    Ok(SkillRow {
        id: row.get(0)?,
        name: row.get(1)?,
        source_type: row.get(2)?,
        source_path: row.get(3)?,
        kind: row.get(4)?,
        sha: row.get(5)?,
        hash: row.get(6)?,
        description: row.get(7)?,
        enabled: row.get(8)?,
    })
}

pub fn list_skills(conn: &Connection) -> Result<Vec<SkillRow>, SkmError> {
    let mut stmt = conn.prepare(LIST_SKILLS_SQL)?;
    let rows = stmt
        .query_map([], skill_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn count_indexed_skills(conn: &Connection) -> Result<usize, SkmError> {
    let count = conn.query_row("SELECT COUNT(*) FROM skills", [], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(count as usize)
}

pub fn search_skills(conn: &Connection, query_terms: &[String]) -> Result<Vec<SkillRow>, SkmError> {
    let terms: Vec<String> = query_terms.iter().map(|term| term.to_lowercase()).collect();
    let mut stmt = conn.prepare(SEARCH_SKILLS_SQL)?;
    let rows = stmt
        .query_map([], skill_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter(|row| {
            let haystack = format!("{} {}", row.id, row.description).to_lowercase();
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_store_layout;
    use tempfile::TempDir;

    #[test]
    fn rebuild_empty_store() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();
        rebuild_from_store(&store).unwrap();
        let conn = open_index(&store).unwrap();
        let skills = list_skills(&conn).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn open_index_migrates_existing_skill_table() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        fs::create_dir_all(store.skm_dir()).unwrap();
        let conn = Connection::open(store.index_db()).unwrap();
        conn.execute_batch(
            "CREATE TABLE skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_path TEXT,
                kind TEXT NOT NULL DEFAULT 'pool',
                sha TEXT,
                hash TEXT NOT NULL
            );
            INSERT INTO skills (id, name, source_type, kind, hash)
            VALUES ('docs', 'docs', 'local', 'pool', 'sha256:test');",
        )
        .unwrap();
        drop(conn);

        let conn = open_index(&store).unwrap();
        let rows = search_skills(&conn, &["docs".to_string()]).unwrap();

        assert_eq!(rows.len(), 1);
        assert!(rows[0].enabled);
        assert!(rows[0].description.is_empty());
    }

    #[test]
    fn rebuild_adopts_skill_with_meta() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill_dir = store.skill_dir("docx");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# docx").unwrap();

        let meta = SkillMeta::new(
            "local",
            "/tmp/docx",
            "sha256:abc",
            "2026-01-01T00:00:00Z",
            "copy",
        );
        let meta_content = toml::to_string(&meta).unwrap();
        fs::write(store.meta_file("docx"), meta_content).unwrap();

        rebuild_from_store(&store).unwrap();
        let conn = open_index(&store).unwrap();
        let skills = list_skills(&conn).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "docx");
    }

    #[test]
    fn refresh_store_index_adopts_skill_without_meta() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill_dir = store.skill_dir("docx");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# docx").unwrap();

        refresh_store_index(&store).unwrap();

        assert!(store.meta_file("docx").is_file());
        let conn = open_index(&store).unwrap();
        let skills = list_skills(&conn).unwrap();
        assert_eq!(skills[0].source_type, "store");
    }

    #[test]
    fn refresh_store_index_does_not_overwrite_existing_meta() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill_dir = store.skill_dir("docx");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# docx").unwrap();

        let meta = SkillMeta::new(
            "local",
            "/tmp/original",
            "sha256:keep",
            "2026-01-01T00:00:00Z",
            "copy",
        );
        fs::write(store.meta_file("docx"), toml::to_string(&meta).unwrap()).unwrap();

        refresh_store_index(&store).unwrap();

        let content = fs::read_to_string(store.meta_file("docx")).unwrap();
        assert!(content.contains("/tmp/original"));
        let conn = open_index(&store).unwrap();
        let skills = list_skills(&conn).unwrap();
        assert_eq!(skills[0].source_type, "local");
    }

    #[test]
    fn refresh_store_index_adopts_bundle_with_one_meta_file() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        for id in ["local/a", "local/b"] {
            let dir = store.skill_dir(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), "# skill").unwrap();
        }

        refresh_store_index(&store).unwrap();

        assert!(store.meta_file("local").is_file());
        assert!(!store.meta_file("local/a").is_file());
        assert!(!store.meta_file("local/b").is_file());
    }

    #[test]
    fn refresh_store_index_discovers_nested_skills() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill_dir = store.skill_dir("engineering/tdd");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# tdd").unwrap();

        refresh_store_index(&store).unwrap();
        let conn = open_index(&store).unwrap();
        let skills = list_skills(&conn).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "engineering/tdd");
        assert_eq!(skills[0].source_type, "store");
        assert!(store.meta_file("engineering").is_file());
    }

    #[test]
    fn list_skills_omits_disabled_skills() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        for id in ["docx", "git"] {
            let dir = store.skill_dir(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), format!("# {id}\n")).unwrap();
        }
        crate::store::skills::write_disabled_ids(&store, &["docx".to_string()]).unwrap();

        rebuild_from_store(&store).unwrap();
        let conn = open_index(&store).unwrap();
        let skills = list_skills(&conn).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "git");
    }

    #[test]
    fn rebuild_indexes_disabled_skills_for_search() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let dir = store.skill_dir("docs");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: docs\ndescription: Internal documentation helper.\n---\n",
        )
        .unwrap();
        crate::store::skills::write_disabled_ids(&store, &["docs".to_string()]).unwrap();

        rebuild_from_store(&store).unwrap();
        let conn = open_index(&store).unwrap();
        let rows = search_skills(&conn, &["documentation".to_string()]).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "docs");
        assert!(!rows[0].enabled);
        assert_eq!(rows[0].description, "Internal documentation helper.");
    }
}
