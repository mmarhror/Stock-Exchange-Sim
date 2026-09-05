//! Stock Exchange Simulator
//!
//! This module implements a priority-based process scheduler that explores all
//! valid execution schedules using a hybrid branch-and-bound search. It finds
//! the optimal schedule within a real-world wall-clock time limit.
//!
//! # Algorithm Overview
//!
//! The search explores a tree of possible schedules. At each state it can:
//! - **Start** an affordable process (consumes resources, adds a running task,
//!   does NOT advance time — so multiple processes can begin at the same cycle).
//! - **Advance** time to the next task completion (releases resources from
//!   finished processes).
//!
//! To avoid exploring duplicate permutations (e.g., starting A then B vs B then
//! A at the same cycle), an index constraint enforces non-decreasing process
//! order within each cycle.
//!
//! When only one move is available (no branching), the search mutates the state
//! in-place inside a loop instead of cloning and recursing. This prevents stack
//! overflow on self-powering configs that run for millions of cycles.

use std::collections::{ HashMap, HashSet };
use std::fs;
use std::path::Path;
use std::time::{ Duration, Instant };

use crate::parser::{ Config, Process };

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// A process that is currently running and will finish at a specific cycle.
#[derive(Clone)]
pub struct RunningProcess {
    pub process: Process,
    pub finish_cycle: usize,
}

/// A snapshot of the simulation at a specific point in time.
///
/// Tracks the current cycle, all running processes, available stock quantities,
/// and the history of actions taken to reach this state.
#[derive(Clone)]
pub struct State {
    pub curr_cycle: usize,
    pub running: Vec<RunningProcess>,
    pub stocks: HashMap<String, usize>,
    pub history: Vec<String>,
}

// ---------------------------------------------------------------------------
// State transitions
// ---------------------------------------------------------------------------

impl State {
    /// Creates a new initial state at cycle 0 with the given starting stocks.
    pub fn new(stocks: HashMap<String, usize>) -> Self {
        State {
            curr_cycle: 0,
            running: Vec::new(),
            stocks,
            history: Vec::new(),
        }
    }

    /// Returns the earliest cycle at which a running process will finish,
    /// or `None` if nothing is currently running.
    fn earliest_finish_cycle(&self) -> Option<usize> {
        self.running
            .iter()
            .map(|r| r.finish_cycle)
            .min()
    }

    /// Finishes all processes whose `finish_cycle` matches `curr_cycle`,
    /// removing them from the running list and adding their results to stock.
    fn finish_completed(&mut self) {
        let (done, still): (Vec<RunningProcess>, Vec<RunningProcess>) = self.running
            .drain(..)
            .partition(|r| r.finish_cycle == self.curr_cycle);

        self.running = still;

        for r in done {
            r.process.finish(&mut self.stocks);
        }
    }

    /// Advances the simulation clock to the given cycle, finishing any
    /// processes that complete along the way.
    ///
    /// Returns an error if the target cycle is in the past.
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

        // Step cycle-by-cycle so intermediate finishes are processed correctly.
        while self.curr_cycle < cycle {
            self.curr_cycle += 1;
            self.finish_completed();
        }

        Ok(())
    }

    /// Starts a single instance of the named process if it is affordable.
    ///
    /// Consumes the process's resource needs from stock, adds it to the running
    /// list with the correct finish cycle, and records the action in history.
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

    /// Force-finishes all remaining running processes regardless of their
    /// scheduled finish cycle. Used when the search hits a timeout or cycle
    /// limit to collect final stock values.
    pub fn finish_all_remaining(&mut self) {
        while let Some(next_cycle) = self.earliest_finish_cycle() {
            let _ = self.advance_to(next_cycle);
        }
    }

    /// Returns `true` if this state is strictly better than `other` according
    /// to the optimization targets.
    ///
    /// Stock targets are compared first (higher is better). The `"time"` target
    /// is always evaluated last as a tie-breaker (lower is better). This ensures
    /// that a schedule producing more output is preferred even if it takes longer.
    fn is_better_than(&self, other: &Self, optimize: &[String]) -> bool {
        // Phase 1: Compare all stock targets (higher is better).
        for op in optimize {
            if op != "time" {
                let my_score = self.stocks.get(op).unwrap_or(&0);
                let other_score = other.stocks.get(op).unwrap_or(&0);

                if my_score < other_score {
                    return false;
                }
                if my_score > other_score {
                    return true;
                }
            }
        }

        // Phase 2: Use time as the final tie-breaker (lower is better).
        if optimize.iter().any(|op| op == "time") {
            if self.curr_cycle < other.curr_cycle {
                return true;
            }
            if self.curr_cycle > other.curr_cycle {
                return false;
            }
        }

        false
    }
}

// ---------------------------------------------------------------------------
// Move discovery
// ---------------------------------------------------------------------------

/// A snapshot of all legal moves available from a given state.
struct Moves {
    /// Indices of processes (in config order) that can afford to start,
    /// filtered by the duplicate-prevention index constraint.
    startable: Vec<usize>,
    /// Whether advancing time is possible (i.e., at least one process is running).
    can_advance: bool,
}

