//! データベース操作を管理するモジュール。
//!
//! SQLiteを使用してコミックのメタデータ情報を保存および検索するための機能を提供します。

use rusqlite::{Connection, Result, params};
use std::{path::Path, sync::Mutex};

/// コミックのメタ情報を保持する構造体。
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

/// SQLiteデータベースの接続を管理する構造体。
///
/// スレッドセーフにアクセスできるように、`Mutex`で接続をラップしています。
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 新しいデータベース接続を作成、または既存のデータベースを開きます。
    ///
    /// 外部キー制約を有効にし、スキーマの初期化を行います。
    ///
    /// # Arguments
    ///
    /// * `path` - データベースファイルのパス。
    ///
    /// # Errors
    ///
    /// データベースのオープンや初期化処理に失敗した場合にエラーを返します。
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        conn.execute_batch(include_str!("./schema.sql"))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// テーブルから `name` に対応するIDを取得し、レコードが存在しなければ新規作成してIDを返します。
    ///
    /// # Arguments
    ///
    /// * `tx` - 実行中のトランザクション。
    /// * `table` - 対象となるマスターテーブルの名前。
    /// * `name` - 検索または挿入する名前。
    ///
    /// # Errors
    ///
    /// SQLクエリの実行やデータの挿入に失敗した場合にエラーを返します。
    fn get_or_create_master_id(
        tx: &rusqlite::Transaction,
        table: &str,
        name: &str,
    ) -> rusqlite::Result<i64> {
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
    }

    /// 中間テーブルへリレーションデータを挿入するヘルパー関数。
    ///
    /// # Arguments
    ///
    /// * `tx` - 実行中のトランザクション。
    /// * `table_master` - 参照先のマスターテーブルの名前。
    /// * `table_rel` - データを挿入する中間テーブルの名前。
    /// * `master_col` - 中間テーブルにおけるマスターIDの列名。
    /// * `item_id` - 紐づけるアイテムのID。
    /// * `names` - 登録する名前のリスト。
    ///
    /// # Errors
    ///
    /// SQLの実行に失敗した場合にエラーを返します。
    fn insert_relations(
        tx: &rusqlite::Transaction,
        table_master: &str,
        table_rel: &str,
        master_col: &str,
        item_id: i64,
        names: &[String],
    ) -> Result<()> {
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

            let master_id = Self::get_or_create_master_id(tx, table_master, name)?;

            // 中間テーブルへの挿入
            stmt_insert_rel.execute(params![master_id, item_id])?;
        }
        Ok(())
    }

    /// ギャラリー情報をデータベースに登録します。
    ///
    /// メインテーブルへの挿入後、各属性（作者、タグなど）の情報を中間テーブルに紐づけます。
    /// `info.gallery_id` が存在しない場合は、本家のIDとの衝突を避けるために
    /// データベース内で一意の**負のID**を自動採番して登録します。
    ///
    /// # Arguments
    ///
    /// * `info` - 挿入するギャラリーのメタデータ情報。
    /// * `dir_path` - ギャラリーの画像が保存されているディレクトリパス。
    ///
    /// # Returns
    ///
    /// 登録された、または既存のアイテムID (i64) を返します。
    ///
    /// # Errors
    ///
    /// トランザクションの実行やコミットに失敗した場合にエラーを返します。
    pub fn insert_info(&self, info: &Info, dir_path: &str) -> Result<i64> {
        let mut conn = self.conn.lock().expect("Database lock poisoned");
        let tx = conn.transaction()?;

        // 1. types テーブル
        let type_id: i64 = if info.type_name.is_empty() {
            Self::get_or_create_master_id(&tx, "types", "unknown")?
        } else {
            Self::get_or_create_master_id(&tx, "types", &info.type_name)?
        };

        // 2. languages テーブル
        let language_id: Option<i64> = if info.language.is_empty() {
            None
        } else {
            Some(Self::get_or_create_master_id(
                &tx,
                "languages",
                &info.language,
            )?)
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
                        row.get::<_, Option<i64>>(0)
                    })?
                    .unwrap_or(0)
                    - 1;

                tx.execute(
                    "INSERT INTO items (id, title, type_id, language_id, path) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![min_negative_new_id, info.title, type_id, language_id, dir_path],
                )?;
                min_negative_new_id
            }
        };

        Self::insert_relations(
            &tx,
            "artists",
            "item_artists",
            "artist_id",
            item_id,
            &info.artists,
        )?;
        Self::insert_relations(
            &tx,
            "groups",
            "item_groups",
            "group_id",
            item_id,
            &info.groups,
        )?;
        Self::insert_relations(
            &tx,
            "series",
            "item_series",
            "series_id",
            item_id,
            &info.series,
        )?;
        Self::insert_relations(
            &tx,
            "characters",
            "item_characters",
            "character_id",
            item_id,
            &info.characters,
        )?;
        Self::insert_relations(&tx, "tags", "item_tags", "tag_id", item_id, &info.tags)?;

        tx.commit()?;
        Ok(item_id)
    }

    /// 検索クエリに基づいてアイテムのパス一覧を取得します。
    ///
    /// プレフィックス（例: `tag:`, `artist:`）を用いた条件指定や、
    /// ハイフンマイナス(`-`)による除外検索、
    /// タイトルの部分一致検索に対応しています。
    /// ワイルドカード（`%` や `_`）はエスケープされ、リテラルとして検索されます。
    ///
    /// # Arguments
    ///
    /// * `query_str` - 空白区切りの検索クエリ文字列。
    ///
    /// # Errors
    ///
    /// クエリのパースやSQLの実行に失敗した場合にエラーを返します。
    pub fn search_items(&self, query_str: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.conn.lock().expect("Database lock poisoned");

        let mut base_query = String::from("SELECT DISTINCT i.path FROM items i WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        let terms = query_str.split_whitespace();

        // ワイルドカード文字をエスケープするクロージャ
        let escape_like = |s: &str| -> String {
            s.replace('\\', r"\\")
                .replace('%', r"\%")
                .replace('_', r"\_")
        };

        for term in terms {
            // 除外検索プレフィックスの処理
            let is_exclude = term.starts_with('-');
            let actual_term = if is_exclude { &term[1..] } else { &term };

            if actual_term.is_empty() {
                continue;
            }

            if let Some((prefix, value)) = actual_term.split_once(':') {
                // プレフィックスの処理
                let (table_rel, table_master, fk_col) = match prefix.to_lowercase().as_str() {
                    "tag" => ("item_tags", "tags", "tag_id"),
                    "artist" => ("item_artists", "artists", "artist_id"),
                    "group" => ("item_groups", "groups", "group_id"),
                    "series" => ("item_series", "series", "series_id"),
                    "character" => ("item_characters", "characters", "character_id"),
                    "language" => {
                        if is_exclude {
                            base_query.push_str(
                                " AND (i.language_id IS NULL OR i.language_id NOT IN (SELECT id FROM languages WHERE name = ?))"
                            );
                        } else {
                            base_query.push_str(
                                " AND i.language_id IN (SELECT id FROM languages WHERE name = ?)",
                            );
                        }
                        params.push(value.to_string());
                        continue;
                    }
                    "type" => {
                        if is_exclude {
                            base_query.push_str(
                                " AND (i.type_id IS NULL OR i.type_id NOT IN (SELECT id FROM types WHERE name = ?))"
                            );
                        } else {
                            base_query.push_str(
                                " AND i.type_id IN (SELECT id FROM types WHERE name = ?)",
                            );
                        }
                        params.push(value.to_string());
                        continue;
                    }
                    _ => {
                        // 未知のプレフィックスは通常のタイトル検索としてフォールバック
                        let condition = if is_exclude { "NOT LIKE" } else { "LIKE" };
                        base_query.push_str(&format!(" AND i.title {} ? ESCAPE '\\'", condition));
                        params.push(format!("%{}%", escape_like(actual_term)));
                        continue;
                    }
                };

                // 中間テーブルを経由する検索条件の構築
                let exists_clause = if is_exclude { "NOT EXISTS" } else { "EXISTS" };
                base_query.push_str(&format!(
                    " AND {} (SELECT 1 FROM {} rel JOIN {} m ON rel.{} = m.id WHERE rel.item_id = i.id AND m.name = ?)",
                    exists_clause, table_rel, table_master, fk_col
                ));
                params.push(value.to_string());
            } else {
                // プレフィックスなしの場合はタイトル検索
                let condition = if is_exclude { "NOT LIKE" } else { "LIKE" };
                base_query.push_str(&format!(" AND i.title {} ? ESCAPE '\\'", condition));
                params.push(format!("%{}%", escape_like(actual_term)));
            }
        }

        base_query.push_str(" ORDER BY i.id DESC");

        let mut stmt = conn.prepare(&base_query)?;

        // params_from_iterを使って動的パラメータをバインド
        let paths = stmt
            .query_map(rusqlite::params_from_iter(params), |row| row.get(0))?
            .filter_map(Result::ok)
            .collect::<Vec<String>>();

        Ok(paths)
    }
}
