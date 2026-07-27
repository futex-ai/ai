//! MiniMax model implementation for the shared AI interface.

#![warn(unreachable_pub)]

mod catalog;
mod minimax;

pub use catalog::{
    MINIMAX_M2_7, MINIMAX_M2_7_HIGHSPEED, MINIMAX_M3, MINIMAX_M3_THINKING_DISABLED, known_models,
};
pub use minimax::MiniMaxModel;
