mod poll;
mod utils;

pub use poll::*;

use ahash::RandomState;

thread_local! {
    pub static MAIN_HASHER: RandomState = RandomState::with_seed(29384);
}
