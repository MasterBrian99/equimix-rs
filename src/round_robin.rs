use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct RoundRobinServers {
    url: String,
    healthy: bool,
}

#[derive(Debug)]
pub struct RoundRobin {
    servers: Arc<RwLock<Vec<RoundRobinServers>>>,
    current: RwLock<usize>, // Use RwLock for mutable current
    total: usize,
}

impl RoundRobin {
    pub fn new(urls: Vec<String>) -> Self {
        RoundRobin {
            current: RwLock::new(0), // Initialize current with RwLock
            total: urls.len(),
            servers: Arc::new(RwLock::new(
                urls.into_iter()
                    .map(|url| RoundRobinServers { url, healthy: true })
                    .collect(),
            )),
        }
    }

    pub async fn get_next_server(&self) -> Option<String> {
        let servers = self.servers.read().unwrap();
        if servers.is_empty() {
            return None;
        }

        let mut attempts = 0;
        let len = self.total;

        while attempts < len {
            let index = *self.current.read().unwrap(); // Read current
            let mut current = self.current.write().unwrap(); // Write current
            *current = (*current + 1) % len; // update current
            attempts += 1;

            if servers[index].healthy {
                return Some(servers[index].url.clone());
            }
        }

        None
    }
}
