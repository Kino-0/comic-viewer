use rusqlite::{Connection, Result, params};
use std::sync::Mutex;

// パースした情報を保持する構造体
#[derive(Debug, Default)]
pub struct Info {
    pub gallery_id: Option<i64>,
    pub title: String,
    pub artists: Vec<String>,
    pub groups: Vec<String>,
    pub type_name: String,
    pub series: Vec<String>,
    pub characters: Vec<String>,
    pub tags: Vec<String>,
    pub language: String,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        let init_sql = include_str!("./schema.sql");
        conn.execute_batch(init_sql)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_info(&self, info: &Info, dir_path: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // 1. types テーブル
        let type_id: u8 = tx.query_row(
            "SELECT id FROM types WHERE name = ?1",
            params![info.type_name],
            |row| row.get(0),
        )?;

        // 共通クロージャ: マスタからIDを取得（なければ新規作成）
        let get_or_create_master_id = |table: &str, name: &str| -> Result<i64> {
            let select_sql = format!("SELECT id FROM {} WHERE name = ?1", table);
            let insert_sql = format!("INSERT INTO {} (name) VALUES (?1)", table);

            let mut stmt_select = tx.prepare_cached(&select_sql)?;

            match stmt_select.query_row(params![name], |row| row.get(0)) {
                Ok(id) => Ok(id), // 既存データがあればIDを返す
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // なければ挿入して新しいIDを返す
                    let mut stmt_insert = tx.prepare_cached(&insert_sql)?;
                    stmt_insert.execute(params![name])?;
                    Ok(tx.last_insert_rowid())
                }
                Err(e) => Err(e),
            }
        };
        // 2. languages テーブル
        let language_id: Option<i64> = if info.language.is_empty() {
            None
        } else {
            Some(get_or_create_master_id("languages", &info.language)?)
        };

        // 3. items テーブルへの挿入
        let item_id = match info.gallery_id {
            Some(id) => {
                tx.execute(
                    "INSERT OR IGNORE INTO items (id, title, type_id, language_id, path) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, info.title, type_id, language_id, dir_path],
                )?;
                id
            }
            None => {
                // 本家のギャラリーIDとの衝突を避けるため、負の数で採番する。
                let min_negative_new_id: i64 = tx
                    .query_row("SELECT MIN(id) FROM items WHERE id < 0", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0)
                    - 1;

                tx.execute(
                    "INSERT INTO items (id, title, type_id, language_id, path) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![min_negative_new_id, info.title, type_id, language_id, dir_path],
                )?;
                min_negative_new_id
            }
        };

        // 4. 中間テーブルへ挿入するヘルパークロージャ
        let insert_relations = |table_master: &str,
                                table_rel: &str,
                                master_col: &str,
                                names: &[String]|
         -> Result<()> {
            if names.is_empty() {
                return Ok(());
            }

            let insert_rel_sql = format!(
                "INSERT OR IGNORE INTO {} ({}, item_id) VALUES (?1, ?2)",
                table_rel, master_col
            );
            let mut stmt_insert_rel = tx.prepare_cached(&insert_rel_sql)?;

            for name in names {
                if name.is_empty() {
                    continue;
                }

                let master_id = get_or_create_master_id(table_master, name)?;

                // 中間テーブルへの挿入
                stmt_insert_rel.execute(params![master_id, item_id])?;
            }
            Ok(())
        };

        insert_relations("artists", "item_artists", "artist_id", &info.artists)?;
        insert_relations("groups", "item_groups", "group_id", &info.groups)?;
        insert_relations("series", "item_series", "series_id", &info.series)?;
        insert_relations(
            "characters",
            "item_characters",
            "character_id",
            &info.characters,
        )?;
        insert_relations("tags", "item_tags", "tag_id", &info.tags)?;

        tx.commit()?;
        Ok(())
    }
}
