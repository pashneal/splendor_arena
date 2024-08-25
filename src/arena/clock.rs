use std::time::Duration;
use std::time::SystemTime;
use super::*;
use warp::{Filter, Rejection, Reply};

/// Keeps track of the amount of time each player has left
pub struct Clock {
    total_time: Vec<Duration>,
    increment: Duration,
    current_timestamp: SystemTime,
    current_player: Option<usize>,
    timed_out: Vec<bool>,
}


impl Clock {
    pub fn new(num_players : usize, initial_time: Duration, increment: Duration) -> Clock {
        Clock {
            total_time: vec![initial_time; num_players],
            increment,
            current_timestamp: SystemTime::now(),
            current_player: None,
            timed_out: vec![false; num_players],
        }
    }

    pub fn next_player(&mut self) {
        if self.current_player.is_none() {
            self.current_player = Some(0);
            return;
        }
        let num_players = self.total_time.len();
        self.current_player = self.current_player.map(|x| {
            (x + 1) % num_players
        });
    }

    // Start the clock for the current player
    // If there is no current player, automatically set the current player to 0
    pub fn start(&mut self) {
        if  self.current_player.is_none() {
            self.current_player = Some(0);
        }
        let current_player = self.current_player.unwrap();
        self.current_timestamp = SystemTime::now();
        self.total_time[current_player] += self.increment;
    }

    // Returns the time remaining for the current player
    // If there is no current player, return 0
    pub fn time_remaining(&self) -> Duration {
        if self.current_player.is_none() {
            return Duration::from_secs(0);
        }
        let current_player = self.current_player.unwrap();
        if self.timed_out[current_player] {
            return Duration::from_secs(0);
        }
        if self.total_time[current_player] < self.current_timestamp.elapsed().unwrap() {
            return Duration::from_secs(0);
        }
        self.total_time[current_player] - self.current_timestamp.elapsed().unwrap()
    }

    // End the clock for the current player
    // If there is no current player, do nothing
    pub fn end(&mut self) {
        if self.current_player.is_none() {
            return;
        }
        let elapsed = self.current_timestamp.elapsed().unwrap();
        let current_player = self.current_player.unwrap();
        if self.total_time[current_player] < Duration::from_secs(0) {
            self.total_time[current_player] = Duration::from_secs(0);
        } else if self.total_time[current_player] < elapsed {
            self.timed_out[current_player] = true;
            self.total_time[current_player] = Duration::from_secs(0);
        } else {
            self.total_time[current_player] -= elapsed;
        }
    }
}

#[derive(Debug, Serialize)]
struct Response {
    time_remaining: Duration,
}


pub async fn current_time_remaining(arena: GlobalArena) -> Result<impl Reply, Rejection> {
    let time_remaining = arena.read().await.time_remaining();
    Ok(warp::reply::json(&Response {
        time_remaining,
    }))
}
