# Benchmarking and profiling

You need to install google-perftool libraries. On my system: `sudo apt libgoogle-perftools-dev`.

This will use data in `test_data/idf/` that comes from https://opendata.stif.info

`cargo run --release 2017-11-28 -i test_data/idf/`
`google-pprof --web target/release/bench ./bench.profile`

More detailed results can be obtained by running in debug:
`cargo run 2017-11-28 -i test_data/idf/`
`google-pprof --web target/debug/bench ./bench.profile`
