// Copyright 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see
// <http://www.gnu.org/licenses/>.

use super::{lines, stops};
use crate::{
    model::{Collections, Model},
    objects::Dataset,
    AddPrefix, Result,
};
use chrono::naive::{MAX_DATE, MIN_DATE};
use std::path::Path;
use transit_model_collection::CollectionWithId;

/// Read Netex IDF format into a Navitia Transit Model
pub fn read<P>(netex_idf_path: P, config_path: Option<P>, prefix: Option<String>) -> Result<Model>
where
    P: AsRef<Path>,
{
    fn init_dataset_validity_period(dataset: &mut Dataset) {
        dataset.start_date = MAX_DATE;
        dataset.end_date = MIN_DATE;
    }

    let mut collections = Collections::default();
    let (contributor, mut dataset, feed_infos) = crate::read_utils::read_config(config_path)?;
    collections.contributors = CollectionWithId::from(contributor);
    init_dataset_validity_period(&mut dataset);
    collections.datasets = CollectionWithId::from(dataset);
    collections.feed_infos = feed_infos;

    let path = netex_idf_path.as_ref();
    stops::from_path(&path.join("arrets.xml"), &mut collections)?;
    // TODO : use _lines_netex_idf to get trips>physical_mode_id
    // let _lines_netex_idf = lines::from_path(&path.join("lignes.xml"), &mut collections)?;

    if let Some(prefix) = prefix {
        collections.add_prefix_with_sep(prefix.as_str(), ":");
    }

    // TODO: uncomment once we have all netex  parsed
    // collections.sanitize()?;
    Model::new(collections)
}
