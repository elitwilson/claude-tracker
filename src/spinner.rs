//! Animated spinner for the refresh indicator.

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct Spinner {
    frame: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % FRAMES.len();
    }

    pub fn reset(&mut self) {
        self.frame = 0;
    }

    pub fn current(&self) -> char {
        FRAMES[self.frame]
    }
}

#[cfg(test)]
mod tests;
