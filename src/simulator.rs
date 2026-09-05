use std::collections::{ HashMap, HashSet };
use std::fs;
use std::path::Path;
use std::time::{ Instant, Duration };

use crate::parser::{ Config, Process };

#[derive(Clone)]
pub struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

#[derive(Clone)]
pub struct State {
    pub curr_cycle: usize,
    pub running: Vec<RunningProcess>,
    pub stocks: HashMap<String, usize>,
    pub history: Vec<String>,
}

impl State {
    pub fn new(stocks: HashMap<String, usize>) -> Self {
        State { curr_cycle: 0, running: Vec::new(), stocks, history: Vec::new() }
    }

    fn get_next_cycle(&self) -> Option<usize> {
        self.running
            .iter()
            .map(|r| r.finish_cycle)
            .min()
    }

    fn finish_processes(&mut self) {
        let (done, still): (Vec<RunningProcess>, Vec<RunningProcess>) = self.running
            .drain(..)
            .partition(|r| r.finish_cycle == self.curr_cycle);

        self.running = still;

        for r in done {
            r.process.finish(&mut self.stocks);
        }
    }

    pub fn advance_to(&mut self, cycle: usize) -> Result<(), String> {
        if cycle < self.curr_cycle {
            return Err(
                format!(
                    "Invalid timestamp: cannot go backwards from cycle {} to {}",
                    self.curr_cycle,
                    cycle
                )
            );
        }

        while self.curr_cycle < cycle {
            self.curr_cycle += 1;
            self.finish_processes();
        }

        Ok(())
    }

    pub fn start_by_name(&mut self, name: &str, processes: &[Process]) -> Result<(), String> {
        let process = processes
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("unknown process: {}", name))?;

        if !process.can_start(&self.stocks) {
            return Err("stock insufficient".to_string());
        }

        process.start(&mut self.stocks);
        self.running.push(RunningProcess {
            process: process.clone(),
            finish_cycle: self.curr_cycle + process.delay,
        });

        self.history.push(format!("{}:{}", self.curr_cycle, name));

        Ok(())
    }

    pub fn finish_all(&mut self) {
        while
            let Some(next_cycle) = self.running
                .iter()
                .map(|r| r.finish_cycle)
                .min()
        {
            let _ = self.advance_to(next_cycle);
        }
    }

    fn print_result(&self, all_stock_names: &[String]) {
        println!("Main Processes :");
        for l in self.history.iter() {
            println!(" {l}");
        }
        println!("No more process doable at cycle {}", self.curr_cycle + 1);
        println!("Stock :");

        let mut sorted_names = all_stock_names.to_vec();
        sorted_names.sort();

        for name in sorted_names {
            let qty = self.stocks.get(&name).unwrap_or(&0);
            println!(" {name} => {qty}");
        }
    }

    fn log_history(&self, filename: &str) -> Result<(), String> {
        let path = Path::new(filename).with_extension("log");
        let log_content = self.history.join("\n") + "\n";
        fs::write(path, log_content).map_err(|e| format!("Failed to write log file: {}", e))
    }
    fn better_then(&self, state: &Self, optimize: &[String]) -> bool {
        for op in optimize {
            if op != "time" {
                let my_score = self.stocks.get(op).unwrap_or(&0);
                let other_score = state.stocks.get(op).unwrap_or(&0);

                if my_score < other_score {
                    return false;
                }
                if my_score > other_score {
                    return true;
                }
            }
        }

        if optimize.contains(&"time".to_string()) {
            if self.curr_cycle < state.curr_cycle {
                return true;
            }
            if self.curr_cycle > state.curr_cycle {
                return false;
            }
        }

        false
    }
}

fn search(
    deadline: Instant,
    mut state: State,
    processes: &[Process],
    mut idx: usize,
    optimize: &[String],
    best: &mut Option<State>
) {
    loop {
        if Instant::now() >= deadline || state.curr_cycle >= usize::MAX {
            let mut final_state = state.clone();
            final_state.finish_all();
            if best.is_none() || final_state.better_then(best.as_ref().unwrap(), optimize) {
                *best = Some(final_state);
            }
            return;
        }

        let mut startable_indices: Vec<usize> = Vec::new();
        for (i, process) in processes.iter().enumerate() {
            if process.can_start(&state.stocks) && i >= idx {
                startable_indices.push(i);
            }
        }

        let can_advance = state.get_next_cycle().is_some();
        let startable_amount = startable_indices.len();
        let total_choices = startable_amount + (can_advance as usize);

        if total_choices == 0 {
            if best.is_none() || state.better_then(best.as_ref().unwrap(), optimize) {
                *best = Some(state);
            }
            return;
        }

        if total_choices > 1 {
            for i in startable_indices {
                let mut child = state.clone();
                if child.start_by_name(&processes[i].name, processes).is_ok() {
                    search(deadline, child, processes, i, optimize, best);
                }
            }

            if let Some(next_cycle) = state.get_next_cycle() {
                let mut child = state.clone();
                if child.advance_to(next_cycle).is_ok() {
                    search(deadline, child, processes, 0, optimize, best);
                }
            }
            return;
        }

        if startable_amount == 1 {
            let i = startable_indices[0];
            let _ = state.start_by_name(&processes[i].name, processes);
            idx = i;
        } else {
            let next_cycle = state.get_next_cycle().unwrap();
            let _ = state.advance_to(next_cycle);
            idx = 0;
        }
    }
}

pub fn simulate(config: Config, filename: &str, time: f64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs_f64(time);

    let initial_state = State::new(config.stocks.clone());
    let mut best: Option<State> = None;

    search(deadline, initial_state, &config.processes, 0, &config.optimize, &mut best);

    match best {
        Some(final_state) => {
            let all_stock_names = get_all_stock_names(&config);
            final_state.print_result(&all_stock_names);
            final_state.log_history(filename)
        }
        None => { Err(format!("No complete schedule found within {}s.", time)) }
    }
}

fn get_all_stock_names(config: &Config) -> Vec<String> {
    let mut names = HashSet::new();

    for name in config.stocks.keys() {
        names.insert(name.clone());
    }

    for p in &config.processes {
        for name in p.needs.keys() {
            names.insert(name.clone());
        }
        for name in p.results.keys() {
            names.insert(name.clone());
        }
    }

    names.into_iter().collect()
}
