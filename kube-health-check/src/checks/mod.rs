//! Built-in and custom dependency health checkers (FR-002 / FR-005).

mod custom;
mod tcp;

#[cfg(feature = "elasticsearch")]
mod elasticsearch;
#[cfg(feature = "kafka")]
mod kafka;
#[cfg(feature = "mongodb")]
mod mongodb;
#[cfg(feature = "mssql")]
mod mssql;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "nats")]
mod nats;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "rabbitmq")]
mod rabbitmq;
#[cfg(feature = "redis")]
mod redis_check;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use custom::CustomCheck;
pub use tcp::TcpCheck;

#[cfg(feature = "elasticsearch")]
pub use elasticsearch::ElasticsearchCheck;
#[cfg(feature = "kafka")]
pub use kafka::KafkaCheck;
#[cfg(feature = "mongodb")]
pub use mongodb::{mongodb, MongoCheck};
#[cfg(feature = "mssql")]
pub use mssql::{mssql, MssqlCheck};
#[cfg(feature = "mysql")]
pub use mysql::MysqlCheck;
#[cfg(feature = "nats")]
pub use nats::NatsCheck;
#[cfg(feature = "postgres")]
pub use postgres::PostgresCheck;
#[cfg(feature = "rabbitmq")]
pub use rabbitmq::RabbitMqCheck;
#[cfg(feature = "redis")]
pub use redis_check::RedisCheck;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteCheck;
