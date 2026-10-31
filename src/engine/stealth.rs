use rand::Rng;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StealthConfig {
    pub enabled: bool,
    pub min_action_delay_ms: u64,
    pub max_action_delay_ms: u64,
    pub jitter_variance: f64,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_action_delay_ms: 300,
            max_action_delay_ms: 600,
            jitter_variance: 0.20,
        }
    }
}

pub struct StealthEngine {
    config: StealthConfig,
}

impl StealthEngine {
    pub fn new(config: StealthConfig) -> Self {
        Self { config }
    }

    /// Retorna um delay com jitter randômico orgânico (evita relógio mecânico sem parar a hunt)
    pub fn jitter_delay(&self, base: Duration) -> Duration {
        if !self.config.enabled || self.config.jitter_variance <= 0.0 {
            return base;
        }

        let mut rng = rand::thread_rng();
        let factor = 1.0 + rng.gen_range(-self.config.jitter_variance..=self.config.jitter_variance);
        let millis = (base.as_millis() as f64 * factor).max(50.0);
        Duration::from_millis(millis as u64)
    }

    /// Retorna o atraso humano orgânico entre cliques consecutivos (300ms a 600ms)
    pub fn human_action_delay(&self) -> Duration {
        if !self.config.enabled {
            return Duration::from_millis(50);
        }
        let mut rng = rand::thread_rng();
        let ms = rng.gen_range(self.config.min_action_delay_ms..=self.config.max_action_delay_ms);
        Duration::from_millis(ms)
    }

    /// Executa o pacing humano entre requisições em lote (anti-burst)
    pub async fn pacing_step(&self) {
        let delay = self.human_action_delay();
        tokio::time::sleep(delay).await;
    }
}
