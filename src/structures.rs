use chrono::prelude::{NaiveDate, Utc};
use itertools::Itertools;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Stop {
    pub id: String,
    pub name: String,
    pub parent_station: Option<String>,
    pub location_type: gtfs_structures::LocationType,
}

impl<'a> From<&'a std::sync::Arc<gtfs_structures::Stop>> for Stop {
    fn from(stop: &std::sync::Arc<gtfs_structures::Stop>) -> Self {
        Self {
            id: stop.id.to_owned(),
            name: stop.name.to_owned(),
            parent_station: stop.parent_station.to_owned(),
            location_type: stop.location_type,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Connection {
    pub trip: usize,
    pub dep_time: u32,
    pub arr_time: u32,
    pub dep_stop: usize,
    pub arr_stop: usize,
}

#[derive(Clone, Debug)]
pub struct Footpath {
    pub from: usize,
    pub duration: u32,
}

pub struct Timetable {
    pub start_date: chrono::NaiveDate,
    pub transform_duration: i64,
    pub stops: Vec<Stop>,
    pub connections: Vec<Connection>,
    pub footpaths: Vec<Vec<Footpath>>,
    pub trips: Vec<Trip>,
}

#[derive(Clone)]
pub struct Trip {}

pub struct TimetableBuilder {
    stop_map: HashMap<String, usize>,
    trips: Vec<Trip>,
    last_stop: Option<(usize, u32)>,
    connections: Vec<Connection>,
}

impl TimetableBuilder {
    pub fn trip(&mut self) -> &mut Self {
        self.last_stop = None;
        self.trips.push(Trip {});
        self
    }

    fn stop(&mut self, stop_id: &str) -> usize {
        let index = self.stop_map.len();
        *self.stop_map.entry(stop_id.to_owned()).or_insert(index)
    }

    pub fn s<'a>(&'a mut self, stop: &str, time: &str) -> &'a mut Self {
        let trip_id = self.trips.len();
        if trip_id == 0 {
            panic!("Timetable builder: trying to add a stop without a trip");
        }
        let stop_index = self.stop(stop);
        let parsed_time = gtfs_structures::parse_time(&format!("0:{}", time))
            .unwrap_or_else(|_| panic!("Invalid time format {}", time));

        if let Some(prev) = self.last_stop {
            self.connections.push(Connection {
                trip: trip_id - 1,
                dep_stop: prev.0,
                dep_time: prev.1,
                arr_stop: stop_index,
                arr_time: parsed_time,
            })
        }

        self.last_stop = Some((stop_index, parsed_time));

        self
    }
    pub fn build(mut self) -> Timetable {
        self.connections.sort_by(|a, b| b.dep_time.cmp(&a.dep_time));
        Timetable {
            start_date: NaiveDate::from_yo(2019, 42),
            trips: self.trips,
            connections: self.connections,
            stops: self
                .stop_map
                .iter()
                .map(|(id, _)| Stop {
                    id: id.to_owned(),
                    name: id.to_owned(),
                    location_type: gtfs_structures::LocationType::StopPoint,
                    parent_station: None,
                })
                .collect(),
            footpaths: self.stop_map.iter().map(|_| Vec::new()).collect(),
            transform_duration: 0,
        }
    }
}

impl Timetable {
    pub fn from_gtfs(
        gtfs: &gtfs_structures::Gtfs,
        start_date_str: &str,
        horizon: u16,
    ) -> Timetable {
        let start_date = start_date_str
            .parse::<NaiveDate>()
            .expect("Could not parse start date");

        let stops: Vec<_> = gtfs.stops.values().map(Stop::from).collect();

        let stop_indices = stops
            .iter()
            .enumerate()
            .map(|(index, stop)| (stop.id.to_owned(), index))
            .collect();

        let now = Utc::now();
        let trips = vec![Trip {}; gtfs.trips.len() * horizon as usize];
        let connections = Timetable::connections(&gtfs, start_date, horizon, &stop_indices);
        let transform_duration = Utc::now().signed_duration_since(now).num_milliseconds();

        Timetable {
            start_date,
            footpaths: Timetable::footpaths(&stops, &stop_indices),
            stops,
            connections,
            transform_duration,
            trips,
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
        gtfs: &gtfs_structures::Gtfs,
        start_date: NaiveDate,
        horizon: u16,
        stop_indices: &HashMap<String, usize>,
    ) -> Vec<Connection> {
        let mut result = Vec::new();

        let mut trip_indices = HashMap::new();
        let mut index = 0;
        for trip_id in gtfs.trips.keys() {
            for day in 0..horizon {
                trip_indices.insert(format!("{}-{}", trip_id, day), index);
                index += 1;
            }
        }

        for (trip_id, gtfs_trip) in &gtfs.trips {
            let days = gtfs.trip_days(&gtfs_trip.service_id, start_date);
            let mut last_arrival = None;

            for (departure, arrival) in gtfs_trip.stop_times.iter().tuple_windows() {
                let dep_time = departure.departure_time.unwrap_or_else(|| {
                    last_arrival.unwrap_or_else(|| {
                        panic!("First departure without time on trip {}", trip_id)
                    })
                });

                let arr_time = arrival.arrival_time.unwrap_or_else(|| dep_time);
                last_arrival = Some(arr_time);
                let dep_stop = *stop_indices
                    .get(&departure.stop.id)
                    .unwrap_or_else(|| panic!("Unknown stop id {}", departure.stop.id));

                let arr_stop = *stop_indices
                    .get(&arrival.stop.id)
                    .unwrap_or_else(|| panic!("Unknown stop id {}", arrival.stop.id));

                for day in &days {
                    if *day < horizon {
                        result.push(Connection {
                            trip: *trip_indices.get(&format!("{}-{}", trip_id, day)).unwrap(),
                            dep_time: dep_time + (u32::from(*day) * 24 * 60 * 60),
                            arr_time: arr_time + (u32::from(*day) * 24 * 60 * 60),
                            dep_stop,
                            arr_stop,
                        });
                    }
                }
            }
        }

        // We want the connections by decreasing departure time
        result.sort_by(|a, b| b.dep_time.cmp(&a.dep_time));
        result
    }

    fn footpaths(stops: &[Stop], stop_indices: &HashMap<String, usize>) -> Vec<Vec<Footpath>> {
        let mut result: Vec<Vec<_>> = stops.iter().map(|_| Vec::new()).collect();
        let mut stop_areas = HashMap::new();

        for stop in stops {
            if let Some(ref parent) = stop.parent_station {
                if stop.location_type == gtfs_structures::LocationType::StopPoint {
                    let children = stop_areas.entry(parent).or_insert_with(Vec::new);
                    children.push(stop.id.to_owned())
                }
            }
        }

        for (_, children) in stop_areas {
            for (child_a, child_b) in children
                .iter()
                .cartesian_product(&children)
                .filter(|&(a, b)| a != b)
            {
                let index_a = *stop_indices
                    .get(child_a)
                    .unwrap_or_else(|| panic!("Missing child station {}", child_b));
                let index_b = *stop_indices
                    .get(child_b)
                    .unwrap_or_else(|| panic!("Missing child station {}", child_b));

                result[index_a as usize].push(Footpath {
                    duration: 5,
                    from: index_b,
                });
            }
        }
        result
    }

    pub fn builder() -> TimetableBuilder {
        TimetableBuilder {
            connections: Vec::new(),
            last_stop: None,
            stop_map: HashMap::new(),
            trips: Vec::new(),
        }
    }

    pub fn stop_index_by_stop_area_id(&self, stop_area_id: &str) -> Vec<usize> {
        self.stops
            .iter()
            .enumerate()
            .filter(|&(_, stop)| stop.parent_station == Some(stop_area_id.to_string()))
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_gtfs() {
        let gtfs = gtfs_structures::Gtfs::new("fixtures/").unwrap();
        let timetable = Timetable::from_gtfs(&gtfs, "2017-1-1", 10);
        assert_eq!(5, timetable.stops.len());
        assert_eq!(2, timetable.connections.len());
        assert_eq!(5, timetable.footpaths.len());
        for i in 0..timetable.stops.len() {
            if timetable.stops[i].id == "stop3" || timetable.stops[i].id == "stop5" {
                assert_eq!(timetable.footpaths[i].len(), 1);
            } else {
                assert!(timetable.footpaths[i].is_empty());
            }
        }
    }

    #[test]
    fn builder() {
        let mut b = Timetable::builder();
        b.trip();
        assert_eq!(1, b.trips.len());
        assert_eq!(0, b.stop("a"));
        assert_eq!(0, b.stop("a"));
        assert_eq!(1, b.stop("b"));
    }

    #[test]
    fn builder_transform() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .s("c", "0:30")
            .trip()
            .s("b", "0:00")
            .s("d", "0:40");

        let t = b.build();
        assert_eq!(4, t.stops.len());
        assert_eq!(2, t.trips.len());
        assert_eq!(3, t.connections.len());
    }
}
