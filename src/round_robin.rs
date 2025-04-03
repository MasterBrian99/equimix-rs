use std::sync::{Arc, RwLock};

use crate::Servers;

#[derive(Debug,Clone)]
pub struct RoundRobinServers {
    pub url: String,
    pub  healthy: bool,
    pub health_check_path:String
}

#[derive(Debug)]
pub struct RoundRobin {
    servers: Arc<RwLock<Vec<RoundRobinServers>>>,
    current: RwLock<usize>, // RwLock is magic
    total: usize,
}

impl RoundRobin {
    pub fn new(servers: Vec<Servers>) -> Self {
        RoundRobin {
            current: RwLock::new(0), // Initialize current with RwLock
            total: servers.len(),
            servers: Arc::new(RwLock::new(
                servers.into_iter()
                    .map(|server| RoundRobinServers { url:server.url, healthy: true, health_check_path:server.health_check_path })
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

    pub fn get_all_servers(&self) -> Option<Vec<RoundRobinServers>> {
        let servers = self.servers.read().unwrap();
        if servers.is_empty() {
            return None;
        }
        Some(servers.clone())
    }

    pub fn update_healthy(&self, index:usize, healthy: bool) {
        let mut servers = self.servers.write().unwrap();
        let server= &mut servers[index];
        server.healthy = healthy;
    }
}
