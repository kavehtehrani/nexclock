pub mod clock;

use ratatui::{
    layout::{Constraint, Layout},
    Frame,
};

use crate::constants::DEFAULT_CLOCK_HEIGHT_PERCENT;

/// Root draw function: composes the full UI layout.
pub fn draw(frame: &mut Frame) {
    let area = frame.area();

    // For Phase 1: clock takes its configured percentage, rest is empty.
    // Later phases will add info panels and a status bar.
    let chunks = Layout::vertical([
        Constraint::Percentage(DEFAULT_CLOCK_HEIGHT_PERCENT),
        Constraint::Min(0),
    ])
    .split(area);

    clock::render(frame, chunks[0]);
}
