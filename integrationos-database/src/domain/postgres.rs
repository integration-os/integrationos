use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use integrationos_domain::database::DatabaseConnectionConfig;
use serde::ser::Error;
use serde::Serializer;
use serde_json::Value;
use sqlx::postgres::PgValueRef;
use sqlx::types::{Decimal, Uuid};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    PgPool,
};
use sqlx::{Decode, Postgres, TypeInfo, ValueRef as PgValue};
use std::time::Duration;

#[derive(Clone)]
pub struct PostgresDatabaseConnection {
    pub pool: PgPool,
}

impl PostgresDatabaseConnection {
    pub async fn new(configuration: &DatabaseConnectionConfig) -> Result<Self> {
        let options = PgConnectOptions::new()
            .username(&configuration.postgres_config.postgres_username)
            .password(&configuration.postgres_config.postgres_password)
            .host(&configuration.postgres_config.postgres_host)
            .ssl_mode(if configuration.postgres_config.postgres_ssl {
                PgSslMode::Require
            } else {
                PgSslMode::Disable
            })
            .port(configuration.postgres_config.postgres_port);

        let pool = PgPoolOptions::new()
            .max_connections(configuration.postgres_config.postgres_pool_size)
            .acquire_timeout(Duration::from_millis(
                configuration.postgres_config.postgres_timeout,
            ))
            .connect_with(options.database(&configuration.postgres_config.postgres_name))
            .await?;

        Ok(Self { pool })
    }
}

pub fn serialize_pgvalueref<S>(value: &PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if value.is_null() {
        return s.serialize_none();
    }
    let value = value.clone();
    let info = value.type_info();

    let name = info.name();
    match name.to_lowercase().as_str() {
        "bool" => serialize_bool(value, s),
        "int2" => serialize_i16(value, s),
        "int4" => serialize_i32(value, s),
        "int8" => serialize_i64(value, s),
        "float4" => serialize_f32(value, s),
        "float8" => serialize_f64(value, s),
        "numeric" => serialize_numeric(value, s),
        "char" | "varchar" | "text" | "\"char\"" | "name" => serialize_string(value, s),
        "bytea" => serialize_bytea(value, s),
        "json" | "jsonb" => serialize_json(value, s),
        "timestamp" => serialize_timestamp(value, s),
        "timestamptz" => serialize_timestamptz(value, s),
        "date" => serialize_date(value, s),
        "time" => serialize_time(value, s),
        "uuid" => serialize_uuid(value, s),
        _ => Err(Error::custom(format!(
            "This type is not supported, please contact platform: {}",
            name.to_lowercase().as_str()
        ))),
    }
}

fn serialize_bool<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<bool, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_bool(val),
        Err(e) => Err(Error::custom(format!("Failed to decode BOOL: {}", e))),
    }
}

fn serialize_i16<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<i16, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_i16(val),
        Err(e) => Err(Error::custom(format!("Failed to decode INT2: {}", e))),
    }
}

fn serialize_i32<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<i32, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_i32(val),

        Err(e) => Err(Error::custom(format!("Failed to decode INT4: {}", e))),
    }
}

fn serialize_i64<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<i64, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_i64(val),
        Err(e) => Err(Error::custom(format!("Failed to decode INT8: {}", e))),
    }
}

fn serialize_f32<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<f32, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_f32(val),
        Err(e) => Err(Error::custom(format!("Failed to decode FLOAT4: {}", e))),
    }
}

fn serialize_f64<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<f64, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_f64(val),
        Err(e) => Err(Error::custom(format!("Failed to decode FLOAT8: {}", e))),
    }
}

fn serialize_numeric<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<Decimal, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val.to_string()),
        Err(e) => Err(Error::custom(format!("Failed to decode NUMERIC: {}", e))),
    }
}

fn serialize_string<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<String, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val),
        Err(e) => Err(Error::custom(format!("Failed to decode STRING: {}", e))),
    }
}

fn serialize_bytea<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<Vec<u8>, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_some(&val),
        Err(e) => Err(Error::custom(format!("Failed to decode BYTEA: {}", e))),
    }
}

fn serialize_json<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<Value, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_some(&val),
        Err(e) => Err(Error::custom(format!("Failed to decode JSON: {}", e))),
    }
}

fn serialize_timestamp<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<NaiveDateTime, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val.format("%Y-%m-%dT%H:%M:%S.%f").to_string()),
        Err(e) => Err(Error::custom(format!("Failed to decode TIMESTAMP: {}", e))),
    }
}

fn serialize_timestamptz<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<DateTime<Utc>, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val.to_rfc3339()),
        Err(e) => Err(Error::custom(format!(
            "Failed to decode TIMESTAMPTZ: {}",
            e
        ))),
    }
}

fn serialize_date<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<NaiveDate, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val.to_string()),
        Err(e) => Err(Error::custom(format!("Failed to decode DATE: {}", e))),
    }
}

fn serialize_time<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<NaiveTime, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val.to_string()),
        Err(e) => Err(Error::custom(format!("Failed to decode TIME: {}", e))),
    }
}

fn serialize_uuid<S>(value: PgValueRef, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let v: Result<Uuid, _> = Decode::<Postgres>::decode(value);
    match v {
        Ok(val) => s.serialize_str(&val.to_string()),
        Err(e) => Err(Error::custom(format!("Failed to decode UUID: {}", e))),
    }
}
