use std::fmt::Error;

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
    servers: Vec<RoundRobinServers>,
    current: usize,
    total: usize,
}

impl RoundRobin {
    fn new(urls: Vec<String>) -> Self {
        RoundRobin {
            current: 0,
            total: urls.len(),
            servers: urls
                .into_iter()
                .map(|url| RoundRobinServers::new(url))
                .collect(),
        }
    }

    fn get_next(&mut self) -> Result<&mut RoundRobinServers, String> {
        let mut attempts = 0;

        while attempts < self.total {
            let next_index = self.current;
            self.current = (self.current + 1) % self.total;

            if self.servers[next_index].healthy {
                return Ok(&mut self.servers[next_index]);
            }
            attempts += 1;
        }
        Err("All servers are unhealthy".into())
    }
}
