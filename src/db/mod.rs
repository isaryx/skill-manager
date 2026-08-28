use std::fs;

use rusqlite::{params, Connection};

use crate::config::SkillMeta;
use crate::error::SkmError;
use crate::store::{discover_skill_ids, skills, StorePaths};
use crate::util::{hash_directory, is_skill_dir};

const SCHEMA: &str = include_str!("schema.sql");

pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub source_path: Option<String>,
    pub kind: String,
    pub sha: Option<String>,
    pub hash: String,
}

pub fn open_index(store: &StorePaths) -> Result<Connection, SkmError> {
    let conn = Connection::open(store.index_db())?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub fn rebuild_from_store(store: &StorePaths) -> Result<(), SkmError> {
    fs::create_dir_all(store.meta_dir())?;
    fs::create_dir_all(store.profiles_dir())?;

    let conn = open_index(store)?;
    conn.execute("DELETE FROM skills", [])?;

    for id in discover_skill_ids(store)? {
        if skills::is_skill_disabled(store, &id)? {
            continue;
        }
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

        conn.execute(
            "INSERT INTO skills (id, name, source_type, source_path, kind, sha, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, leaf_name, source_type, source_path, "pool", None::<String>, hash],
        )?;
    }

    Ok(())
}

/// Adopt on-disk skills missing meta, then rebuild `index.db`.
pub fn refresh_store_index(store: &StorePaths) -> Result<(), SkmError> {
    crate::store::ensure_meta_for_discovered_skills(store)?;
    rebuild_from_store(store)
}

pub fn list_skills(conn: &Connection) -> Result<Vec<SkillRow>, SkmError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, source_type, source_path, kind, sha, hash FROM skills ORDER BY id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SkillRow {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                source_path: row.get(3)?,
                kind: row.get(4)?,
                sha: row.get(5)?,
                hash: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
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
    fn rebuild_adopts_skill_with_meta() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill_dir = store.skill_dir("docx");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# docx").unwrap();

        let meta = SkillMeta {
            source_type: "local".to_string(),
            path: "/tmp/docx".to_string(),
            hash: "sha256:abc".to_string(),
            imported_at: "2026-01-01T00:00:00Z".to_string(),
            transfer: "copy".to_string(),
        };
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

        let meta = SkillMeta {
            source_type: "local".to_string(),
            path: "/tmp/original".to_string(),
            hash: "sha256:keep".to_string(),
            imported_at: "2026-01-01T00:00:00Z".to_string(),
            transfer: "copy".to_string(),
        };
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
}

