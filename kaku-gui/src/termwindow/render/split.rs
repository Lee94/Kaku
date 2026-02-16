use crate::termwindow::render::TripleLayerQuadAllocator;
use crate::termwindow::{UIItem, UIItemType};
use mux::pane::Pane;
use mux::tab::{PositionedSplit, SplitDirection};
use std::sync::Arc;

impl crate::TermWindow {
    pub fn paint_split(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
        split: &PositionedSplit,
        all_splits: &[PositionedSplit],
        pane: &Arc<dyn Pane>,
    ) -> anyhow::Result<()> {
        let palette = pane.palette();
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;

        let foreground = palette.split.to_linear();

        let border = self.get_os_border();
        let first_row_offset = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
            self.tab_bar_pixel_height()?
        } else {
            0.
        } + border.top.get() as f32;

        let (padding_left, padding_top) = self.padding_left_top();
        let pos_y = split.top as f32 * cell_height + first_row_offset + padding_top;
        let pos_x = split.left as f32 * cell_width + padding_left + border.left.get() as f32;

        let split_thickness = 2.0;
        let is_horizontal = split.direction == SplitDirection::Horizontal;

        // Check if another split crosses this one
        let crossing = all_splits.iter().any(|s| {
            s.direction != split.direction
                && if is_horizontal {
                    s.top >= split.top
                        && s.top < split.top + split.size
                        && s.left <= split.left
                        && split.left < s.left + s.size
                } else {
                    s.left >= split.left
                        && s.left < split.left + split.size
                        && s.top <= split.top
                        && split.top < s.top + s.size
                }
        });

        if is_horizontal {
            let extend = if crossing { 3.0 } else { 1.5 };
            let y_start = pos_y - (cell_height / 2.0) - extend * cell_height;
            let height = (1.0 + split.size as f32 + extend * 2.0) * cell_height;

            self.filled_rectangle(
                layers,
                2,
                euclid::rect(
                    pos_x + (cell_width / 2.0) - (split_thickness / 2.0),
                    y_start,
                    split_thickness,
                    height,
                ),
                foreground,
            )?;
        } else {
            let extend = if crossing { 4.0 } else { 2.0 };
            let x_start = pos_x - (cell_width / 2.0) - extend * cell_width;
            let width = (1.0 + split.size as f32 + extend * 2.0) * cell_width;

            self.filled_rectangle(
                layers,
                2,
                euclid::rect(
                    x_start,
                    pos_y + (cell_height / 2.0) - (split_thickness / 2.0),
                    width,
                    split_thickness,
                ),
                foreground,
            )?;
        }

        // UI item for hit testing
        let (x, y, width, height) = if is_horizontal {
            (
                border.left.get() as usize
                    + padding_left as usize
                    + (split.left * cell_width as usize),
                padding_top as usize + first_row_offset as usize + split.top * cell_height as usize,
                cell_width as usize,
                split.size * cell_height as usize,
            )
        } else {
            (
                border.left.get() as usize
                    + padding_left as usize
                    + (split.left * cell_width as usize),
                padding_top as usize + first_row_offset as usize + split.top * cell_height as usize,
                split.size * cell_width as usize,
                cell_height as usize,
            )
        };

        self.ui_items.push(UIItem {
            x,
            y,
            width,
            height,
            item_type: UIItemType::Split(split.clone()),
        });

        Ok(())
    }
}
