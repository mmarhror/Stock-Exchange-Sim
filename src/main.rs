use std::env;
use std::process;

use stock_exchange::parser;
use stock_exchange::simulator;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: ./stock_exchange <config_file> <waiting_time>");
        process::exit(1);
    }

    let config_file = &args[1];
    let waiting_time_str = &args[2];

    let waiting_time: usize = match parser::parse_time(waiting_time_str) {
        Ok(time) => time,
        Err(e) => {
            eprintln!("Time Parsing Error: {}", e);
            process::exit(1);
        }
    };

    let config = match parser::parse_file(config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("File Parsing Error: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = simulator::simulate(config, config_file, waiting_time) {
        eprintln!("Simulation Error: {}", e);
        process::exit(1);
    }
}
