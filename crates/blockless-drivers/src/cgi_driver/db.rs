use rusqlite::Connection;
use anyhow::Result;

pub(crate) struct ExtensionMeta {
    pub md5: String,
    pub description: String,
    pub alias: String,
}

struct DB {
    connect: Connection
}

impl DB {
    pub(crate) fn new(file: &str) -> Result<DB> {
        let connect = Connection::open(file)?;
        Ok(Self{
            connect
        })
    }

    pub(crate) fn create_schema() -> Result<()> {

        Ok(())
    }

    pub(crate) fn save_extension_meta(
        self: &mut DB, 
        meta: &ExtensionMeta
    ) -> Result<()> {
        let insert_sql = r#"
            insert into 
            extension_meta(alias, md5, description)
            values(?1,?2,?3);
        "#;
        self.connect.execute(
            insert_sql, 
            (&meta.alias, &meta.md5, &meta.description)
        )?;
        Ok(())
    }
}

