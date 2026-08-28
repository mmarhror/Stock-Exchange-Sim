use std::collections::HashMap;
use crate::parser::Config;

#[derive(Debug, Clone)]
pub struct Process {
    pub name: String,
    pub needs: HashMap<String, usize>,
    pub results: HashMap<String, usize>,
    pub delay: usize,
}

impl Process {
    pub fn can_start(&self, stocks: &HashMap<String, usize>) -> bool {
        for (name, qty) in &self.needs {
            match stocks.get(name) {
                Some(curr_qty) if curr_qty >= qty => {}
                _ => {
                    return false;
                }
            }
        }
        true
    }

    pub fn start(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.needs {
            // Safe to unwrap because we assume can_start was called first
            let curr_qty = stocks.get_mut(name).unwrap();
            *curr_qty -= qty;
            if *curr_qty == 0 {
                stocks.remove(name);
            }
        }
    }

    pub fn finish(&self, stocks: &mut HashMap<String, usize>) {
        for (name, qty) in &self.results {
            let curr_qty = stocks.entry(name.clone()).or_insert(0);
            *curr_qty += qty;
        }
    }
}

pub struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

pub fn simulate(config: Config, time: usize) {
    let mut stocks = config.stocks;
    let curr_cycle = 0;
    let running: Vec<RunningProcess> = Vec::new();

    loop {
        for r in running.iter() {
            if curr_cycle == r.finish_cycle {
                r.process.finish(&mut stocks);
                
            }
        }


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
