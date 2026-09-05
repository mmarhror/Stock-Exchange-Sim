use std::collections::HashMap;
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

    fn print_result(&self) {
        println!("Main Processes :");
        for l in self.history.iter() {
            println!(" {l}");
        }
        println!("No more process doable at cycle {}", self.curr_cycle);
        println!("Stock :");

        for (name, qty) in self.stocks.iter() {
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
            if op == "time" {
                return self.curr_cycle < state.curr_cycle;
            }

            let my_stocks = &self.stocks;
            let other_stock = &self.stocks;
        }

        false
    }
}

fn search(
    max_cycles: usize,
    deadline: Instant,
    state: &State,
    processes: &[Process],
    idx: usize,
    best: &mut Option<State>
) {
    if Instant::now() >= deadline {
        return;
    }

    if state.curr_cycle >= max_cycles {
        let mut final_state = state.clone();
        final_state.finish_all();

        *best = Some(final_state);
        return;
    }

    let any_startable: bool = processes.iter().any(|p| p.can_start(&state.stocks));
    let any_running: bool = !state.running.is_empty();

    if !any_startable && !any_running {
        *best = Some(state.clone());
        return;
    }

    for (i, process) in processes.iter().enumerate() {
        let mut child = state.clone();

        if process.can_start(&state.stocks) && i >= idx {
            if child.start_by_name(&process.name, processes).is_ok() {
                search(max_cycles, deadline, &child, processes, i, best);
            }
        }
    }

    if any_running {
        if let Some(next_cycle) = state.get_next_cycle() {
            let mut child = state.clone();
            if child.advance_to(next_cycle).is_ok() {
                search(max_cycles, deadline, &child, processes, 0, best);
            }
        }
    }
}

pub fn simulate(config: Config, filename: &str, time: usize) {
    let deadline = Instant::now() + Duration::from_secs(time as u64);
    let initial_state = State::new(config.stocks.clone());
    let mut best: Option<State> = None;

    let max_cycles = 1000;

    search(max_cycles, deadline, &initial_state, &config.processes, 0, &mut best);

    match best {
        Some(final_state) => {
            final_state.print_result();
            if let Err(e) = final_state.log_history(filename) {
                eprintln!("{}", e);
            }
        }
        None => {
            println!("No complete schedule found within {}s.", time);
        }
    }
}
// 