use crate::models::commands::{
    CommandEvent, CommandEventType, EventConversionFailure, ParsedCommand, RawCommandExecution,
};
use crate::models::world::items::Item;
use crate::models::world::people::Person;
use crate::models::world::scenes::{Exit, Prop, Scene, Stage};
use crate::models::Insertable;
use itertools::Itertools;

use tabled::settings::Style;
use tabled::{Table, Tabled};

const UNKNOWN: &'static str = "unknown";
const PERSON: &'static str = "person";
const ITEM: &'static str = "item";
const PROP: &'static str = "prop";
const NO_KEY: &'static str = "n/a";

#[derive(Tabled)]
pub struct EntityTableRow<'a> {
    name: &'a str,
    #[tabled(rename = "type")]
    entity_type: &'a str,
    key: &'a str,
}

impl<'a> From<&'a Person> for EntityTableRow<'a> {
    fn from(value: &'a Person) -> Self {
        EntityTableRow {
            name: &value.name,
            key: value.key().unwrap_or(UNKNOWN),
            entity_type: PERSON,
        }
    }
}

impl<'a> From<&'a Item> for EntityTableRow<'a> {
    fn from(value: &'a Item) -> Self {
        EntityTableRow {
            name: &value.name,
            key: value.key().unwrap_or(UNKNOWN),
            entity_type: ITEM,
        }
    }
}

impl<'a> From<&'a Prop> for EntityTableRow<'a> {
    fn from(value: &'a Prop) -> Self {
        EntityTableRow {
            name: &value.name,
            entity_type: PROP,
            key: NO_KEY,
        }
    }
}

#[derive(Tabled)]
pub struct ExitTableRow<'a> {
    pub name: &'a str,
    pub direction: &'a str,
    pub scene_key: &'a str,
    pub region: &'a str,
}

impl<'a> From<&'a Exit> for ExitTableRow<'a> {
    fn from(value: &'a Exit) -> Self {
        ExitTableRow {
            name: &value.name,
            direction: &value.direction,
            scene_key: &value.scene_key,
            region: &value.region,
        }
    }
}

pub(super) fn entity_table(stage: &Stage) -> Table {
    let people = stage.people.iter().map_into::<EntityTableRow>();
    let items = stage.items.iter().map_into::<EntityTableRow>();
    let props = stage.scene.props.iter().map_into::<EntityTableRow>();
    let entities = people.chain(items).chain(props);

    let mut entities_table = Table::new(entities);
    entities_table.with(Style::markdown());

    entities_table
}

pub(super) fn exit_table<'a, I>(exits: I) -> Table
where
    I: IntoIterator<Item = &'a Exit>,
{
    let exits = exits.into_iter();
    let mut table = Table::new(exits.map_into::<ExitTableRow>());
    table.with(Style::markdown());
    table
}
