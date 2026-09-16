use std::path::Path;
use std::sync::Mutex;

use rusqlite::{named_params, Connection, OptionalExtension, Row};

use crate::error::{Error, Result};
use crate::model::{Item, ItemType, NewItem, Rule, Stats};

const SELECT_COLUMNS: &str = r#"
    id, "type", content, html_content, plain_text, image_path, thumb_path,
    content_hash, source_app, source_title, group_id, pinned,
    created_at, last_copied_at, last_used_at
"#;

/// 全文索引的版本。改了索引结构 / 分词器就 +1，启动时会自动重建一次。
const FTS_VERSION: &str = "1";

/// trigram 分词器至少要 3 个字符才能匹配。短于这个长度的查询（中文里两个字
/// 的词非常常见，比如「预算」）走 `LIKE` 回退 —— 宁可慢一点，也不能搜不到。
const FTS_MIN_CHARS: usize = 3;

pub struct Db {
    conn: Mutex<Connection>,
}

fn now_ms() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64 }

impl Db {
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("plico.db");

        match Self::connect(&db_path) {
            Ok(db) => Ok(db),
            Err(first) => {
                eprintln!("[plico] 数据库打开失败：{first}");
                quarantine(&db_path);
                eprintln!("[plico] 已把原文件改名保留，重建空库");
                Self::connect(&db_path)
            }
        }
    }

    fn connect(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // WAL + 单写连接；写操作通过 Mutex 串行化（R7 的对策）
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// 取数据库连接锁。
    ///
    /// 用 `unwrap_or_else(|e| e.into_inner())` 而不是 `unwrap()`：锁中毒只意味着
    /// 某个持有者 panic 过，SQLite 连接本身仍然可用（未提交的事务在 Drop 时已经
    /// 回滚）。如果这里直接 panic，监督线程重启后第一件事又是 panic，规格书
    /// 第 7 节的「崩溃自恢复」就永远起不来。
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(SCHEMA)?;
        drop(conn);
        self.ensure_fts()?;
        Ok(())
    }

    /// 建立 / 校验全文索引（F22）。
    ///
    /// 分两步：`SCHEMA` 里已经 `CREATE VIRTUAL TABLE IF NOT EXISTS` 并挂好了
    /// 同步触发器，所以**新建的库**从第一条记录起索引就是准的。但**老库**升级
    /// 上来时 `items` 里已经有数据、触发器又只覆盖之后的写入，必须全量 `rebuild`
    /// 一次。用 `schema_meta` 记一个版本号把它限定成「只做一次」——
    /// 每次启动都重建在万条量级上虽然也就几十毫秒，但那是白花的。
    fn ensure_fts(&self) -> Result<()> {
        let conn = self.conn();
        let done: Option<String> = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'fts_version'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if done.as_deref() == Some(FTS_VERSION) {
            return Ok(());
        }

        conn.execute("INSERT INTO items_fts(items_fts) VALUES('rebuild')", [])?;
        conn.execute(
            "INSERT INTO schema_meta(key, value) VALUES('fts_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [FTS_VERSION],
        )?;
        Ok(())
    }

    /// 插入或合并。返回 (条目, 是否命中重复)。
    ///
    /// 命中重复时：更新 `last_copied_at`、补齐来源信息，条目自然排到顶部（列表按
    /// `pinned DESC, last_copied_at DESC` 排序）。对应 F2。
    ///
    /// `max_items` 由调用方从设置里取（F14），不在这里读库，避免每来一条记录
    /// 就多查一次设置表。
    pub fn upsert(&self, new: &NewItem, hash: &str, now: i64, max_items: i64) -> Result<(Item, bool)> {
        let existing_id: Option<i64> = {
            let conn = self.conn();
            conn.query_row(
                r#"SELECT id FROM items WHERE content_hash = ?1"#,
                [hash],
                |r| r.get(0),
            )
            .optional()?
        };

        if let Some(id) = existing_id {
            {
                let conn = self.conn();
                conn.execute(
                    r#"UPDATE items
                          SET last_copied_at = ?1,
                              source_app   = COALESCE(?2, source_app),
                              source_title = COALESCE(?3, source_title)
                        WHERE id = ?4"#,
                    rusqlite::params![now, new.source_app, new.source_title, id],
                )?;
            }
            let item = self.get(id)?;
            return Ok((item, true));
        }

        {
            let conn = self.conn();
            conn.execute(
                r#"INSERT INTO items
                     ("type", content, html_content, plain_text, image_path, thumb_path,
                      content_hash, source_app, source_title, pinned, created_at, last_copied_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?10)"#,
                rusqlite::params![
                    new.kind.as_str(),
                    new.content,
                    new.html_content,
                    new.plain_text,
                    new.image_path,
                    new.thumb_path,
                    hash,
                    new.source_app,
                    new.source_title,
                    now,
                ],
            )?;
        }

        let item = self
            .get_by_hash(hash)?
            .ok_or_else(|| Error::Other("插入后未能读回条目".into()))?;

        // 容量策略：超出上限时淘汰最旧的未置顶条目
        self.enforce_capacity(max_items)?;

        Ok((item, false))
    }

    pub fn get(&self, id: i64) -> Result<Item> {
        let conn = self.conn();
        conn.query_row(
            &format!(r#"SELECT {SELECT_COLUMNS} FROM items WHERE id = ?1"#),
            [id],
            row_to_item,
        )
        .optional()?
        .ok_or(Error::NotFound(id))
    }

    fn get_by_hash(&self, hash: &str) -> Result<Option<Item>> {
        let conn = self.conn();
        Ok(conn
            .query_row(
                &format!(r#"SELECT {SELECT_COLUMNS} FROM items WHERE content_hash = ?1"#),
                [hash],
                row_to_item,
            )
            .optional()?)
    }

    /// monitor 在「命中去重就不落盘」的快路径上需要按哈希查一下存在性。
    /// 单独开一个 `pub(crate)` 出口，免得为这一处把 `get_by_hash` 整个公开。
    pub(crate) fn get_by_hash_pub(&self, hash: &str) -> Result<Option<Item>> {
        self.get_by_hash(hash)
    }

    /// 删除条目并把它引用的图片文件一起清掉（F9/F12/F14）。
    ///
    /// 图片文件按哈希命名，不同条目不可能共享同一张图（同哈希就合并成一条了），
    /// 所以删文件是安全的。文件删失败不阻塞删库记录 —— 孤儿文件比「想删删不掉」
    /// 好收拾得多。
    pub fn delete_with_files(&self, id: i64) -> Result<()> {
        let item = self.get(id)?;
        self.delete(id)?;
        remove_image_files(&item);
        Ok(())
    }

    /// 列表查询。`query` 走 LIKE 子串匹配（F6，大小写不敏感由 SQLite 的 LIKE 默认行为保证）。
    pub fn list(
        &self,
        query: Option<&str>,
        kind: Option<ItemType>,
        limit: i64,
    ) -> Result<Vec<Item>> {
        let q = query.map(str::trim).filter(|s| !s.is_empty());
        let (tag_query, text_query) = q.map(parse_tag_query).unwrap_or((None, None));
        let like = text_query.as_ref().map(|s| format!("%{}%", escape_like(s)));
        // 够长就走 FTS5，短查询走 LIKE（trigram 匹配不了少于 3 个字符的模式）
        let fts = text_query
            .as_ref()
            .filter(|s| s.chars().count() >= FTS_MIN_CHARS)
            .map(|s| fts_phrase(s));
        let kind_str = kind.map(|k| k.as_str().to_string());

        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            r#"SELECT {SELECT_COLUMNS} FROM items
                WHERE (:tag IS NULL OR EXISTS (
                          SELECT 1 FROM item_tags it JOIN tags t ON t.id = it.tag_id
                           WHERE it.item_id = items.id AND lower(t.name) = lower(:tag)
                       ))
                  AND (:text IS NULL
                       OR (:match IS NOT NULL
                           AND items.id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH :match))
                       OR (:match IS NULL
                           AND (COALESCE(plain_text, '') LIKE :like ESCAPE '\'
                             OR COALESCE(content, '')    LIKE :like ESCAPE '\'
                             OR COALESCE(source_app, '') LIKE :like ESCAPE '\'
                             OR COALESCE(source_title, '') LIKE :like ESCAPE '\')))
                  AND (:kind IS NULL OR "type" = :kind)
                ORDER BY pinned DESC, last_copied_at DESC
                LIMIT :limit"#
        ))?;

        let rows = stmt.query_map(
            named_params! {
                ":tag": tag_query,
                ":text": text_query,
                ":like": like,
                ":match": fts,
                ":kind": kind_str,
                ":limit": limit,
            },
            row_to_item,
        )?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        let n = conn.execute("DELETE FROM items WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(Error::NotFound(id));
        }
        Ok(())
    }

    /// 置顶 / 取消置顶，返回更新后的条目。
    pub fn toggle_pin(&self, id: i64) -> Result<Item> {
        {
            let conn = self.conn();
            let n = conn.execute(
                "UPDATE items SET pinned = CASE pinned WHEN 0 THEN 1 ELSE 0 END WHERE id = ?1",
                [id],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id));
            }
        }
        self.get(id)
    }

    /// 清空历史。`keep_pinned = true` 时保留置顶条目（F9 的默认行为）。
    pub fn clear(&self, keep_pinned: bool) -> Result<usize> {
        let conn = self.conn();
        let n = if keep_pinned {
            conn.execute("DELETE FROM items WHERE pinned = 0", [])?
        } else {
            conn.execute("DELETE FROM items", [])?
        };
        Ok(n)
    }

    pub fn mark_used(&self, id: i64, now: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE items SET last_used_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    pub fn stats(&self) -> Result<Stats> {
        let conn = self.conn();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))?;
        let pinned: i64 =
            conn.query_row("SELECT COUNT(*) FROM items WHERE pinned = 1", [], |r| r.get(0))?;
        Ok(Stats { total, pinned })
    }

    /// 图片配额（F14）：`images/` 下文件总大小超过 `quota_mb` 时，
    /// 删除最旧的未置顶图片条目直到回到配额内。返回删除条数。
    ///
    /// 策略与 `enforce_capacity` 一致：只动 `pinned = 0` 的，置顶永不因配额被删。
    pub fn purge_images(&self, quota_mb: i64) -> Result<usize> {
        let quota_bytes = quota_mb * 1024 * 1024;

        // 先查出所有未置顶的图片条目（含各自的原图路径），按时间从新到旧
        let items = {
            let conn = self.conn();
            let mut stmt = conn.prepare(
                r#"SELECT id, image_path, thumb_path FROM items
                    WHERE "type" = 'image' AND pinned = 0
                    ORDER BY last_copied_at DESC"#,
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            out
        };

        // 从头往后累计，遇到累计量超过配额的条目就删
        let mut used: i64 = 0;
        let mut removed = 0usize;
        for (id, image_path, thumb_path) in items {
            let size = image_path
                .as_deref()
                .map(|p| std::fs::metadata(p).map(|m| m.len() as i64).unwrap_or(0))
                .unwrap_or(0);

            used += size;
            if used > quota_bytes {
                self.delete(id)?;
                remove_image_files_raw(image_path.as_deref(), thumb_path.as_deref());
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// 容量策略（F14）：超出上限时删除最旧的未置顶条目。置顶条目永不因容量被删。
    ///
    /// 注意这里只删库记录 —— 条目引用的图片文件由 `purge_images` 在下一轮清理里
    /// 处理，容量淘汰路径上不值得为文件删除再加一次磁盘 IO。
    fn enforce_capacity(&self, max_items: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            r#"DELETE FROM items
                WHERE pinned = 0
                  AND id IN (
                      SELECT id FROM items
                       WHERE pinned = 0
                       ORDER BY last_copied_at DESC
                       LIMIT -1 OFFSET ?1
                  )"#,
            [max_items],
        )?;
        Ok(())
    }

    /// 保留策略（F14）：删除超过 `retention_days` 天的未置顶条目。返回删除条数。
    /// 置顶条目同样不受影响。
    pub fn purge_expired(&self, retention_days: i64, now: i64) -> Result<usize> {
        let cutoff = now - retention_days * 24 * 60 * 60 * 1000;
        let conn = self.conn();
        let n = conn.execute(
            "DELETE FROM items WHERE pinned = 0 AND last_copied_at < ?1",
            [cutoff],
        )?;
        Ok(n)
    }

    pub fn get_item_tags(&self, item_id: i64) -> Result<Vec<crate::model::Tag>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT t.id, t.name FROM tags t JOIN item_tags it ON it.tag_id=t.id WHERE it.item_id=?1 ORDER BY t.name")?;
        let rows = stmt.query_map([item_id], |r| Ok(crate::model::Tag { id: r.get(0)?, name: r.get(1)? }))?;
        let mut out = Vec::new(); for row in rows { out.push(row?); } Ok(out)
    }

    pub fn list_snippets(&self) -> Result<Vec<crate::model::Snippet>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id,title,content,tags,shortcut,created_at,updated_at FROM snippets ORDER BY updated_at DESC, id DESC")?;
        let rows = stmt.query_map([], |r| Ok(crate::model::Snippet { id:r.get(0)?, title:r.get(1)?, content:r.get(2)?, tags:r.get(3)?, shortcut:r.get(4)?, created_at:r.get(5)?, updated_at:r.get(6)? }))?;
        let mut out=Vec::new(); for row in rows { out.push(row?); } Ok(out)
    }

    pub fn create_snippet(&self, s: &crate::model::NewSnippet) -> Result<crate::model::Snippet> {
        let now = now_ms(); let conn=self.conn();
        conn.execute("INSERT INTO snippets(title,content,tags,shortcut,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)", rusqlite::params![s.title.trim(), s.content, s.tags, s.shortcut, now])?;
        self.list_snippets()?.into_iter().find(|x| x.created_at==now).ok_or_else(|| Error::Other("创建片段后未能读回".into()))
    }

    pub fn update_snippet(&self, id:i64, s:&crate::model::NewSnippet) -> Result<()> {
        let conn=self.conn(); let n=conn.execute("UPDATE snippets SET title=?1,content=?2,tags=?3,shortcut=?4,updated_at=?5 WHERE id=?6", rusqlite::params![s.title.trim(),s.content,s.tags,s.shortcut,now_ms(),id])?; if n==0 { return Err(Error::NotFound(id)); } Ok(())
    }

    pub fn delete_snippet(&self, id:i64) -> Result<()> { let conn=self.conn(); let n=conn.execute("DELETE FROM snippets WHERE id=?1",[id])?; if n==0 { return Err(Error::NotFound(id)); } Ok(()) }



    /// 读全部设置行。这里只负责搬运 key-value，解析交给 `settings` 模块，
    /// 免得两处都懂「设置长什么样」。
    pub fn all_settings(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// 批量 upsert 设置。整份设置是一个事务，避免改到一半留个半成品状态。
    pub fn put_settings(&self, pairs: &[(String, String)]) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )?;
            for (k, v) in pairs {
                stmt.execute(rusqlite::params![k, v])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// 分组列表，按用户排定的顺序（F16）。
    ///
    /// **`ORDER BY sort_order, name` 里的 `name` 只是平手时的兜底**，不是主要依据：
    /// `sort_order` 理论上唯一，但老库（`sort_order` 全是默认 0 的那批）和并发写入
    /// 都可能产生并列，加一个确定性次级键能让结果稳定，不会每次查询顺序都在跳。
    pub fn list_groups(&self) -> Result<Vec<crate::model::Group>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, name, color, sort_order FROM groups ORDER BY sort_order, name")?;
        let rows = stmt.query_map([], |r| Ok(crate::model::Group { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, sort_order: r.get(3)? }))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    /// 建分组（F16）。
    ///
    /// `color` 允许为空 —— 「石墨」是默认色，用户不选就是它。
    /// 撞名**报错而不是静默返回已有分组**：用户以为新建了一个，实际上
    /// 在往另一个分组里塞条目，这种失败很难自己发现。与 `rename_tag` 同一立场。
    ///
    /// **`sort_order` 必须显式取 `MAX + 1`**：建表时它是 `DEFAULT 0`，而排序读的是
    /// `ORDER BY sort_order, name` —— 全写 0 的话顺序实际退化成按名称字母序，
    /// 「新建的分组排在末尾」这条就不成立了（会插到字母序该在的位置去）。
    pub fn create_group(&self, name: &str, color: Option<&str>) -> Result<crate::model::Group> {
        let name = name.trim();
        if name.is_empty() { return Err(Error::Other("分组名称不能为空".into())); }
        if self.group_name_taken(name, None)? {
            return Err(Error::Other(format!("已存在同名分组「{name}」")));
        }
        let color = color.map(str::trim).filter(|c| !c.is_empty());
        let conn = self.conn();
        conn.execute(
            "INSERT INTO groups (name, color, sort_order)
             VALUES (?1, ?2, COALESCE((SELECT MAX(sort_order) FROM groups), -1) + 1)",
            rusqlite::params![name, color],
        )?;
        drop(conn);
        self.list_groups()?.into_iter().find(|g| g.name == name).ok_or_else(|| Error::Other("创建分组失败".into()))
    }

    /// 改名 / 改色（F16）。`color` 传 `None` 表示不动颜色。
    ///
    /// `color` 与 `name` 分成两个参数而不是塞一个 struct：重命名对话框只想改名字，
    /// 不该被迫把当前色值原样回传一遍（那样前端得先读一次颜色，多一次往返）。
    pub fn rename_group(&self, id: i64, name: &str, color: Option<&str>) -> Result<()> {
        let name = name.trim();
        if name.is_empty() { return Err(Error::Other("分组名称不能为空".into())); }
        if self.group_name_taken(name, Some(id))? {
            return Err(Error::Other(format!("已存在同名分组「{name}」")));
        }
        let color = color.map(str::trim).filter(|c| !c.is_empty());
        let conn = self.conn();
        let changed = match color {
            Some(c) => conn.execute("UPDATE groups SET name=?1, color=?2 WHERE id=?3", rusqlite::params![name, c, id])?,
            None => conn.execute("UPDATE groups SET name=?1 WHERE id=?2", rusqlite::params![name, id])?,
        };
        if changed == 0 { return Err(Error::NotFound(id)); }
        Ok(())
    }

    /// 分组名是否已被占用。`except` 用于重命名时排除自己。
    fn group_name_taken(&self, name: &str, except: Option<i64>) -> Result<bool> {        let conn = self.conn();
        let hit: Option<i64> = conn
            .query_row(
                "SELECT id FROM groups WHERE name = ?1",
                [name],
                |r| r.get(0),
            )
            .optional()?;
        Ok(matches!(hit, Some(id) if Some(id) != except))
    }

    /// 按传入顺序重写分组的 `sort_order`（F16 排序 UI）。
    ///
    /// 收的是**完整 id 顺序**（不是「把 X 上移一位」这种增量指令）：拖动/上下移
    /// 之后前端手里就是一份完整列表，整份提交最省事，也不会出现「同一秒两个增量
    /// 指令互相抵消」的问题。`sort_order` 重写成 `0..n-1`，顺带把老库那批全是 0 的
    /// 纪录扶正。
    ///
    /// **校验 id 集合必须与库中完全一致**：前端可能拿着一份过期的列表（别的窗口
    /// 刚建/删过分组）。若只按传入的 id 更新，没被提到的分组会保留旧的 `sort_order`，
    /// 与新的 0..n 撞在一起 —— 表现为「排完序顺序还是乱的」，很难查。宁可报错让前端重取。
    pub fn reorder_groups(&self, ids: &[i64]) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;

        let existing: std::collections::HashSet<i64> = {
            let mut stmt = tx.prepare("SELECT id FROM groups")?;
            let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
            rows.collect::<rusqlite::Result<std::collections::HashSet<_>>>()?
        };

        let incoming: std::collections::HashSet<i64> = ids.iter().copied().collect();
        // 重复 id 会让后面按 index 写 sort_order 时产生并列值，也要挡住
        if incoming.len() != ids.len() {
            return Err(Error::Other("分组顺序里有重复项".into()));
        }
        if incoming != existing {
            return Err(Error::Other("分组列表已变化，请重试".into()));
        }

        for (index, id) in ids.iter().enumerate() {
            tx.execute("UPDATE groups SET sort_order=?1 WHERE id=?2", rusqlite::params![index as i64, id])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 删分组。条目**回到未分组**，不跟着一起消失（F16 验收标准）。
    ///
    /// 两条语句包在一个事务里：否则「删了分组但条目还挂着已消失的 group_id」
    /// 这种中间态一旦被并发读到，过滤视图里那些条目会直接看不见。
    pub fn delete_group(&self, id: i64) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        if tx.execute("DELETE FROM groups WHERE id=?1", [id])? == 0 { return Err(Error::NotFound(id)); }
        tx.execute("UPDATE items SET group_id=NULL WHERE group_id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }
    pub fn assign_item_group(&self, item_id: i64, group_id: Option<i64>) -> Result<()> { let conn=self.conn(); if conn.execute("UPDATE items SET group_id=?1 WHERE id=?2", rusqlite::params![group_id,item_id])?==0{return Err(Error::NotFound(item_id));} Ok(()) }

    pub fn list_tags(&self) -> Result<Vec<crate::model::Tag>> {
        let conn=self.conn(); let mut s=conn.prepare("SELECT id,name FROM tags ORDER BY name")?;
        let rows=s.query_map([], |r| Ok(crate::model::Tag{id:r.get(0)?,name:r.get(1)?}))?; let out=rows.collect::<rusqlite::Result<Vec<_>>>()?; Ok(out)
    }
    pub fn assign_item_tag(&self, item_id:i64, name:&str) -> Result<Vec<crate::model::Tag>> {
        let name=name.trim(); if name.is_empty(){return Err(Error::Other("标签不能为空".into()));}
        let mut c=self.conn(); let tx=c.transaction()?;
        if tx.query_row("SELECT id FROM items WHERE id=?1",[item_id],|r|r.get::<_,i64>(0)).optional()?.is_none(){return Err(Error::NotFound(item_id));}
        tx.execute("INSERT INTO tags(name) VALUES(?1) ON CONFLICT(name) DO NOTHING",[name])?;
        let tid:i64=tx.query_row("SELECT id FROM tags WHERE name=?1",[name],|r|r.get(0))?;
        tx.execute("INSERT OR IGNORE INTO item_tags(item_id,tag_id) VALUES(?1,?2)",[item_id,tid])?; tx.commit()?; drop(c); self.item_tags(item_id)
    }
    pub fn remove_item_tag(&self,item_id:i64,name:&str)->Result<Vec<crate::model::Tag>> { let c=self.conn(); c.execute("DELETE FROM item_tags WHERE item_id=?1 AND tag_id=(SELECT id FROM tags WHERE name=?2)",rusqlite::params![item_id,name.trim()])?; drop(c); self.item_tags(item_id) }
    pub fn item_tags(&self,item_id:i64)->Result<Vec<crate::model::Tag>> { let c=self.conn(); let mut s=c.prepare("SELECT t.id,t.name FROM tags t JOIN item_tags it ON it.tag_id=t.id WHERE it.item_id=?1 ORDER BY t.name")?; let rows=s.query_map([item_id],|r|Ok(crate::model::Tag{id:r.get(0)?,name:r.get(1)?}))?; let out=rows.collect::<rusqlite::Result<Vec<_>>>()?; Ok(out) }

    /// 标签 + 使用计数，按名称排序。
    pub fn list_tag_stats(&self) -> Result<Vec<crate::model::TagStat>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, COUNT(it.item_id)
               FROM tags t LEFT JOIN item_tags it ON it.tag_id = t.id
              GROUP BY t.id, t.name
              ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(crate::model::TagStat { id: r.get(0)?, name: r.get(1)?, count: r.get(2)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// 重命名标签。
    ///
    /// 目标名已被别的标签占用时**报错而不是静默合并** —— 合并会丢掉一个标签，
    /// 是破坏性操作，必须由用户显式选择走 `merge_tags`。
    pub fn rename_tag(&self, id: i64, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() { return Err(Error::Other("标签名不能为空".into())); }
        let conn = self.conn();
        let taken: Option<i64> = conn
            .query_row("SELECT id FROM tags WHERE name = ?1", [name], |r| r.get(0))
            .optional()?;
        if let Some(other) = taken {
            if other != id { return Err(Error::Other(format!("已存在同名标签「{name}」"))); }
        }
        if conn.execute("UPDATE tags SET name = ?1 WHERE id = ?2", rusqlite::params![name, id])? == 0 {
            return Err(Error::NotFound(id));
        }
        Ok(())
    }

    /// 把 `from` 合并进 `to`：条目归属整体搬迁，然后删掉 `from`。
    ///
    /// `INSERT OR IGNORE` 是必需的 —— 同一条记录可能同时挂着两个标签，
    /// 直接 INSERT 会撞 `item_tags` 的联合主键。
    pub fn merge_tags(&self, from: i64, to: i64) -> Result<()> {
        if from == to { return Ok(()); }
        let mut c = self.conn();
        let tx = c.transaction()?;
        let found: i64 = tx.query_row(
            "SELECT COUNT(*) FROM tags WHERE id IN (?1, ?2)",
            rusqlite::params![from, to],
            |r| r.get(0),
        )?;
        if found != 2 { return Err(Error::NotFound(if found == 0 { from } else { to })); }
        tx.execute(
            "INSERT OR IGNORE INTO item_tags(item_id, tag_id) SELECT item_id, ?2 FROM item_tags WHERE tag_id = ?1",
            rusqlite::params![from, to],
        )?;
        // 删 tags 行时外键 CASCADE 会清掉 from 残留的 item_tags（连接已开 foreign_keys）
        tx.execute("DELETE FROM tags WHERE id = ?1", [from])?;
        tx.commit()?;
        Ok(())
    }

    /// 删除标签。条目本身不受影响，只是不再带这个标签。
    pub fn delete_tag(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        if conn.execute("DELETE FROM tags WHERE id = ?1", [id])? == 0 {
            return Err(Error::NotFound(id));
        }
        Ok(())
    }


    pub fn list_rules(&self) -> Result<Vec<Rule>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, kind, value, enabled FROM rules ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Rule {
                id: r.get(0)?,
                kind: r.get(1)?,
                value: r.get(2)?,
                enabled: r.get::<_, i64>(3)? != 0,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// 只取启用的规则。监听线程走这条，省掉一次过滤。
    pub fn enabled_rules(&self) -> Result<Vec<Rule>> {
        Ok(self
            .list_rules()?
            .into_iter()
            .filter(|r| r.enabled)
            .collect())
    }

    pub fn add_rule(&self, kind: &str, value: &str) -> Result<Rule> {
        let value = value.trim();
        if value.is_empty() {
            return Err(Error::Other("规则内容不能为空".into()));
        }

        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO rules (kind, value, enabled) VALUES (?1, ?2, 1)",
                rusqlite::params![kind, value],
            )?;
        }

        // 读回刚插入的那条，把自增 id 带回去给前端
        self.list_rules()?
            .into_iter()
            .filter(|r| r.kind == kind && r.value == value)
            .next_back()
            .ok_or_else(|| Error::Other("插入规则后未能读回".into()))
    }

    pub fn delete_rule(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        let n = conn.execute("DELETE FROM rules WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(Error::NotFound(id));
        }
        Ok(())
    }

    pub fn set_rule_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.conn();
        let n = conn.execute(
            "UPDATE rules SET enabled = ?1 WHERE id = ?2",
            rusqlite::params![enabled as i64, id],
        )?;
        if n == 0 {
            return Err(Error::NotFound(id));
        }
        Ok(())
    }
}

fn row_to_item(row: &Row<'_>) -> rusqlite::Result<Item> {
    let kind_str: String = row.get(1)?;
    Ok(Item {
        id: row.get(0)?,
        kind: ItemType::parse(&kind_str).unwrap_or(ItemType::Text),
        content: row.get(2)?,
        html_content: row.get(3)?,
        plain_text: row.get(4)?,
        image_path: row.get(5)?,
        thumb_path: row.get(6)?,
        content_hash: row.get(7)?,
        source_app: row.get(8)?,
        source_title: row.get(9)?,
        group_id: row.get(10)?,
        pinned: row.get(11)?,
        created_at: row.get(12)?,
        last_copied_at: row.get(13)?,
        last_used_at: row.get(14)?,
    })
}

/// Match the first whitespace-delimited tag filter, as in the browser mock.
fn parse_tag_query(query: &str) -> (Option<String>, Option<String>) {
    for (start, _) in query.char_indices().filter(|&(i, _)| {
        i == 0 || query[..i].chars().next_back().is_some_and(char::is_whitespace)
    }) {
        let token = query[start..].split_whitespace().next().unwrap_or("");
        if token.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("tag:"))
            && token.len() > 4
        {
            let text = format!("{} {}", query[..start].trim_end(), &query[start + token.len()..]);
            let text = text.trim().to_string();
            return (Some(token[4..].to_string()), (!text.is_empty()).then_some(text));
        }
    }
    (None, Some(query.to_string()))
}
/// 转义 LIKE 的通配符，避免用户输入的 `%` / `_` 被当成模式。
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// 把用户输入包成 FTS5 的**带引号短语**，这样整个串按字面匹配。
///
/// 必须包引号：FTS5 的 MATCH 语法里 `AND` / `OR` / `NOT` / `*` / `^` / `-`
/// 都是操作符，用户搜 `foo -bar` 或 `a AND b` 会直接语法错误。
/// 引号内部的 `"` 按 FTS5 的规矩用两个 `"` 转义。
fn fts_phrase(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// 把打不开的数据库文件挪到一边。返回实际使用的备份路径（挪不动就返回 None）。
///
/// 用改名而不是删除：万一还能人工抢救出内容，直接删掉就一点机会都没有了。
/// WAL / SHM 必须一起挪走 —— 残留的 `-wal` 会让新建的空库读到旧事务。
fn quarantine(db_path: &Path) -> Option<std::path::PathBuf> {
    if !db_path.exists() {
        return None;
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let backup = db_path.with_extension(format!("db.corrupt-{stamp}"));
    if let Err(e) = std::fs::rename(db_path, &backup) {
        eprintln!("[plico] 无法改名损坏的数据库：{e}");
        return None;
    }

    for suffix in ["-wal", "-shm"] {
        let side = append_suffix(db_path, suffix);
        if side.exists() {
            let _ = std::fs::rename(&side, append_suffix(&backup, suffix));
        }
    }

    eprintln!("[plico] 损坏的数据库已保留为：{}", backup.display());
    Some(backup)
}

/// `plico.db` + `-wal` → `plico.db-wal`。
/// 不能用 `with_extension`，那会把 `.db` 整个换掉。
fn append_suffix(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    std::path::PathBuf::from(s)
}

/// 删除条目引用的图片文件（原图 + 缩略图）。删不掉只记日志，不阻塞调用方。
fn remove_image_files(item: &Item) {
    remove_image_files_raw(item.image_path.as_deref(), item.thumb_path.as_deref());
}

fn remove_image_files_raw(image_path: Option<&str>, thumb_path: Option<&str>) {
    for path in [image_path, thumb_path].into_iter().flatten() {
        if let Err(e) = std::fs::remove_file(path) {
            // 文件可能已经被手动删了，不存在不算错误
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[plico] 删除图片文件失败 {path}：{e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个用例一个独占目录。用 pid + 纳秒，避免并行跑测试时撞车。
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("plico-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("创建临时目录失败");
        dir
    }

    #[test]
    fn 附属文件名是拼接而不是换扩展名() {        assert_eq!(
            append_suffix(Path::new("C:/x/plico.db"), "-wal"),
            Path::new("C:/x/plico.db-wal")
        );
    }

    #[test]
    fn 损坏的库被改名保留而不是删除() {
        let dir = temp_dir("corrupt");
        let db_path = dir.join("plico.db");
        std::fs::write(&db_path, b"not a database").unwrap();
        std::fs::write(append_suffix(&db_path, "-wal"), b"stale wal").unwrap();

        let backup = quarantine(&db_path).expect("应该改名成功");

        assert!(!db_path.exists(), "原文件应该已经挪走");
        assert!(backup.exists(), "备份文件应该存在");
        assert!(
            append_suffix(&backup, "-wal").exists(),
            "WAL 必须跟着一起挪，否则会污染新建的空库"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 文件不存在时不做任何事() {
        let dir = temp_dir("missing");
        assert!(quarantine(&dir.join("nope.db")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 打开损坏的库会重建空库并保留原文件() {
        let dir = temp_dir("reopen");
        // 一个头部不合法的文件，SQLite 会判定为 not a database
        std::fs::write(dir.join("plico.db"), vec![0xABu8; 4096]).unwrap();

        let db = Db::open(&dir).expect("应该能重建出可用的空库");
        assert_eq!(db.stats().expect("重建后的库应该可用").total, 0);
        drop(db);

        let kept = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(kept, "损坏的原文件应该被改名保留，而不是删掉");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 这条是「监听线程崩溃自恢复」能成立的前提：某个线程 panic 过之后，
    /// 锁会中毒，但数据库本身必须还能用，否则重启后的线程立刻又死。
    #[test]
    fn 锁中毒之后数据库仍然可用() {
        let dir = temp_dir("poison");
        let db = Db::open(&dir).expect("建库失败");

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = db.conn.lock().unwrap();
            panic!("故意毒化锁");
        }));

        assert!(db.conn.is_poisoned(), "锁应该已经中毒");
        assert_eq!(db.stats().expect("中毒后仍应能查询").total, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------------- F17 标签管理 ----------------

    /// 建一个文本条目，返回它的 id。标签相关的用例都从「先有东西可挂」开始。
    fn seed_item(db: &Db, text: &str) -> i64 {
        let new = NewItem {
            kind: ItemType::Text,
            content: Some(text.to_string()),
            plain_text: Some(text.to_string()),
            ..Default::default()
        };
        let hash = crate::storage::normalize::hash_text(text);
        db.upsert(&new, &hash, 1_000, 1_000)
            .expect("插入条目失败")
            .0
            .id
    }

    /// 计数是标签页决定「合并还是删除」的依据，算错了用户就会盲操作。
    /// 特别要守住「count = 0 的标签仍然出现在列表里」—— 否则它就没法被删掉了。
    #[test]
    fn 标签计数反映实际挂载的条目数() {
        let dir = temp_dir("tag-stat");
        let db = Db::open(&dir).expect("建库失败");

        let a = seed_item(&db, "第一条");
        let b = seed_item(&db, "第二条");
        db.assign_item_tag(a, "工作").unwrap();
        db.assign_item_tag(b, "工作").unwrap();
        db.assign_item_tag(a, "重要").unwrap();

        let stats = db.list_tag_stats().unwrap();
        let count = |n: &str| stats.iter().find(|s| s.name == n).map(|s| s.count);
        assert_eq!(count("工作"), Some(2));
        assert_eq!(count("重要"), Some(1));

        // 摘掉最后一个归属后，标签本身还在（count = 0），不能被 LEFT JOIN 吃掉
        db.assign_item_tag(a, "临时").unwrap();
        db.remove_item_tag(a, "临时").unwrap();
        let stats = db.list_tag_stats().unwrap();
        assert_eq!(
            stats.iter().find(|s| s.name == "临时").map(|s| s.count),
            Some(0),
            "没人用的标签也要留在列表里，否则用户在标签页里看不到它"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 合并的核心是「归属整体搬迁」。同时挂着两个待合并标签的条目是最容易出问题
    /// 的情况：直接 INSERT 会撞 `item_tags` 的联合主键，必须 `INSERT OR IGNORE`。
    #[test]
    fn 合并标签会把条目归属整体搬迁() {
        let dir = temp_dir("tag-merge");
        let db = Db::open(&dir).expect("建库失败");

        let a = seed_item(&db, "只挂旧标签");
        let b = seed_item(&db, "两个都挂");
        db.assign_item_tag(a, "旧").unwrap();
        db.assign_item_tag(b, "旧").unwrap();
        db.assign_item_tag(b, "新").unwrap();

        let stats = db.list_tag_stats().unwrap();
        let old = stats.iter().find(|s| s.name == "旧").unwrap().id;
        let new = stats.iter().find(|s| s.name == "新").unwrap().id;

        db.merge_tags(old, new).expect("合并应该成功");

        let stats = db.list_tag_stats().unwrap();
        assert!(stats.iter().all(|s| s.name != "旧"), "被合并掉的标签应该消失");
        assert_eq!(stats.iter().find(|s| s.name == "新").unwrap().count, 2);

        // b 本来就挂着「新」，搬迁时那一行要被 IGNORE 掉，不能出现重复归属
        assert_eq!(db.item_tags(b).unwrap().len(), 1);
        assert_eq!(db.item_tags(a).unwrap()[0].name, "新");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 重命名撞上已有标签时**必须报错**。若静默合并，用户会以为只是改了个名，
    /// 实际丢了一个标签 —— 那是破坏性操作，得由用户显式走 merge。
    #[test]
    fn 重命名撞名时拒绝而不是静默合并() {
        let dir = temp_dir("tag-rename");
        let db = Db::open(&dir).expect("建库失败");

        let a = seed_item(&db, "条目");
        db.assign_item_tag(a, "甲").unwrap();
        db.assign_item_tag(a, "乙").unwrap();

        let stats = db.list_tag_stats().unwrap();
        let jia = stats.iter().find(|s| s.name == "甲").unwrap().id;

        assert!(db.rename_tag(jia, "乙").is_err(), "重名必须报错");
        assert_eq!(db.list_tag_stats().unwrap().len(), 2, "失败后两个标签都应原样还在");

        db.rename_tag(jia, "丙").expect("换成没被占用的名字应该成功");
        let names: Vec<String> = db.list_tag_stats().unwrap().into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"丙".to_string()) && names.contains(&"乙".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------------- F16 分组 ----------------

    /// 分组名撞上已有分组时必须报错。理由和标签重命名一致：静默复用会变成
    /// 「用户以为新建了，实际在往另一个分组塞条目」，这种失败用户自己发现不了。
    #[test]
    fn 建分组撞名时拒绝() {
        let dir = temp_dir("group-dupe");
        let db = Db::open(&dir).expect("建库失败");

        db.create_group("工作", Some("--pl-group-sky")).expect("首次创建应成功");
        assert!(db.create_group("工作", None).is_err(), "重名必须报错");
        assert_eq!(db.list_groups().unwrap().len(), 1, "失败后不该多出分组");

        // 首尾空白应该被裁掉后再比较，`"  工作  "` 也算撞名
        assert!(db.create_group("  工作  ", None).is_err(), "裁剪空白后仍算撞名");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 改名时不能被自己挡住 —— 这是 `group_name_taken` 里 `except` 参数的意义。
    /// 只改色不改名也要走通（`color` 与 `name` 是两个独立参数）。
    #[test]
    fn 分组改名与改色互不干扰() {
        let dir = temp_dir("group-rename");
        let db = Db::open(&dir).expect("建库失败");

        let g = db.create_group("工作", Some("--pl-group-sky")).expect("建库失败");
        assert_eq!(g.color.as_deref(), Some("--pl-group-sky"));

        // 名字没变，只是改色：不能报撞名
        db.rename_group(g.id, "工作", Some("--pl-group-amber")).expect("只改色应成功");
        let after = db.list_groups().unwrap();
        assert_eq!(after[0].color.as_deref(), Some("--pl-group-amber"));

        // 只改名，颜色要保持不动
        db.rename_group(g.id, "项目", None).expect("只改名应成功");
        let after = db.list_groups().unwrap();
        assert_eq!(after[0].name, "项目");
        assert_eq!(after[0].color.as_deref(), Some("--pl-group-amber"), "没传 color 时不该被清掉");

        // 另一个分组撞上来仍然要拦
        db.create_group("归档", None).unwrap();
        assert!(db.rename_group(g.id, "归档", None).is_err(), "改成别人的名字必须报错");

        // 空名字拒绝
        assert!(db.rename_group(g.id, "   ", None).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 删分组后条目回到未分组，而不是跟着一起消失（F16 验收标准）。
    #[test]
    fn 删分组后条目回到未分组() {
        let dir = temp_dir("group-delete");
        let db = Db::open(&dir).expect("建库失败");

        let g = db.create_group("临时", None).unwrap();
        let a = seed_item(&db, "第一条");
        let b = seed_item(&db, "第二条");
        db.assign_item_group(a, Some(g.id)).unwrap();
        db.assign_item_group(b, Some(g.id)).unwrap();

        db.delete_group(g.id).expect("删除应成功");

        assert!(db.list_groups().unwrap().is_empty());
        // 条目本身还在，只是 group_id 被清空
        assert_eq!(db.list(None, None, 100).unwrap().len(), 2, "条目不该被连带删除");
        assert!(db.list(None, None, 100).unwrap().iter().all(|i| i.group_id.is_none()));

        assert!(db.delete_group(g.id).is_err(), "删不存在的分组要报错");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F16 排序：新建的分组必须落在末尾。
    ///
    /// 建表时 `sort_order` 是 `DEFAULT 0`，若 INSERT 不显式写值，全部记录都是 0，
    /// `ORDER BY sort_order, name` 会退化成按名字字母序 —— 「新建的排在最后」这条
    /// 就不再成立（会插到字母序该在的位置去）。这里连建三个名字**故意乱序**的
    /// 分组，逼出这个区别。
    #[test]
    fn 新建分组排在末尾() {
        let dir = temp_dir("group-append");
        let db = Db::open(&dir).expect("建库失败");

        db.create_group("丙", None).unwrap();
        db.create_group("甲", None).unwrap();
        db.create_group("乙", None).unwrap();

        let names: Vec<String> = db.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert_eq!(names, vec!["丙", "甲", "乙"], "应按创建顺序，而不是按名字排序");

        let orders: Vec<i64> = db.list_groups().unwrap().into_iter().map(|g| g.sort_order).collect();
        assert_eq!(orders, vec![0, 1, 2]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F16 排序：整份 id 顺序提交后，列表就按新顺序读出来，且 `sort_order` 归一成 `0..n-1`。
    #[test]
    fn 重排分组后顺序生效() {
        let dir = temp_dir("group-reorder");
        let db = Db::open(&dir).expect("建库失败");

        let a = db.create_group("甲", None).unwrap();
        let b = db.create_group("乙", None).unwrap();
        let c = db.create_group("丙", None).unwrap();

        db.reorder_groups(&[c.id, a.id, b.id]).expect("重排应成功");
        let names: Vec<String> = db.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert_eq!(names, vec!["丙", "甲", "乙"]);

        let orders: Vec<i64> = db.list_groups().unwrap().into_iter().map(|g| g.sort_order).collect();
        assert_eq!(orders, vec![0, 1, 2], "应重写成 0..n-1，不留空洞");

        // 再排一次回到原序，结果必须稳定（幂等：同一份顺序提交两次没有额外影响）
        db.reorder_groups(&[a.id, b.id, c.id]).unwrap();
        db.reorder_groups(&[a.id, b.id, c.id]).unwrap();
        let names: Vec<String> = db.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert_eq!(names, vec!["甲", "乙", "丙"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F16 排序：前端拿的是过期列表时必须报错，而不是「部分更新」。
    ///
    /// 关键点是**集合必须完全一致**。若只按传入的 id 更新，没被提到的分组会留着
    /// 旧的 `sort_order`，与新的 0..n 撞在一起 —— 表现为「排完序顺序还是乱的」，
    /// 而且数据已经写坏，很难查。
    #[test]
    fn 重排分组时过期列表被拒绝() {
        let dir = temp_dir("group-reorder-stale");
        let db = Db::open(&dir).expect("建库失败");

        let a = db.create_group("甲", None).unwrap();
        let b = db.create_group("乙", None).unwrap();
        let c = db.create_group("丙", None).unwrap();

        // 少一个：别的窗口刚删了分组
        assert!(db.reorder_groups(&[a.id, b.id]).is_err(), "漏掉 id 必须报错");
        // 多一个：别的窗口刚建了分组
        assert!(db.reorder_groups(&[a.id, b.id, c.id, 9999]).is_err(), "多出不存在的 id 必须报错");
        // 重复 id：会让按 index 写入时产生并列值
        assert!(db.reorder_groups(&[a.id, a.id, b.id]).is_err(), "重复 id 必须报错");

        // 失败后数据必须原封不动，不能留下半截写入
        let names: Vec<String> = db.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert_eq!(names, vec!["甲", "乙", "丙"], "被拒绝的请求不该改动任何 sort_order");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------------- F22 全文索引 ----------------

    /// 建条目并把它的正文塞进去。`seed_item` 的文本是 content + plain_text 双写。
    fn search_ids(db: &Db, q: &str) -> Vec<i64> {
        db.list(Some(q), None, 100).unwrap().into_iter().map(|i| i.id).collect()
    }

    #[test]
    fn 全文索引能搜到子串() {
        let dir = temp_dir("fts-sub");
        let db = Db::open(&dir).expect("建库失败");
        let a = seed_item(&db, "关于 Q4 预算的回复，重点是研发投入拆成人力与设备两块");
        let b = seed_item(&db, "下周的产品评审会议纪要");

        assert_eq!(search_ids(&db, "预算"), vec![a], "两字中文查询走 LIKE 回退");
        assert_eq!(search_ids(&db, "人力与设备"), vec![a]);
        assert_eq!(search_ids(&db, "会议纪要"), vec![b]);
        assert!(search_ids(&db, "不存在的词").is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 删掉条目后索引必须跟着删 —— 外部内容表不会自动同步，
    /// 漏了 DELETE 触发器就会搜出已经不在的条目。
    #[test]
    fn 删除条目后索引不再命中() {
        let dir = temp_dir("fts-del");
        let db = Db::open(&dir).expect("建库失败");
        let a = seed_item(&db, "临时记录：这是一段会被删掉的测试文本");
        assert_eq!(search_ids(&db, "会被删掉"), vec![a]);

        db.delete(a).expect("删除失败");
        assert!(search_ids(&db, "会被删掉").is_empty(), "索引里不该留下已删条目的切片");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 清空历史（批量 DELETE）也要同步。
    #[test]
    fn 清空历史后索引不再命中() {
        let dir = temp_dir("fts-clear");
        let db = Db::open(&dir).expect("建库失败");
        seed_item(&db, "批量清理的测试文本，关键词是紫色独角兽");
        assert_eq!(search_ids(&db, "紫色独角兽").len(), 1);

        db.clear(true).expect("清空失败");
        assert!(search_ids(&db, "紫色独角兽").is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 老库升级：`items` 里已有数据、触发器还没建。重建一次之后必须能搜到。
    #[test]
    fn 老库重建索引后能搜到已有条目() {
        let dir = temp_dir("fts-rebuild");
        let db = Db::open(&dir).expect("建库失败");
        let a = seed_item(&db, "升级前就存在的记录，关键词是琥珀色犀牛");

        // 模拟「索引没建过」：清掉版本标记 + 清空索引，再跑一次 ensure_fts
        {
            let conn = db.conn();
            conn.execute("DELETE FROM schema_meta WHERE key = 'fts_version'", []).unwrap();
            conn.execute("INSERT INTO items_fts(items_fts) VALUES('delete-all')", []).unwrap();
        }
        assert!(search_ids(&db, "琥珀色犀牛").is_empty(), "清空索引后应该搜不到");

        db.ensure_fts().expect("重建失败");
        assert_eq!(search_ids(&db, "琥珀色犀牛"), vec![a], "重建后应该能搜到");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 重建是幂等的，而且第二次会被版本号挡住（不重复做无用功）。
    #[test]
    fn 重建索引幂等() {
        let dir = temp_dir("fts-idem");
        let db = Db::open(&dir).expect("建库失败");
        let a = seed_item(&db, "幂等性测试文本，关键词是青色鲸鱼");
        db.ensure_fts().unwrap();
        db.ensure_fts().unwrap();
        assert_eq!(search_ids(&db, "青色鲸鱼"), vec![a]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 用户在搜索框里敲的可能是 FTS5 的语法字符。不包成短语就会直接语法报错。
    #[test]
    fn 查询里的_fts_语法字符不会炸() {
        let dir = temp_dir("fts-syntax");
        let db = Db::open(&dir).expect("建库失败");
        seed_item(&db, "一条普通记录，用于确认特殊字符不会让查询失败");

        for q in ["a AND b", "foo -bar", "x*", "\"引号\"", "^start", "NOT thing", "a:b"] {
            // 只要不返回 Err 就算过 —— 搜不到是正常结果
            let _ = db.list(Some(q), None, 10).expect("特殊字符不该让查询失败");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 搜索要同时覆盖来源应用名（老行为），FTS 索引里也带了这两列。
    #[test]
    fn 全文索引覆盖来源应用名() {
        let dir = temp_dir("fts-source");
        let db = Db::open(&dir).expect("建库失败");
        let new = NewItem {
            kind: ItemType::Text,
            content: Some("一条来自截图工具的记录".into()),
            plain_text: Some("一条来自截图工具的记录".into()),
            source_app: Some("SnippingTool.exe".into()),
            ..Default::default()
        };
        let hash = crate::storage::normalize::hash_text("一条来自截图工具的记录");
        let id = db.upsert(&new, &hash, 1_000, 1_000).unwrap().0.id;

        assert_eq!(search_ids(&db, "SnippingTool"), vec![id]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 短语转义把引号加倍() {
        assert_eq!(fts_phrase("hello"), "\"hello\"");
        assert_eq!(fts_phrase("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    /// 搜索框是大小写不敏感的，而且两条路径（长查询走 FTS、短查询走 LIKE）
    /// 必须给出一致的答案 —— 否则用户会看到「多打一个字结果就变了」。
    #[test]
    fn 搜索大小写不敏感且两条路径一致() {
        let dir = temp_dir("fts-case");
        let db = Db::open(&dir).expect("建库失败");
        let new = NewItem {
            kind: ItemType::Text,
            content: Some("Plico Release Notes".into()),
            plain_text: Some("Plico Release Notes".into()),
            ..Default::default()
        };
        let hash = crate::storage::normalize::hash_text("Plico Release Notes");
        let id = db.upsert(&new, &hash, 1_000, 1_000).unwrap().0.id;

        // 2 字符 → LIKE 回退
        assert_eq!(search_ids(&db, "re"), vec![id]);
        assert_eq!(search_ids(&db, "RE"), vec![id]);
        // 5 字符 → FTS5
        assert_eq!(search_ids(&db, "release"), vec![id]);
        assert_eq!(search_ids(&db, "RELEASE"), vec![id]);
        assert_eq!(search_ids(&db, "release notes"), vec![id], "跨词的空格短语也要命中");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 删除标签只解绑条目不影响条目本身() {
        let dir = temp_dir("tag-delete");
        let db = Db::open(&dir).expect("建库失败");

        let a = seed_item(&db, "条目");
        db.assign_item_tag(a, "待删").unwrap();
        let id = db.list_tag_stats().unwrap()[0].id;

        db.delete_tag(id).expect("删除应该成功");
        assert!(db.list_tag_stats().unwrap().is_empty());
        assert!(db.item_tags(a).unwrap().is_empty(), "外键 CASCADE 应该清掉关联行");
        // 条目本身还在 —— 删标签不该顺手把内容也带走
        assert_eq!(db.get(a).expect("条目应该还在").id, a);

        assert!(db.delete_tag(id).is_err(), "删不存在的标签要报错而不是静默成功");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS items (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  "type"         TEXT NOT NULL,
  content        TEXT,
  html_content   TEXT,
  plain_text     TEXT,
  image_path     TEXT,
  thumb_path     TEXT,
  content_hash   TEXT NOT NULL UNIQUE,
  source_app     TEXT,
  source_title   TEXT,
  group_id       INTEGER,
  pinned         INTEGER NOT NULL DEFAULT 0,
  created_at     INTEGER NOT NULL,
  last_copied_at INTEGER NOT NULL,
  last_used_at   INTEGER
);

CREATE INDEX IF NOT EXISTS idx_items_order ON items (pinned DESC, last_copied_at DESC);
CREATE INDEX IF NOT EXISTS idx_items_type  ON items ("type");

CREATE TABLE IF NOT EXISTS tags (
  id   INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS item_tags (
  item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  tag_id  INTEGER NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
  PRIMARY KEY (item_id, tag_id)
);

CREATE TABLE IF NOT EXISTS groups (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  color      TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS snippets (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  title      TEXT NOT NULL,
  content    TEXT NOT NULL,
  tags       TEXT,
  shortcut   TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rules (
  id      INTEGER PRIMARY KEY AUTOINCREMENT,
  kind    TEXT NOT NULL,
  value   TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT
);

CREATE TABLE IF NOT EXISTS schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT
);

-- ---------- F22 全文索引 ----------
--
-- 用 `trigram` 而不是默认的 `unicode61`：默认分词器会把一整串中文当成一个
-- token，`剪贴板历史` 只能整串命中，搜「历史」找不到。trigram 按三字符切片，
-- 天然支持子串匹配，中文英文一视同仁（代价是模式必须 ≥3 字符，短查询由
-- `list` 里的 LIKE 回退兜住）。
--
-- `content='items'` 是「外部内容表」：索引本身不存原文，只存切片，原文始终
-- 以 items 为准。这样能省一份存储，代价是增删改必须由触发器同步 ——
-- 少了触发器索引就会悄悄变旧，而且是「搜不到」这种不报错的坏法。
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
  plain_text, content, source_app, source_title,
  content='items',
  content_rowid='id',
  tokenize='trigram'
);

CREATE TRIGGER IF NOT EXISTS items_fts_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid, plain_text, content, source_app, source_title)
  VALUES (new.id, new.plain_text, new.content, new.source_app, new.source_title);
END;

-- 外部内容表的删除要把**旧值**喂给 'delete' 命令，索引才知道该摘掉哪些切片
CREATE TRIGGER IF NOT EXISTS items_fts_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, plain_text, content, source_app, source_title)
  VALUES ('delete', old.id, old.plain_text, old.content, old.source_app, old.source_title);
END;

CREATE TRIGGER IF NOT EXISTS items_fts_au AFTER UPDATE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, plain_text, content, source_app, source_title)
  VALUES ('delete', old.id, old.plain_text, old.content, old.source_app, old.source_title);
  INSERT INTO items_fts(rowid, plain_text, content, source_app, source_title)
  VALUES (new.id, new.plain_text, new.content, new.source_app, new.source_title);
END;
"#;
