use ratatui::layout::Rect;

use crate::component::GridPlacement;
use crate::config::GridConfig;

/// Computes a 2D array of cell rects from the grid configuration and available area.
pub fn compute_grid(area: Rect, grid: &GridConfig) -> Vec<Vec<Rect>> {
    let col_widths = resolve_sizes(area.width, grid.columns, grid.column_widths.as_deref());
    let row_heights = resolve_sizes(area.height, grid.rows, grid.row_heights.as_deref());

    let mut cells = Vec::with_capacity(grid.rows as usize);
    let mut y = area.y;

    for rh in &row_heights {
        let mut row = Vec::with_capacity(grid.columns as usize);
        let mut x = area.x;

        for cw in &col_widths {
            row.push(Rect {
                x,
                y,
                width: *cw,
                height: *rh,
            });
            x += cw;
        }

        cells.push(row);
        y += rh;
    }

    cells
}

/// Computes the merged rect for a component that spans multiple grid cells.
/// Returns None if the placement is out of bounds.
pub fn merged_rect(cells: &[Vec<Rect>], placement: &GridPlacement) -> Option<Rect> {
    let r = placement.row as usize;
    let c = placement.column as usize;

    if r >= cells.len() || c >= cells.first().map_or(0, |row| row.len()) {
        return None;
    }

    let top_left = cells[r][c];

    let end_row = (r + placement.row_span as usize).min(cells.len()) - 1;
    let end_col = (c + placement.col_span as usize).min(cells[0].len()) - 1;

    let bottom_right = cells[end_row][end_col];

    Some(Rect {
        x: top_left.x,
        y: top_left.y,
        width: (bottom_right.x + bottom_right.width).saturating_sub(top_left.x),
        height: (bottom_right.y + bottom_right.height).saturating_sub(top_left.y),
    })
}

/// Distributes `total` pixels across `count` segments according to optional percentages.
/// If percentages are omitted, distributes evenly.
fn resolve_sizes(total: u16, count: u16, percentages: Option<&[u16]>) -> Vec<u16> {
    if count == 0 {
        return vec![];
    }

    match percentages {
        Some(pcts) if pcts.len() == count as usize => {
            let pct_sum: u16 = pcts.iter().sum();
            let mut sizes: Vec<u16> = pcts
                .iter()
                .map(|&p| (u32::from(total) * u32::from(p) / u32::from(pct_sum.max(1))) as u16)
                .collect();

            // Distribute rounding remainder to the last segment
            let allocated: u16 = sizes.iter().sum();
            if let Some(last) = sizes.last_mut() {
                *last += total.saturating_sub(allocated);
            }

            sizes
        }
        _ => {
            // Even distribution
            let base = total / count;
            let remainder = total % count;
            let mut sizes = vec![base; count as usize];
            // Give extra pixels to the last segments
            for i in 0..remainder as usize {
                sizes[count as usize - 1 - i] += 1;
            }
            sizes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_clock_rect_spans_only_row_0() {
        let grid = GridConfig {
            rows: 3,
            columns: 2,
            row_heights: Some(vec![40, 30, 30]),
            column_widths: None,
        };
        let content_area = Rect { x: 0, y: 0, width: 117, height: 35 };
        let cells = compute_grid(content_area, &grid);

        // Main clock: row=0, col=0, row_span=1, col_span=2
        let placement = GridPlacement { row: 0, column: 0, row_span: 1, col_span: 2 };
        let rect = merged_rect(&cells, &placement).unwrap();

        assert_eq!(rect.y, 0);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.width, 117);
        assert!(rect.height <= 15, "main_clock height should be ~14, got {}", rect.height);

        // NY clock: row=1, col=0
        let ny = GridPlacement { row: 1, column: 0, row_span: 1, col_span: 1 };
        let ny_rect = merged_rect(&cells, &ny).unwrap();
        assert!(rect.y + rect.height <= ny_rect.y, "overlap between main_clock and ny_clock");
    }

}
