use structures::*;

// A profile defines a route
// Given its connection, we can rebuild the whole route
#[derive(Debug)]
pub struct Profile {
    // If None, it means that it is the starting point
    pub out_connection: Option<usize>,
    pub dep_time: u16,
    pub arr_time: u16,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            out_connection: None,
            dep_time: u16::max_value(),
            arr_time: 0,
        }
    }
}

fn arrival_time_with_stop_change(profiles: &[Profile], c: &Connection) -> Option<u16> {
    let transfer_duration = 5;
    profiles
        .iter()
        .rposition(|p| p.dep_time > c.arr_time + transfer_duration)
        .map(|pos| {
            let p = &profiles[pos];
            if p.out_connection.is_some() {
                p.arr_time
            } else {
                // If this is the very last connection to target, it gives us the arrival time
                c.arr_time
            }
        })
}

trait Incorporate {
    fn incorporate(&mut self, candidate: Profile) -> bool;
    fn insert_and_filter(&mut self, candidate: Profile, pivot: usize);
}

impl Incorporate for Vec<Profile> {
    fn insert_and_filter(&mut self, candidate: Profile, pivot: usize) {
        // Remove all the dominated solutions
        // We only consider profiles leaving earlier after the candidate
        // As self is sorted by decreasing dep_time, we need only to look after the pivot
        let mut i = pivot + 1;
        while i < self.len() {
            if candidate.arr_time <= self[i].arr_time {
                self.remove(i);
            } else {
                i += 1;
            }
        }
        self.insert(pivot + 1, candidate);
    }

    fn incorporate(&mut self, candidate: Profile) -> bool {
        // The profiles are ordered in decreasing dep_time
        // The pivot is the element leaving just after the candidate
        match self.iter().rposition(|p| p.dep_time >= candidate.dep_time) {
            Some(pivot) => {
                if candidate.arr_time < self[pivot].arr_time {
                    self.insert_and_filter(candidate, pivot);
                    true
                } else {
                    false
                }
            }
            None => {
                self.push(candidate);
                true
            }
        }
    }
}

fn min_duration(a: Option<u16>, b: Option<u16>) -> Option<u16> {
    match (a, b) {
        (None, _) => b,
        (_, None) => a,
        (Some(a), Some(b)) => Some(a.min(b)),
    }
}

// It returns all the possible routes, from all possible nodes to the given destination
pub fn compute(timetable: &Timetable, destinations: &[usize]) -> Vec<Vec<Profile>> {
    let mut arr_time_with_trip: Vec<_> = timetable.trips.iter().map(|_| None).collect();
    let mut profiles: Vec<_> = timetable.stops.iter().map(|_| Vec::new()).collect();
    let mut final_footpaths: Vec<Option<u16>> = timetable.stops.iter().map(|_| None).collect();
    for destination in destinations {
        for fp in &timetable.footpaths[*destination] {
            final_footpaths[fp.from] = min_duration(final_footpaths[fp.from], Some(fp.duration));
        }
        profiles[*destination].push(Default::default());
    }

    for (conn_index, c) in timetable.connections.iter().enumerate() {
        // Case 1: walking to target
        let t1 = final_footpaths[c.arr_stop].map(|d| c.arr_time + d);

        // Case 2: Staying seated in the trip, we will reach the target at `t2`
        let t2 = arr_time_with_trip[c.trip];

        // Case 3: Transfering in the same stop, we look up the earliest compatible arrival
        let t3 = arrival_time_with_stop_change(&profiles[c.arr_stop], c);

        if let Some(t) = [t1, t2, t3].iter().filter_map(|t| *t).min() {
            let candidate = Profile {
                out_connection: Some(conn_index),
                dep_time: c.dep_time,
                arr_time: t,
            };

            if profiles[c.dep_stop].incorporate(candidate) {
                for footpath in timetable.footpaths[c.dep_stop]
                    .iter()
                    .filter(|p| p.duration < c.dep_time)
                {
                    profiles[footpath.from].incorporate(Profile {
                        out_connection: Some(conn_index),
                        dep_time: c.dep_time - footpath.duration,
                        arr_time: t,
                    });
                }
            }
            // Using this trip, we will reach the target at `t`
            arr_time_with_trip[c.trip] = Some(t);
        }
    }

    profiles
}

mod tests {
    use super::*;

