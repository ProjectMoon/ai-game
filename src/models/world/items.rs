use crate::models::new_uuid_string;
use serde::{Deserialize, Serialize};
use strum::{EnumString, EnumVariantNames, Display};

use super::super::Insertable;

#[derive(Serialize, Deserialize, Debug, EnumString, EnumVariantNames, Clone, Display)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Weapon,
    Armor,
    Accessory,
    Other,
}

#[derive(Serialize, Deserialize, Debug, EnumString, EnumVariantNames, Clone)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Mythic,
    Legendary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Item {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub _id: Option<String>,

    pub name: String,
    pub description: String,
    pub category: Category,
    pub rarity: Rarity,
    pub attributes: Vec<String>,
    pub secret_attributes: Vec<String>,
}

impl_insertable!(Item);

impl Default for Item {
    fn default() -> Self {
        Self {
            _key: Some(new_uuid_string()),
            _id: None,
            name: "".to_string(),
            description: "".to_string(),
            category: Category::Other,
            rarity: Rarity::Common,
            attributes: vec![],
            secret_attributes: vec![],
        }
    }
}
