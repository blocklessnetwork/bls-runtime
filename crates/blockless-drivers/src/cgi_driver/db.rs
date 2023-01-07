use anyhow::Result;
use rusqlite::Connection;

#[derive(Clone, Copy)]
pub(crate) enum ExtensionMetaStatus {
    Normal = 0,
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
            -1|_ => ExtensionMetaStatus::Normal,
        }
    }
}

#[derive(Default)]
pub(crate) struct ExtensionMeta {
    pub id: i32,
    pub md5: String,
    pub description: String,
    pub alias: String,
    pub path: String,
    pub status: ExtensionMetaStatus,
}

pub(crate) struct DB {
    connect: Connection,
}

impl DB {
    pub(crate) fn new(file: &str) -> Result<DB> {
        let connect = Connection::open(file)?;
        Ok(Self { connect })
    }

    pub(crate) fn create_schema(self: &mut DB) -> Result<()> {
        let schema_sql = r#"
            create table if not exists extension_meta (
                id INTEGER PRIMARY KEY,
                alias TEXT NOT NULL,
                md5 TEXT NOT NULL,
                path TEXT NOT NULL,
                status INTEGER DEFAULT 0,
                description TEXT NOT NULL
            );
        "#;
        self.connect.execute(schema_sql, ())?;
        Ok(())
    }

    pub(crate) fn list_extensions(self: &mut DB) -> Result<Vec<ExtensionMeta>> {
        let query_sql = r#"
            select id, alias, md5, path, description, status
            from extension_meta where status = 0;
        "#;
        let mut stmt = self.connect.prepare(query_sql)?;
        Ok(stmt
            .query_map([], |row| {
                let id = row.get(0)?; 
                let alias = row.get(1)?; 
                let md5 = row.get(2)?; 
                let path = row.get(3)?; 
                let description = row.get(4)?; 
                let status = row.get::<usize, i32>(5)?.into(); 
                Ok(ExtensionMeta {
                    id,
                    alias,
                    md5,
                    path,
                    description,
                    status,
                })
            })
            .map(|rows| {
                rows.filter_map(|row| row.ok())
                    .collect::<Vec<_>>()
            })?)
    }

    pub(crate) fn save_extension_meta(self: &mut DB, meta: &ExtensionMeta) -> Result<()> {
        let insert_sql = r#"
            insert into 
            extension_meta(alias, md5, path, description, status)
            values(?1,?2,?3,?4,?5);
        "#;
        self.connect.execute(
            insert_sql,
            (&meta.alias, &meta.md5, &meta.path, &meta.description, meta.status as i32),
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
        let path: String = "path".into();
        let alias: String = "file".into();
        db.create_schema()?;
        let meta  = ExtensionMeta {
            md5: md5.clone(),
            path: path.clone(),
            alias: alias.clone(),
            description: description.clone(),
            ..Default::default()
        };
        db.save_extension_meta(&meta)?;
        let rs = db.list_extensions()?;
        assert!(rs.len() == 1);
        assert_eq!(rs[0].description, description);
        assert_eq!(rs[0].md5, md5);
        assert_eq!(rs[0].path, path);
        assert_eq!(rs[0].alias, alias);
        Ok(())   
    }
}