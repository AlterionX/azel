use diesel::{Connection, ConnectionError, PgConnection};
use diesel_async::{AsyncConnection, AsyncPgConnection};

treeerror::treeerror! {
    #[derive(Debug)]
    DbError {
        Connection(ConnectionError),
        Query(diesel::result::Error),
    },
}

pub type DbResult<T> = Result<T, DbError>;

pub trait Connector {
    fn connect(&self) -> Result<PgConnection, ConnectionError>;
    fn async_connect(&self) -> impl Future<Output = Result<AsyncPgConnection, ConnectionError>> + Send;
}

impl Connector for &super::DatabaseConfiguration {
    fn connect(&self) -> Result<PgConnection, ConnectionError> {
        PgConnection::establish(self.url.as_str())
    }

    async fn async_connect(&self) -> Result<AsyncPgConnection, ConnectionError> {
        AsyncPgConnection::establish(self.url.as_str()).await
    }
}
