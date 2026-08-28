use std::collections::HashMap;
use crate::parser::{ Process, Config };

pub struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

pub fn simulate(config: Config, time: usize) {
    let running: Vec<RunningProcess> = Vec::new();
    
    loop {
    }
}

//
//
//
//
//
//
//
//
//
//
//
//
//
//

pub fn can_start(stocks: &HashMap<String, usize>, process: &Process) -> bool {
    for (name, qty) in &process.needs {
        match stocks.get(name) {
            Some(curr_qty) if curr_qty >= qty => {}
            _ => {
                return false;
            }
        }
    }

    true
}

pub fn start_process(stocks: &mut HashMap<String, usize>, process: &Process) {
    for (name, qty) in process.needs.iter() {
        let curr_qty = stocks.get_mut(name).unwrap();

        *curr_qty -= qty;

        if *curr_qty == 0 {
            stocks.remove(name);
        }
    }
}

pub fn finish_process(stocks: &mut HashMap<String, usize>, process: &Process) {
    for (name, qty) in process.results.iter() {
        let curr_qty = stocks.entry(name.clone()).or_insert(0);

        *curr_qty += qty;
    }
}