impl Moves {
    /// Discovers all legal moves from the current state.
    ///
    /// Only considers processes at or after `idx` to prevent exploring
    /// duplicate permutations within the same cycle.
    fn new(state: &State, processes: &[Process], idx: usize) -> Self {
        let startable = processes
            .iter()
            .enumerate()
            .filter(|(i, p)| *i >= idx && p.can_start(&state.stocks))
            .map(|(i, _)| i)
            .collect();

        let can_advance = state.earliest_finish_cycle().is_some();

        Self {
            startable,
            can_advance,
        }
    }

    /// Total number of distinct moves (startable processes + advance option).
    fn total(&self) -> usize {
        self.startable.len() + (self.can_advance as usize)
    }
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Compares a completed schedule candidate against the current best,
/// updating `best` in-place if the candidate is superior.
fn update_champion(candidate: State, best: &mut Option<State>, optimize: &[String]) {
    if best.is_none() || candidate.is_better_than(best.as_ref().unwrap(), optimize) {
        *best = Some(candidate);
    }
}

/// Hybrid recursive/iterative search that explores all valid schedules.
///
/// - **Branching (2+ moves):** Clones the state and recurses for each option.
/// - **Forced (1 move):** Mutates the state in-place and loops, avoiding stack
///   overflow on long linear chains (e.g., self-powering infinite loops).
/// - **Terminal (0 moves):** Saves the state as a candidate champion.
///
/// The search stops when the real-world deadline expires, at which point the
/// best schedule found so far is returned.
fn search(
    deadline: Instant,
    mut state: State,
    processes: &[Process],
    mut idx: usize,
    optimize: &[String],
    best: &mut Option<State>
) {
    loop {
        // Check real-world timeout.
        if Instant::now() >= deadline {
            state.finish_all_remaining();
            update_champion(state, best, optimize);
            return;
        }

        let moves = Moves::new(&state, processes, idx);

        match moves.total() {
            // No moves left — schedule is complete.
            0 => {
                update_champion(state, best, optimize);
                return;
            }

            // Multiple moves — must branch via recursion.
            _ if moves.total() > 1 => {
                for i in &moves.startable {
                    let mut child = state.clone();
                    if child.start_by_name(&processes[*i].name, processes).is_ok() {
                        search(deadline, child, processes, *i, optimize, best);
                    }
                }
                if moves.can_advance {
                    let mut child = state.clone();
                    let next = state.earliest_finish_cycle().unwrap();
                    if child.advance_to(next).is_ok() {
                        search(deadline, child, processes, 0, optimize, best);
                    }
                }
                return;
            }

            // Single move — mutate in-place and loop (zero clones, zero stack growth).
            _ => {
                if moves.startable.len() == 1 {
                    let i = moves.startable[0];
                    let _ = state.start_by_name(&processes[i].name, processes);
                    idx = i;
                } else {
                    let next = state.earliest_finish_cycle().unwrap();
                    let _ = state.advance_to(next);
                    idx = 0;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reporting (I/O — intentionally separate from State)
// ---------------------------------------------------------------------------

/// Prints the final schedule summary to stdout, including the action history,
/// the terminal cycle, and all stock quantities (including zeros).
fn print_result(state: &State, all_stock_names: &[String]) {
    println!("Main Processes :");
    for line in &state.history {
        println!(" {line}");
    }
    println!("No more process doable at cycle {}", state.curr_cycle + 1);
    println!("Stock :");

    let mut sorted_names = all_stock_names.to_vec();
    sorted_names.sort();
    for name in sorted_names {
        let qty = state.stocks.get(&name).unwrap_or(&0);
        println!(" {name} => {qty}");
    }
}

/// Writes the action history to a `.log` file for the checker to validate.
fn log_history(state: &State, filename: &str) -> Result<(), String> {
    let path = Path::new(filename).with_extension("log");
    let content = state.history.join("\n") + "\n";
    fs::write(path, content).map_err(|e| format!("Failed to write log file: {}", e))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collects every unique stock name mentioned anywhere in the configuration
/// (initial stocks, process needs, and process results).
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Runs the full simulation: parses the config, searches for the optimal
/// schedule within the given time limit, prints the result, and writes the log.
///
/// # Arguments
/// * `config` — Parsed configuration containing stocks, processes, and targets.
/// * `filename` — Base path for the output `.log` file.
/// * `time` — Real-world time limit in seconds (supports fractional values).
pub fn simulate(config: Config, filename: &str, time: f64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs_f64(time);
    let initial_state = State::new(config.stocks.clone());
    let mut best: Option<State> = None;

    search(deadline, initial_state, &config.processes, 0, &config.optimize, &mut best);

    match best {
        Some(final_state) => {
            let all_stock_names = get_all_stock_names(&config);
            print_result(&final_state, &all_stock_names);
            log_history(&final_state, filename)
        }
        None => Err(format!("No complete schedule found within {}s.", time)),
    }
}
