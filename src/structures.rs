extern crate chrono;
extern crate itertools;
use gtfs_structures;
use std::collections::HashMap;
use self::chrono::prelude::*;
use self::itertools::Itertools;

struct Stop {
    id: String,
    name: String,
    parent_station: Option<String>,
    location_type: gtfs_structures::LocationType,
}


impl<'a> From<&'a gtfs_structures::Stop> for Stop {
    fn from(stop: &gtfs_structures::Stop) -> Self {
        Self {
            id: stop.id.to_owned(),
            name: stop.stop_name.to_owned(),
            parent_station: stop.parent_station.to_owned(),
            location_type: stop.location_type,
        }
    }
}

struct Connection {
    trip: u32,
    dep_time: u16,
    arr_time: u16,
    dep_stop: u32,
    arr_stop: u32,
}

struct Footpath {
    to: u32,
    duration: u16,
}

pub struct Timetable {
    transform_duration: i64,
    stops: Vec<Stop>,
    connections: Vec<Connection>,
    footpaths: Vec<Vec<Footpath>>,
}

impl Timetable {
    pub fn from_gtfs(gtfs: gtfs_structures::Gtfs, start_date_str: &str, horizon: u16) -> Timetable {
        let start_date = start_date_str.parse::<NaiveDate>().expect(
            "Could not parse start date",
        );

        let stops: Vec<_> = gtfs.stops.iter().map(Stop::from).collect();

        let stop_indices = stops
            .iter()
            .enumerate()
            .map(|(index, stop)| (stop.id.to_owned(), index as u32))
            .collect();

        let now = Utc::now();
        let connections = Timetable::connections(gtfs, start_date, horizon, &stop_indices);
        let transform_duration = Utc::now().signed_duration_since(now).num_milliseconds();

        Timetable {
            footpaths: Timetable::footpaths(&stops, &stop_indices),
            stops: stops,
            connections: connections,
            transform_duration: transform_duration,
        }
    }

    pub fn print_stats(&self) {
        println!("Final data structures: ");
        println!("  Stops: {}", self.stops.len());
        println!(
            "  Footpaths: {}",
            self.footpaths.iter().map(|e| e.len()).sum::<usize>()
        );
        println!("  Connections: {}", self.connections.len());
        println!("  Connections built in {} ms", self.transform_duration);
    }

    fn connections(
        gtfs: gtfs_structures::Gtfs,
        start_date: NaiveDate,
        horizon: u16,
        stop_indices: &HashMap<String, u32>,
    ) -> Vec<Connection> {
        let mut result = Vec::new();

        let trip_indices: HashMap<_, _> = gtfs.trips
            .keys()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();

        for (trip_id, stop_times) in &(&gtfs.stop_times).into_iter().group_by(|elt| &elt.trip_id) {
            let trip_index = *trip_indices.get(trip_id).expect(&format!(
                "Unknown trip id {}",
                trip_id
            ));
            let gtfs_trip = gtfs.trips.get(trip_id).expect("Something went wrong");

            let days = gtfs.trip_days(&gtfs_trip.service_id, start_date);

            for (departure, arrival) in stop_times.tuple_windows() {
                let dep_time = departure.departure_time;
                let arr_time = arrival.arrival_time;
                let dep_stop = *stop_indices.get(&departure.stop_id).expect(&format!(
                    "Unknown stop id {}",
                    departure.stop_id
                ));

                let arr_stop = *stop_indices.get(&arrival.stop_id).expect(&format!(
                    "Unknown stop id {}",
                    arrival.stop_id
                ));

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

    fn footpaths(stops: &Vec<Stop>, stop_indices: &HashMap<String, u32>) -> Vec<Vec<Footpath>> {
        let mut result: Vec<Vec<_>> = stops.iter().map(|_| Vec::new()).collect();
        let mut stop_areas = HashMap::new();

        for stop in stops {
            if let Some(ref parent) = stop.parent_station {
                if stop.location_type == gtfs_structures::LocationType::StopPoint {
                    let children = stop_areas.entry(parent).or_insert(Vec::new());
                    children.push(stop.id.to_owned())
                }
            }
        }

        for (_, children) in stop_areas {
            for (child_a, child_b) in
                children.iter().cartesian_product(&children).filter(
                    |&(a, b)| a != b,
                )
            {
                let index_a = *stop_indices.get(child_a).expect(&format!(
                    "Missing child station {}",
                    child_a
                ));
                let index_b = *stop_indices.get(child_b).expect(&format!(
                    "Missing child station {}",
                    child_b
                ));

                result[index_a as usize].push(Footpath {
                    duration: 5,
                    to: index_b,
                });
            }
        }

        result
    }
}
