use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};

#[derive(Clone, Copy)]
pub(crate) enum ExtensionMetaStatus {
    Normal = 0,
    UPDATE = 1,
    Invalid = -1,
}

impl Default for ExtensionMetaStatus {
    fn default() -> Self {
        ExtensionMetaStatus::Normal
    }
}

impl From<i32> for ExtensionMetaStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => ExtensionMetaStatus::Normal,
            1 => ExtensionMetaStatus::UPDATE,
            -1 | _ => ExtensionMetaStatus::Invalid,
        }
    }
}

#[derive(Default)]
pub(crate) struct ExtensionMeta {
    pub id: i32,
    pub md5: String,
    pub description: String,
    pub alias: String,
    pub file_name: String,
    pub status: ExtensionMetaStatus,
}

pub(crate) struct DB {
    connect: Connection,
}

impl DB {
    pub(crate) fn new(file: impl AsRef<Path>) -> Result<DB> {
        let connect = Connection::open(file)?;
        Ok(Self { connect })
    }

    pub(crate) fn create_schema(&mut self) -> Result<()> {
        let schema_sql = r#"
            create table if not exists extension_meta (
                id INTEGER PRIMARY KEY,
                alias TEXT NOT NULL UNIQUE,
                md5 TEXT NOT NULL,
                filename TEXT NOT NULL,
                status INTEGER DEFAULT 0,
                description TEXT NOT NULL
            );
            UPDATE sqlite_sequence SET seq = 0 WHERE name = 'extension_meta';
        "#;
        self.connect.execute(schema_sql, ())?;
        Ok(())
    }

    pub(crate) fn get_extension_by_alias(&self, alias: &str) -> Result<Option<ExtensionMeta>> {
        let query_sql = r#"
            select id, alias, md5, filename, description, status
            from extension_meta where status = 0 and alias=?1;
        "#;
        Ok(self.connect.query_row(query_sql, &[alias], |row| {
            let id = row.get(0)?;
            let alias = row.get(1)?;
            let md5 = row.get(2)?;
            let file_name = row.get(3)?;
            let description = row.get(4)?;
            let status = row.get::<usize, i32>(5)?.into();
            Ok(ExtensionMeta {
                id,
                md5,
                alias,
                status,
                file_name,
                description,
            })
        }).optional()?)
    }

    pub(crate) fn list_extensions(&self) -> Result<Vec<ExtensionMeta>> {
        let query_sql = r#"
            select id, alias, md5, filename, description, status
            from extension_meta where status = 0;
        "#;
        let mut stmt = self.connect.prepare(query_sql)?;
        Ok(stmt
            .query_map([], |row| {
                let id = row.get(0)?;
                let alias = row.get(1)?;
                let md5 = row.get(2)?;
                let file_name = row.get(3)?;
                let description = row.get(4)?;
                let status = row.get::<usize, i32>(5)?.into();
                Ok(ExtensionMeta {
                    id,
                    md5,
                    alias,
                    status,
                    file_name,
                    description,
                })
            })
            .map(|rows| rows.filter_map(|row| row.ok()).collect::<Vec<_>>())?)
    }

    pub(crate) fn save_extensions(&mut self, exts: &Vec<ExtensionMeta>) -> Result<()> {
        for meta in exts.iter() {
            match meta.status {
                ExtensionMetaStatus::Normal => self.insert_extension_meta(meta),
                ExtensionMetaStatus::UPDATE => self.update_extension_meta(meta),
                ExtensionMetaStatus::Invalid => self.delete_extension_meta(meta),
            }?;
        }
        Ok(())
    }

    pub(crate) fn delete_extension_meta(&mut self, meta: &ExtensionMeta) -> Result<()> {
        let update_sql = r#"
            delete from extension_meta
            where id = ?1 and filename=?2
        "#;
        self.connect.execute(
            update_sql,
            (
                &meta.id,
                &meta.file_name,
            )
        )?;
        Ok(())
    }

    pub(crate) fn update_extension_meta(&mut self, meta: &ExtensionMeta) -> Result<()> {
        let update_sql = r#"
            update extension_meta
            set alias=?1, md5=?2, description=?3
            where filename = ?4
        "#;
        self.connect.execute(
            update_sql,
            (
                &meta.alias,
                &meta.md5,
                &meta.description,
                &meta.file_name,
            )
        )?;
        Ok(())
    }

    pub(crate) fn insert_extension_meta(&mut self, meta: &ExtensionMeta) -> Result<()> {
        let insert_sql = r#"
            insert into 
            extension_meta(alias, md5, filename, description, status)
            values(?1,?2,?3,?4,?5);
        "#;
        self.connect.execute(
            insert_sql,
            (
                &meta.alias,
                &meta.md5,
                &meta.file_name,
                &meta.description,
                meta.status as i32,
            ),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_create_insert() -> Result<()> {
        let mut db = DB {
            connect: Connection::open_in_memory()?,
        };
        let description: String = "123456".into();
        let md5: String = "0x1123456".into();
        let file_name: String = "file_name".into();
        let alias: String = "file".into();
        db.create_schema()?;
        let meta = ExtensionMeta {
            md5: md5.clone(),
            file_name: file_name.clone(),
            alias: alias.clone(),
            description: description.clone(),
            ..Default::default()
        };
        db.insert_extension_meta(&meta)?;
        let rs = db.list_extensions()?;
        assert!(rs.len() == 1);
        assert_eq!(rs[0].id, 1);
        assert_eq!(rs[0].description, description);
        assert_eq!(rs[0].md5, md5);
        assert_eq!(rs[0].file_name, file_name);
        assert_eq!(rs[0].alias, alias);
        let rs = db.get_extension_by_alias(&alias)?;
        rs.map(|rs| {
            assert_eq!(rs.id, 1);
            assert_eq!(rs.description, description);
            assert_eq!(rs.md5, md5);
            assert_eq!(rs.file_name, file_name);
            assert_eq!(rs.alias, alias);
        });
        Ok(())
    }
}
