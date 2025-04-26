use diesel::{Connection, PgConnection};

pub trait Connector {
    fn connect(&self) -> PgConnection;
}

impl Connector for &super::DatabaseConfiguration {
    fn connect(&self) -> PgConnection {
        diesel::pg::PgConnection::establish(self.url.as_str()).expect("DB to connect")
    }
}