    #[test]
    fn test_incorporate() {
        let mut profiles = Vec::new();
        profiles.incorporate(Profile {
            dep_time: 20,
            arr_time: 30,
            out_connection: None,
        });

        assert_eq!(1, profiles.len());

        profiles.incorporate(Profile {
            dep_time: 10,
            arr_time: 20,
            out_connection: None,
        });
        assert_eq!(2, profiles.len());

        // Dominated profile, should not be inserted
        profiles.incorporate(Profile {
            dep_time: 8,
            arr_time: 21,
            out_connection: None,
        });
        assert_eq!(2, profiles.len());
        assert_eq!(10, profiles[1].dep_time);

        profiles.incorporate(Profile {
            dep_time: 0,
            arr_time: 10,
            out_connection: None,
        });
        assert_eq!(3, profiles.len());

        // Dominating profil, should remove the existing one
        profiles.incorporate(Profile {
            dep_time: 11,
            arr_time: 20,
            out_connection: None,
        });
        assert_eq!(3, profiles.len());
        assert_eq!(11, profiles[1].dep_time);
    }

    #[test]
    fn simple_transfer() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("b", "0:30")
            .s("c", "0:40");

        let t = b.build();
        let profiles = compute(&t, &[2]);
        assert_eq!(1, profiles[0].len());
        assert_eq!(10, profiles[0][0].dep_time);
        assert_eq!(40, profiles[0][0].arr_time);

        assert_eq!(1, profiles[1].len());
        assert_eq!(30, profiles[1][0].dep_time);
        assert_eq!(40, profiles[1][0].arr_time);
    }

    #[test]
    fn no_route() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("c", "0:30")
            .s("d", "0:40");
        let t = b.build();
        let profiles = compute(&t, &[0]);
        assert!(profiles[2].is_empty());
    }

    #[test]
    fn insuffisent_transfer_time() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("b", "0:20")
            .s("c", "0:40");
        let t = b.build();
        let profiles = compute(&t, &[2]);
        assert!(profiles[0].is_empty());
        assert!(!profiles[1].is_empty());
    }

    #[test]
    fn equivalent_solutions() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("a", "1:10")
            .s("b", "1:20")
            .trip()
            .s("b", "0:30")
            .s("c", "0:40")
            .trip()
            .s("b", "1:30")
            .s("c", "1:40");

        let t = b.build();
        let profiles = compute(&t, &[2]);
        assert_eq!(2, profiles[0].len());
        assert_eq!(10, profiles[0][1].dep_time);
        assert_eq!(40, profiles[0][1].arr_time);
        assert_eq!(70, profiles[0][0].dep_time);
        assert_eq!(100, profiles[0][0].arr_time);
    }

    #[test]
    fn dominated_solution() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("b", "0:30")
            .s("c", "0:40")
            .trip()
            .s("a", "0:10")
            .s("c", "0:50");

        let t = b.build();
        let profiles = compute(&t, &[2]);
        assert_eq!(1, profiles[0].len());
        assert_eq!(40, profiles[0][0].arr_time);
    }

    #[test]
    fn stay_seated() {
        let mut b = Timetable::builder();
        b.trip().s("a", "0:10").s("b", "0:20").s("c", "0:40");
        let t = b.build();
        let profiles = compute(&t, &[2]);
        assert_eq!(1, profiles[0].len());
        assert_eq!(10, profiles[0][0].dep_time);
        assert_eq!(40, profiles[0][0].arr_time);
        assert_eq!(1, profiles[1].len());
        assert_eq!(20, profiles[1][0].dep_time);
        assert_eq!(40, profiles[1][0].arr_time);
    }

    #[test]
    fn footpath() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("c", "0:30")
            .s("d", "0:40");
        let mut t = b.build();
        t.footpaths[2].push(Footpath {
            from: 1,
            duration: 3,
        });
        let profiles = compute(&t, &[3]);
        assert_eq!(1, profiles[0].len());
        assert_eq!(10, profiles[0][0].dep_time);
        assert_eq!(1, profiles[1].len());
        assert_eq!(27, profiles[1][0].dep_time);
    }

    #[test]
    fn final_footpath() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("b", "0:30")
            .s("c", "0:40");
        let mut t = b.build();
        t.footpaths[2].push(Footpath {
            from: 1,
            duration: 3,
        });
        let profiles = compute(&t, &[2]);
        assert_eq!(23, profiles[0][0].arr_time);
    }

    #[test]
    fn final_multiple_footpath() {
        let mut b = Timetable::builder();
        b.trip()
            .s("a", "0:10")
            .s("b", "0:20")
            .trip()
            .s("c", "0:30")
            .s("d", "0:40");
        let mut t = b.build();
        t.footpaths[2].push(Footpath {
            from: 1,
            duration: 3,
        });
        t.footpaths[3].push(Footpath {
            from: 1,
            duration: 10,
        });
        let profiles = compute(&t, &[2, 3]);
        assert_eq!(23, profiles[0][0].arr_time);
    }
}
