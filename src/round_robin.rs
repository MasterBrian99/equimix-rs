use std::{
    fmt::Error,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct RoundRobinServers {
    url: String,
    healthy: bool,
}
impl RoundRobinServers {
    fn new(url: String) -> Self {
        RoundRobinServers { url, healthy: true }
    }
}

#[derive(Debug)]
pub struct RoundRobin {
    servers: Arc<RwLock<Vec<RoundRobinServers>>>,
    current: usize,
    total: usize,
}

impl RoundRobin {
    pub fn new(urls: Vec<String>) -> Self {
        RoundRobin {
            current: 0,
            total: urls.len(),
            servers: Arc::new(RwLock::new(
                urls.into_iter()
                    .map(|url| RoundRobinServers { url, healthy: true })
                    .collect(),
            )),
        }
    }

    pub async fn get_next_server(&mut self) -> Option<String> {
        let servers = self.servers.read().unwrap();
        if servers.is_empty() {
            return None;
        }

        let mut attempts = 0;
        let len = self.total;

        while attempts < len {
            let index = self.current;
            self.current = (self.current + 1) % len;
            attempts += 1;

            if servers[index].healthy {
                return Some(servers[index].url.clone());
            }
        }

        None
    }
}
