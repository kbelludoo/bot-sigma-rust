use crate::game::catalog::get_global_catalog;

#[derive(Default)]
pub struct RouteSolver;

impl RouteSolver {
    pub fn new() -> Self {
        Self
    }

    pub fn select_best_hunt(&self, level: u32) -> &'static str {
        get_global_catalog().get_optimal_hunt(level)
    }
}
