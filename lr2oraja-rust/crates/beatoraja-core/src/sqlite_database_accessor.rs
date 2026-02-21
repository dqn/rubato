use rusqlite::{Connection, params};

// SQLite column definition
#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub type_name: String,
    pub notnull: i32,
    pub pk: i32,
    pub defaultval: Option<String>,
}

impl Column {
    pub fn new(name: &str, type_name: &str) -> Self {
        Self::with_pk(name, type_name, 0, 0)
    }

    pub fn with_pk(name: &str, type_name: &str, notnull: i32, pk: i32) -> Self {
        Self {
            name: name.to_string(),
            type_name: type_name.to_string(),
            notnull,
            pk,
            defaultval: None,
        }
    }

    pub fn with_default(
        name: &str,
        type_name: &str,
        notnull: i32,
        pk: i32,
        defaultval: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            type_name: type_name.to_string(),
            notnull,
            pk,
            defaultval: Some(defaultval.to_string()),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> &str {
        &self.type_name
    }

    pub fn get_notnull(&self) -> i32 {
        self.notnull
    }

    pub fn get_pk(&self) -> i32 {
        self.pk
    }

    pub fn get_defaultval(&self) -> Option<&str> {
        self.defaultval.as_deref()
    }
}

// SQLite table definition
#[derive(Clone, Debug)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn new(name: &str, columns: Vec<Column>) -> Self {
        Self {
            name: name.to_string(),
            columns,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_columns(&self) -> &[Column] {
        &self.columns
    }
}

/// SQLite database accessor base.
/// Java's abstract class SQLiteDatabaseAccessor is translated as a struct
/// with methods that subclasses use via composition.
pub struct SQLiteDatabaseAccessor {
    tables: Vec<Table>,
}

impl SQLiteDatabaseAccessor {
    pub fn new(tables: Vec<Table>) -> Self {
        Self { tables }
    }

    /// Create tables and add missing columns.
    /// Translated from Java: validate(QueryRunner qr)
    pub fn validate(&self, conn: &Connection) -> anyhow::Result<()> {
        for table in &self.tables {
            let mut pk: Vec<&Column> = Vec::new();

            // Check if the table exists
            let table_exists: bool = {
                let mut stmt =
                    conn.prepare("SELECT * FROM sqlite_master WHERE name = ? and type='table';")?;
                let rows = stmt.query_map(params![table.get_name()], |_row| Ok(()))?;
                rows.count() > 0
            };

            if !table_exists {
                let mut sql = format!("CREATE TABLE [{}] (", table.get_name());
                let mut comma = false;
                for column in table.get_columns() {
                    if comma {
                        sql.push(',');
                    }
                    sql.push('[');
                    sql.push_str(column.get_name());
                    sql.push_str("] ");
                    sql.push_str(column.get_type());
                    if column.get_notnull() == 1 {
                        sql.push_str(" NOT NULL");
                    }
                    if let Some(dv) = column.get_defaultval()
                        && !dv.is_empty()
                    {
                        sql.push_str(" DEFAULT ");
                        sql.push_str(dv);
                    }
                    comma = true;
                    if column.get_pk() == 1 {
                        pk.push(column);
                    }
                }

                if !pk.is_empty() {
                    sql.push_str(",PRIMARY KEY(");
                    let mut comma2 = false;
                    for column in &pk {
                        if comma2 {
                            sql.push(',');
                        }
                        sql.push_str(column.get_name());
                        comma2 = true;
                    }
                    sql.push(')');
                }
                sql.push_str(");");
                conn.execute(&sql, [])?;
            }

            // Check for missing columns and add them
            let existing_columns: Vec<String> = {
                let mut stmt =
                    conn.prepare(&format!("PRAGMA table_info('{}');", table.get_name()))?;
                let rows = stmt.query_map([], |row| {
                    let name: String = row.get(1)?;
                    Ok(name)
                })?;
                rows.filter_map(|r| r.ok()).collect()
            };

            let mut adds: Vec<&Column> = table.get_columns().iter().collect();
            for existing_name in &existing_columns {
                adds.retain(|col| col.get_name() != existing_name.as_str());
            }

            for add in adds {
                let mut sql = format!(
                    "ALTER TABLE {} ADD COLUMN [{}] {}",
                    table.get_name(),
                    add.get_name(),
                    add.get_type()
                );
                if add.get_notnull() == 1 {
                    sql.push_str(" NOT NULL");
                }
                if let Some(dv) = add.get_defaultval()
                    && !dv.is_empty()
                {
                    sql.push_str(" DEFAULT ");
                    sql.push_str(dv);
                }
                conn.execute(&sql, [])?;
            }
        }
        Ok(())
    }

    /// Insert or replace a row using column values provided by a closure.
    /// The closure maps column name -> rusqlite Value.
    pub fn insert_with_values(
        &self,
        conn: &Connection,
        tablename: &str,
        get_value: &dyn Fn(&str) -> rusqlite::types::Value,
    ) -> anyhow::Result<()> {
        let columns = match self.get_columns_for_table(tablename) {
            Some(c) => c,
            None => return Ok(()),
        };

        let mut sql = format!("INSERT OR REPLACE INTO {} (", tablename);
        let mut comma = false;
        for column in columns {
            if comma {
                sql.push(',');
            }
            sql.push_str(column.get_name());
            comma = true;
        }
        sql.push_str(") VALUES(");

        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        comma = false;
        for column in columns {
            if comma {
                sql.push_str(",?");
            } else {
                sql.push('?');
            }
            comma = true;
            params_vec.push(get_value(column.get_name()));
        }
        sql.push_str(");");

        conn.execute(&sql, rusqlite::params_from_iter(params_vec.iter()))?;
        Ok(())
    }

    pub fn get_columns_for_table(&self, tablename: &str) -> Option<&[Column]> {
        for table in &self.tables {
            if table.get_name() == tablename {
                return Some(table.get_columns());
            }
        }
        None
    }

    pub fn get_tables(&self) -> &[Table] {
        &self.tables
    }
}
