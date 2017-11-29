mod structures;
mod gtfs_structures;
#[macro_use]
extern crate serde_derive;

fn main() {
    println!("Hello, world!");
    let gtfs = gtfs_structures::Gtfs::new("/home/tristram/workspace/csa/test_data/idf/");
    match gtfs {
        Ok(g) => {
            g.print_stats();
            let csa = structures::Timetable::from_gtfs(g, "2017-11-28", 10);
            csa.print_stats();
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
