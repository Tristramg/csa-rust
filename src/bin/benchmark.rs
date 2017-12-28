extern crate csa;

extern crate chrono;
extern crate cpuprofiler;
extern crate itertools;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use chrono::prelude::*;
use csa::*;
use cpuprofiler::PROFILER;
use itertools::Itertools;

fn main() {
    let gtfs = gtfs_structures::Gtfs::new("./test_data/idf/").unwrap();
    gtfs.print_stats();
    let timetable = structures::Timetable::from_gtfs(gtfs, "2017-11-28", 1);
    timetable.print_stats();
    let to = timetable.stops
                .iter()
                .enumerate()
                // RER A, Châtelet
                .filter(|&(_, stop)| stop.id == "StopPoint:8775860:810:A")
                .last()
                .unwrap()
                .0;

    let runs = 9;
    let now = Utc::now();
    PROFILER.lock().unwrap().start("./bench.profile").unwrap();
    for _ in 0..runs {
        algo::compute(&timetable, to);
    }
    let routes = algo::compute(&timetable, to);
    PROFILER.lock().unwrap().stop().unwrap();

    println!(
        "Number of routes to {}: {}, computed in {} ms and {} runs",
        to,
        routes.iter().map(|p| p.len()).sum::<usize>(),
        Utc::now().signed_duration_since(now).num_milliseconds(),
        runs + 1
    );
}
