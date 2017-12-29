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

    let runs = 5;
    let chatelet_les_halles = "StopArea:8775860";
    let gare_de_provins = "StopArea:8711616";
    let gare_de_mantes = "StopArea:8738150";
    let vignoles = "StopArea:59498";

    let stop_areas = &[
        chatelet_les_halles,
        gare_de_mantes,
        gare_de_provins,
        vignoles,
    ];
    let now = Utc::now();
    PROFILER.lock().unwrap().start("./bench.profile").unwrap();
    for sa in stop_areas {
        for _ in 0..runs {
            let to = timetable.stop_index_by_stop_area_id(sa);
            algo::compute(&timetable, &to);
        }
    }
    PROFILER.lock().unwrap().stop().unwrap();

    println!(
        "Benchmark done. Computed in {} ms and {} runs",
        Utc::now().signed_duration_since(now).num_milliseconds(),
        runs * stop_areas.len()
    );
}
