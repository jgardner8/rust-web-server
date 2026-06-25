use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::ops::AddAssign;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

pub struct RateLimiter<T> {
    request_limit: usize,
    window_sec: u16,
    start_time: RwLock<Instant>,
    request_log: Mutex<HashMap<T, VecDeque<u16>>>,
}

impl<T> RateLimiter<T>
where
    T: Hash + Eq,
{
    pub fn new(request_limit: usize, window_sec: u16) -> Self {
        RateLimiter {
            request_limit,
            window_sec,
            start_time: RwLock::new(Instant::now()),
            request_log: Mutex::new(HashMap::new()),
        }
    }

    fn get_sec_since_start(&self) -> u16 {
        let sec_since_start = self
            .start_time
            .read()
            .expect("RwLock poisoned")
            .elapsed()
            .as_secs();

        if sec_since_start > u16::MAX as u64 {
            // Clear request log every 65536 seconds or approx 18 hours to avoid unbounded memory usage
            let mut request_log = self.request_log.lock().expect("Mutex poisoned");
            request_log.clear();

            self.start_time
                .write()
                .expect("RwLock poisoned")
                .add_assign(Duration::from_secs(u16::MAX as u64))
        }

        sec_since_start as u16
    }

    fn drop_requests_outside_window(
        &self,
        request_times: &mut VecDeque<u16>,
        sec_since_start: u16,
    ) {
        let window_start = sec_since_start.saturating_sub(self.window_sec);

        while !request_times.is_empty() {
            if request_times[0] < window_start {
                request_times.pop_front();
            } else {
                break;
            }
        }
    }

    fn client_over_rate_limit_at_time(&self, client: T, sec_since_start: u16) -> bool {
        let mut request_log = self.request_log.lock().expect("Mutex poisoned");

        match request_log.entry(client) {
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::from([sec_since_start]));
                false
            }
            Entry::Occupied(mut entry) => {
                let request_times = entry.get_mut();

                self.drop_requests_outside_window(request_times, sec_since_start);

                if request_times.len() >= self.request_limit {
                    true
                } else {
                    request_times.push_back(sec_since_start);
                    false
                }
            }
        }
    }

    pub fn client_over_rate_limit(&self, client: T) -> bool {
        let sec_since_start = self.get_sec_since_start();
        self.client_over_rate_limit_at_time(client, sec_since_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let rate_limiter = RateLimiter::new(5, 60);

        let user_id = "john";

        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 0));
        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 56));
        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 57));
        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 58));
        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 59));
        assert!(rate_limiter.client_over_rate_limit_at_time(user_id, 60));
        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 61));
        assert!(rate_limiter.client_over_rate_limit_at_time(user_id, 61));
        assert!(rate_limiter.client_over_rate_limit_at_time(user_id, 61));
        assert!(rate_limiter.client_over_rate_limit_at_time(user_id, 61));
        assert!(rate_limiter.client_over_rate_limit_at_time(user_id, 61));
        assert!(rate_limiter.client_over_rate_limit_at_time(user_id, 61));
        assert!(!rate_limiter.client_over_rate_limit_at_time(user_id, 117));
    }
}
