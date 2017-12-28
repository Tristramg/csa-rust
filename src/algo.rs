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

impl Profile {
    fn dominates(&self, other: &Self) -> bool {
        self.arr_time <= other.arr_time && self.dep_time >= other.dep_time
    }

    fn is_non_dominated(&self, other_opt: Option<&Self>) -> bool {
        // By construction, we know self.dep_time <= other.dep_time
        match other_opt {
            Some(other) => self.arr_time <= other.arr_time,
            None => true,
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
}

impl Incorporate for Vec<Profile> {
    fn incorporate(&mut self, candidate: Profile) -> bool {
        // The profiles are ordered in decreasing dep_time
        // The pivot is the element just before the candidate
        let pivot = self.iter().rposition(|p| p.dep_time >= candidate.dep_time);
        let mut earlier_profiles: Vec<Profile> = match pivot {
            None => Vec::new(),
            Some(position) => self.drain(position + 1..)
                .filter(|p| candidate.dominates(p))
                .collect(),
        };

        let incorporated = candidate.is_non_dominated(self.last());
        if incorporated {
            self.push(candidate);
        }

        self.append(&mut earlier_profiles);

        incorporated
    }
}

// It returns all the possible routes, from all possible nodes to the given destination
pub fn compute(timetable: &Timetable, destination: usize) -> Vec<Vec<Profile>> {
    let mut arr_time_with_trip: Vec<_> = timetable.trips.iter().map(|_| None).collect();
    let mut profiles: Vec<_> = timetable.stops.iter().map(|_| Vec::new()).collect();
    profiles[destination].push(Default::default());
    let final_footpaths = &timetable.footpaths[destination];

    for (conn_index, c) in timetable.connections.iter().enumerate() {
        // Case 1: walking to target
        let t1 = final_footpaths
            .iter()
            .find(|f| f.from == c.arr_stop)
            .map(|f| c.arr_time + f.duration);

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
    use structures::Timetable;

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
    fn domination() {
        let p = Profile {
            out_connection: None,
            dep_time: 10,
            arr_time: 20,
        };

        assert!(p.dominates(&Profile {
            out_connection: None,
            dep_time: 9,
            arr_time: 21,
        }));
        assert!(!p.dominates(&Profile {
            out_connection: None,
            dep_time: 9,
            arr_time: 19,
        }));
        assert!(!p.dominates(&Profile {
            out_connection: None,
            dep_time: 11,
            arr_time: 21,
        }));
    }

    #[test]
    fn non_domination() {
        let p = Profile {
            out_connection: None,
            dep_time: 10,
            arr_time: 20,
        };

        assert!(p.is_non_dominated(None));
        assert!(p.is_non_dominated(Some(&Profile {
            out_connection: None,
            dep_time: 11,
            arr_time: 21,
        })));
        assert!(!p.is_non_dominated(Some(&Profile {
            out_connection: None,
            dep_time: 11,
            arr_time: 19,
        })));
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
        let profiles = compute(&t, 2);
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
        let profiles = compute(&t, 0);
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
        let profiles = compute(&t, 2);
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
        let profiles = compute(&t, 2);
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
        let profiles = compute(&t, 2);
        assert_eq!(1, profiles[0].len());
        assert_eq!(40, profiles[0][0].arr_time);
    }

    #[test]
    fn stay_seated() {
        let mut b = Timetable::builder();
        b.trip().s("a", "0:10").s("b", "0:20").s("c", "0:40");
        let t = b.build();
        let profiles = compute(&t, 2);
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
        let profiles = compute(&t, 3);
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
        let profiles = compute(&t, 2);
        assert_eq!(23, profiles[0][0].arr_time);
    }
}
