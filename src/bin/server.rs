use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use csa::structures::Timetable;
use serde::Serialize;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "csa-server", about = "Runs a web server to request routes")]
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

#[derive(Serialize)]
struct Summary {
    departure: chrono::NaiveDateTime,
    arrival: chrono::NaiveDateTime,
    transfers: usize,
}

impl Summary {
    fn from(
        connections: &[&csa::structures::Connection],
        timetable: &csa::structures::Timetable,
    ) -> Self {
        let departure = connections.first().expect("Missing departure in connexion");
        let arrival = connections.last().expect("Missing arrival in connexion");
        let trips: std::collections::HashSet<_> = connections.iter().map(|c| c.trip).collect();
        let dep_time = chrono::NaiveTime::from_hms(0, 0, 0)
            + chrono::Duration::seconds(departure.dep_time as i64); //chrono::NaiveTime::from_num_seconds_from_midnight(departure.dep_time, 0);
        let arr_time = chrono::NaiveTime::from_hms(0, 0, 0)
            + chrono::Duration::seconds(arrival.arr_time as i64);

        Self {
            departure: timetable.start_date.and_time(dep_time),
            arrival: timetable.start_date.and_time(arr_time),
            transfers: trips.len(),
        }
    }
}

async fn compute(req: HttpRequest, timetable: web::Data<Timetable>) -> impl Responder {
    // Chatelet les halles
    let stop_area = req
        .match_info()
        .get("stop_area")
        .unwrap_or("StopArea:8775860");

    let to = timetable.stop_index_by_stop_area_id(stop_area);
    let result = csa::algo::compute(&timetable, &to);
    let mut output = Vec::<Vec<_>>::new();

    for i in 0..timetable.stops.len() {
        let routes = result[i]
            .iter()
            .map(|profile| Summary::from(&profile.route(result.as_slice(), &timetable), &timetable))
            .collect();
        output.push(routes);
    }
    serde_json::to_string(&output)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();
    let gtfs = gtfs_structures::Gtfs::new(&opt.input).unwrap();
    gtfs.print_stats();
    let timetable = Timetable::from_gtfs(&gtfs, &opt.first_day.clone(), opt.horizon);
    let data = web::Data::new(timetable);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/to/{stop_area}", web::get().to(compute))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
