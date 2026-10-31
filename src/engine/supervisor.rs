use crate::game::bot::BotInstance;
use crate::game::types::{AccountConfig, AccountStatus};
use std::sync::Arc;

pub struct Supervisor {
    bots: Vec<Arc<BotInstance>>,
}

impl Supervisor {
    pub fn new(configs: Vec<AccountConfig>) -> Self {
        let mut bots = Vec::new();
        for cfg in configs {
            bots.push(Arc::new(BotInstance::new(cfg)));
        }
        Self { bots }
    }

    pub fn bots(&self) -> &[Arc<BotInstance>] {
        &self.bots
    }

    pub fn get_bot(&self, id: &str) -> Option<Arc<BotInstance>> {
        self.bots.iter().find(|b| b.config.id == id).cloned()
    }

    pub async fn get_all_statuses(&self) -> Vec<AccountStatus> {
        let mut statuses = Vec::with_capacity(self.bots.len());
        for b in &self.bots {
            statuses.push(b.get_status().await);
        }
        statuses
    }

    pub async fn start_all(&self) {
        for bot in &self.bots {
            let b = Arc::clone(bot);
            tokio::spawn(async move {
                b.run_loop().await;
            });
        }
    }
}
