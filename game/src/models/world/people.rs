use crate::models::new_uuid_string;
use tabled::Tabled;

use super::super::Insertable;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::{EnumString, EnumVariantNames};

#[derive(Serialize, Deserialize, Debug, EnumString, EnumVariantNames, Clone)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Sex {
    Male,
    Female,
}

impl Display for Sex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Female => write!(f, "female"),
            Self::Male => write!(f, "male"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, EnumString, EnumVariantNames, Clone)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    NonBinary,
}

impl Display for Gender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gender::Female => write!(f, "woman"),
            Gender::Male => write!(f, "man"),
            Gender::NonBinary => write!(f, "nonbinary"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Person {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _id: Option<String>,

    pub name: String,
    pub description: String,
    pub age: u32,
    pub residence: String,
    pub current_activity: String,
    pub occupation: String,
    pub race: String,
    pub sex: Sex,
    pub gender: Gender,
}

impl_insertable!(Person);

impl Default for Person {
    fn default() -> Self {
        Person {
            _key: Some(new_uuid_string()),
            _id: None,
            name: "".to_string(),
            description: "".to_string(),
            age: 0,
            residence: "".to_string(),
            current_activity: "".to_string(),
            occupation: "".to_string(),
            race: "".to_string(),
            sex: Sex::Male,
            gender: Gender::Male,
        }
    }
}
