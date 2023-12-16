//! Shove glyphs from a variable font into a Lottie template.

pub mod error;

use bodymovin::{layers::AnyLayer, Bodymovin as Lottie};

use crate::error::Error;

pub trait Template {
    fn replace_shape(&mut self) -> Result<(), Error>;
}

impl Template for Lottie {
    fn replace_shape(&mut self) -> Result<(), Error> {
        for layer in &self.layers {
            let AnyLayer::Shape(layer) = layer else {
                continue;
            };
            let Some(name) = &layer.name else {
                continue;
            };
            eprintln!("Found a shape named '{name}'");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
