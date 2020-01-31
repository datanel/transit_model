// Copyright (C) 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

//! The `transit_model` crate proposes a model to manage transit data.
//! It can import and export data from [GTFS](http://gtfs.org/) and
//! [NTFS](https://github.com/CanalTP/ntfs-specification/blob/master/ntfs_fr.md).

use crate::model::GetCorresponding;
use crate::{
    objects::{Calendar, VehicleJourney},
    Model, Result,
};
use failure::bail;
use std::collections::{BTreeSet, HashMap, HashSet};
use transit_model_collection::{CollectionWithId, Id, Idx};
use transit_model_relations::IdxSet;

#[derive(Debug)]
pub enum Action {
    Extract,
    Remove,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum ObjectType {
    Network,
    Line,
}

type PropertyValues = HashMap<String, HashSet<String>>;

#[derive(Debug)]
pub struct Filter {
    action: Action,
    filters: HashMap<ObjectType, PropertyValues>,
}

impl Filter {
    pub fn new(action: Action) -> Self {
        Filter {
            action,
            filters: HashMap::new(),
        }
    }

    pub fn add<T: Into<String>>(&mut self, object_type: ObjectType, prop: T, value: T) {
        let props = self.filters.entry(object_type).or_insert_with(HashMap::new);
        props
            .entry(prop.into())
            .or_insert_with(HashSet::new)
            .insert(value.into());
    }
}

fn updated_stop_time_attributes<T>(
    vehicle_journeys: &CollectionWithId<VehicleJourney>,
    attributes_map: &HashMap<(Idx<VehicleJourney>, u32), T>,
    old_vj_idx_to_vj_id: &HashMap<Idx<VehicleJourney>, String>,
) -> HashMap<(Idx<VehicleJourney>, u32), T>
where
    T: Clone,
{
    let mut updated_attributes_map = HashMap::new();
    for (&(old_vj_idx, sequence), attribute) in attributes_map {
        if let Some(new_vj_idx) = old_vj_idx_to_vj_id
            .get(&old_vj_idx)
            .and_then(|vj_id| vehicle_journeys.get_idx(vj_id))
        {
            updated_attributes_map.insert((new_vj_idx, sequence), attribute.clone());
        }
    }
    updated_attributes_map
}

fn i_do_not_know_how_to_name_it<T: Id<T>>(
    action: &Action,
    model: &Model,
    collection: &mut CollectionWithId<T>,
    ids: HashSet<String>,
    calendars_used: &mut IdxSet<Calendar>,
    vjs_used: &mut IdxSet<VehicleJourney>,
) where
    IdxSet<T>: GetCorresponding<Calendar>,
    IdxSet<T>: GetCorresponding<VehicleJourney>,
{
    let id_to_old_idx = collection.get_id_to_idx().clone();
    match action {
        Action::Extract => collection.retain(|obj| ids.contains(obj.id())),
        Action::Remove => collection.retain(|obj| !ids.contains(obj.id())),
    }

    let set_idx = collection
        .values()
        .map(|obj| id_to_old_idx[obj.id()])
        .collect();
    *calendars_used = calendars_used
        .clone()
        .union(&model.get_corresponding(&set_idx))
        .cloned()
        .collect();
    *vjs_used = vjs_used
        .clone()
        .union(&model.get_corresponding(&set_idx))
        .cloned()
        .collect();
}

/// Extract or remove or networks/lines
pub fn filter(model: Model, filter: Filter) -> Result<Model> {
    let calendars = model.calendars.clone();
    let vjs = model.vehicle_journeys.clone();
    let old_vj_idx_to_vj_id: HashMap<Idx<VehicleJourney>, String> = model
        .vehicle_journeys
        .get_id_to_idx()
        .iter()
        .map(|(id, &idx)| (idx, id.clone()))
        .collect();

    let mut calendars_used: IdxSet<Calendar> = BTreeSet::new();
    let mut vjs_used: IdxSet<VehicleJourney> = BTreeSet::new();

    for (object_type, prop_values) in filter.filters {
        match object_type {
            ObjectType::Network => {
                let mut collection = model.networks.clone();

                let mut ids: HashSet<String> = HashSet::new();
                for (prop, values) in prop_values {
                    ids = match prop.as_ref() {
                        "network_id" => values
                            .into_iter()
                            .map(|id| match collection.get(&id) {
                                Some(_) => Ok(id.to_string()),
                                None => bail!("network {} not found.", id),
                            })
                            .collect::<Result<_>>()?,
                        _ => bail!("property {} not found.", prop),
                    };
                }

                i_do_not_know_how_to_name_it(
                    &filter.action,
                    &model,
                    &mut collection,
                    ids,
                    &mut calendars_used,
                    &mut vjs_used,
                );
            }
            ObjectType::Line => {
                let mut collection = model.lines.clone();

                let mut ids = HashSet::new();
                for (prop, values) in prop_values {
                    ids = match prop.as_ref() {
                        "line_code" => {
                            let ids: HashSet<String> = collection
                                .values()
                                .filter(|l| {
                                    let code = l.code.as_deref().unwrap_or("");
                                    values.contains(code)
                                })
                                .map(|l| l.id.clone())
                                .collect();
                            if ids.is_empty() {
                                bail!("no lines with property {} {:?} found.", prop, values);
                            }

                            ids
                        }
                        _ => bail!("property {} not found.", prop),
                    };
                }
                i_do_not_know_how_to_name_it(
                    &filter.action,
                    &model,
                    &mut collection,
                    ids,
                    &mut calendars_used,
                    &mut vjs_used,
                );
            }
        };
    }

    let mut collections = model.into_collections();

    collections
        .calendars
        .retain(|c| calendars_used.contains(&calendars.get_idx(&c.id).unwrap()));

    collections
        .vehicle_journeys
        .retain(|c| vjs_used.contains(&vjs.get_idx(&c.id).unwrap()));

    collections.stop_time_ids = updated_stop_time_attributes(
        &collections.vehicle_journeys,
        &collections.stop_time_ids,
        &old_vj_idx_to_vj_id,
    );
    collections.stop_time_headsigns = updated_stop_time_attributes(
        &collections.vehicle_journeys,
        &collections.stop_time_headsigns,
        &old_vj_idx_to_vj_id,
    );
    collections.stop_time_comments = updated_stop_time_attributes(
        &collections.vehicle_journeys,
        &collections.stop_time_comments,
        &old_vj_idx_to_vj_id,
    );

    if collections.calendars.is_empty() {
        bail!("the data does not contain services anymore.")
    }

    Model::new(collections)
}
