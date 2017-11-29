extern crate chrono;
extern crate itertools;
use gtfs_structures::Gtfs;
use std::collections::HashMap;
use self::chrono::prelude::*;
use self::itertools::Itertools;

struct Stop {
    id: String,
    name: String,
}

struct Connection {
    trip: u32,
    dep_time: u16,
    arr_time: u16,
    dep_stop: u32,
    arr_stop: u32,
}

struct Footpath {}

pub struct Timetable {
    transform_duration: i64,
    stops: Vec<Stop>,
    connections: Vec<Connection>,
    footpaths: Vec<Footpath>,
}

impl Timetable {
    pub fn from_gtfs(gtfs: Gtfs, start_date_str: &str, horizon: u16) -> Timetable {
        let start_date = start_date_str
            .parse::<NaiveDate>()
            .expect("Could not parse start date");

        let now = Utc::now();
        let connections = Timetable::connections(&gtfs, start_date, horizon);
        let transform_duration = Utc::now().signed_duration_since(now).num_milliseconds();

        Timetable {
            stops: gtfs.stops
                .iter()
                .map(|stop| {
                    Stop {
                        id: stop.id.to_owned(),
                        name: stop.stop_name.to_owned(),
                    }
                })
                .collect(),
            connections: Timetable::connections(&gtfs, start_date, horizon),
            footpaths: Vec::new(),
        }
    }

    pub fn print_stats(&self) {
        println!("Stops: {}", self.stops.len());
        println!("Connections: {}", self.connections.len());
        println!("Footpaths: {}", self.footpaths.len());
    }

    fn connections(gtfs: &Gtfs, start_date: NaiveDate, horizon: u16) -> Vec<Connection> {
        let mut result = Vec::new();

        let mut trip_indices = HashMap::new();
        let trip_ids: Vec<&String> = gtfs.trips.keys().collect();
        for i in 0..trip_ids.len() {
            trip_indices.insert(trip_ids[i], i);
        }

        let mut stop_indices = HashMap::new();
        for i in 0..gtfs.stops.len() {
            stop_indices.insert(&gtfs.stops[i].id, i as u32);
        }

        for (trip_id, stop_times) in &(&gtfs.stop_times).into_iter().group_by(|elt| &elt.trip_id) {
            let trip_index = *trip_indices
                .get(trip_id)
                .expect(&format!("Unknown trip id {}", trip_id));
            let gtfs_trip = gtfs.trips.get(trip_id).expect("Something went wrong");

            let days = gtfs.trip_days(&gtfs_trip.service_id, start_date);

            for (departure, arrival) in stop_times.tuple_windows() {
                let dep_time = departure.departure_time;
                let arr_time = arrival.arrival_time;
                let dep_stop = *stop_indices
                    .get(&departure.stop_id)
                    .expect(&format!("Unknown stop id {}", departure.stop_id));

                let arr_stop = *stop_indices
                    .get(&arrival.stop_id)
                    .expect(&format!("Unknown stop id {}", arrival.stop_id));

                for day in &days {
                    if *day < horizon {
                        result.push(Connection {
                            trip: trip_index as u32,
                            dep_time: dep_time + (day * 24 * 60),
                            arr_time: arr_time,
                            dep_stop: dep_stop,
                            arr_stop: arr_stop,
                        });
                    }
                }
            }
        }
        result
    }
}
