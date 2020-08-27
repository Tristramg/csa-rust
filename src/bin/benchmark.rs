use chrono::prelude::*;
use cpuprofiler::PROFILER;
use csa::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "csa-benchmark",
    about = "Runs a benchmark and profiles the algorithm."
)]
struct Opt {
    #[structopt(help = "The first day of the timetable")]
    first_day: String,

    #[structopt(
        short = "h",
        long = "horizon",
        help = "How many days are loaded",
        default_value = "1"
    )]
    horizon: u16,

    #[structopt(
        short = "i",
        long = "input",
        help = "Folder where the GTFS files are",
        default_value = "."
    )]
    input: String,
}

fn main() {
    let opt = Opt::from_args();
    let gtfs = gtfs_structures::Gtfs::new(&opt.input).unwrap();
    gtfs.print_stats();
    let timetable = structures::Timetable::from_gtfs(&gtfs, &opt.first_day, opt.horizon);
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
