use serde::{Serialize, Deserialize};
use chrono::{NaiveDateTime};

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionDto {
    pub version_id: i32,
    pub database_version: String,
    pub database_version_info: String,
    pub application_version: String,
    pub application_version_info: String,
    pub date_created: NaiveDateTime,
    pub date_modified: NaiveDateTime,
}
