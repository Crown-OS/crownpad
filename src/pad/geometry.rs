use crownui::prelude::{Point, Size};

use crate::settings::{Browsing, GridShape};

pub const SEARCH_TOP: f32 = 44.0;
pub const SEARCH_WIDTH: f32 = 240.0;
pub const SEARCH_HEIGHT: f32 = 32.0;
pub const LABEL_HEIGHT: f32 = 18.0;
pub const LABEL_GAP: f32 = 8.0;
const GRID_TOP: f32 = SEARCH_TOP + SEARCH_HEIGHT + 32.0;
const DOTS_AREA: f32 = 72.0;
const GRID_WIDTH_SHARE: f32 = 0.84;
const MAX_CELL_WIDTH: f32 = 168.0;
const ICON_SHARE_OF_CELL: f32 = 0.62;
const MIN_ICON: f32 = 32.0;
const MAX_ICON: f32 = 128.0;
const MERGE_ZONE: f32 = 0.32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub surface: Size,
    pub shape: GridShape,
    pub browsing: Browsing,
    pub cell: Size,
    pub icon: f32,
    origin: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Before(usize),
    Onto(usize),
}

impl Geometry {
    pub fn new(surface: Size, shape: GridShape, browsing: Browsing) -> Self {
        let columns = f32::from(shape.columns);
        let rows = f32::from(shape.rows);
        let width = (surface.width * GRID_WIDTH_SHARE)
            .min(MAX_CELL_WIDTH * columns)
            .max(columns);
        let height = (surface.height - GRID_TOP - DOTS_AREA).max(rows);
        let cell = Size::new(width / columns, height / rows);
        let icon = (cell.width * ICON_SHARE_OF_CELL)
            .min(cell.height - LABEL_HEIGHT - LABEL_GAP)
            .clamp(MIN_ICON, MAX_ICON)
            .floor();
        Self {
            surface,
            shape,
            browsing,
            cell,
            icon,
            origin: Point::new((surface.width - width) / 2.0, GRID_TOP),
        }
    }

    pub fn capacity(&self) -> usize {
        self.shape.capacity()
    }

    pub fn page_width(&self) -> f32 {
        self.surface.width
    }

    pub fn grid_top(&self) -> f32 {
        self.origin.y
    }

    pub fn grid_bottom(&self) -> f32 {
        self.surface.height - DOTS_AREA
    }

    pub fn viewport_height(&self) -> f32 {
        self.grid_bottom() - self.grid_top()
    }

    pub fn cell_origin(&self, page: usize, index: usize) -> Point {
        let columns = usize::from(self.shape.columns);
        let (row, column) = match self.browsing {
            Browsing::Paged => (index / columns, index % columns),
            Browsing::Scroll => ((page * self.capacity() + index) / columns, index % columns),
        };
        let page_offset = match self.browsing {
            Browsing::Paged => page as f32 * self.page_width(),
            Browsing::Scroll => 0.0,
        };
        Point::new(
            page_offset + self.origin.x + column as f32 * self.cell.width,
            self.origin.y + row as f32 * self.cell.height,
        )
    }

    pub fn content_height(&self, items: usize) -> f32 {
        let columns = usize::from(self.shape.columns);
        items.div_ceil(columns) as f32 * self.cell.height
    }

    pub fn scroll_limit(&self, items: usize) -> f32 {
        (self.content_height(items) - self.viewport_height()).max(0.0)
    }

    pub fn target_at(&self, point: Point, len: usize) -> Option<Target> {
        let local = Point::new(point.x - self.origin.x, point.y - self.origin.y);
        let columns = usize::from(self.shape.columns);
        if local.x < 0.0 || local.y < 0.0 || local.x >= self.cell.width * columns as f32 {
            return None;
        }
        let column = (local.x / self.cell.width) as usize;
        let row = (local.y / self.cell.height) as usize;
        if self.browsing == Browsing::Paged && row >= usize::from(self.shape.rows) {
            return None;
        }
        let index = row * columns + column;
        if index >= len {
            return Some(Target::Before(len));
        }
        let within = (local.x - column as f32 * self.cell.width) / self.cell.width;
        let centred = (within - 0.5).abs() < MERGE_ZONE;
        Some(if centred {
            Target::Onto(index)
        } else if within < 0.5 {
            Target::Before(index)
        } else {
            Target::Before(index + 1)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> Geometry {
        Geometry::new(
            Size::new(1920.0, 1080.0),
            GridShape {
                rows: 5,
                columns: 7,
            },
            Browsing::Paged,
        )
    }

    #[test]
    fn icon_fits_inside_its_cell_with_room_for_the_label() {
        let geometry = geometry();
        assert!(geometry.icon <= geometry.cell.width);
        assert!(geometry.icon + LABEL_GAP + LABEL_HEIGHT <= geometry.cell.height);
        assert!((MIN_ICON..=MAX_ICON).contains(&geometry.icon));
    }

    #[test]
    fn wide_displays_cap_the_grid_width() {
        let wide = Geometry::new(
            Size::new(5120.0, 1440.0),
            GridShape {
                rows: 5,
                columns: 7,
            },
            Browsing::Paged,
        );
        assert_eq!(wide.cell.width, MAX_CELL_WIDTH);
    }

    #[test]
    fn pages_sit_side_by_side() {
        let geometry = geometry();
        let first = geometry.cell_origin(0, 3);
        let second = geometry.cell_origin(1, 3);
        assert_eq!(second.x - first.x, 1920.0);
        assert_eq!(second.y, first.y);
    }

    #[test]
    fn scroll_mode_stacks_pages_vertically() {
        let geometry = Geometry::new(
            Size::new(1920.0, 1080.0),
            GridShape {
                rows: 5,
                columns: 7,
            },
            Browsing::Scroll,
        );
        let last_of_first = geometry.cell_origin(0, 34);
        let first_of_second = geometry.cell_origin(1, 0);
        assert_eq!(first_of_second.y - last_of_first.y, geometry.cell.height);
        assert_eq!(first_of_second.x, geometry.cell_origin(0, 0).x);
    }

    #[test]
    fn centre_of_a_cell_merges_and_its_edges_insert() {
        let geometry = geometry();
        let cell = geometry.cell_origin(0, 2);
        let centre = Point::new(cell.x + geometry.cell.width / 2.0, cell.y + 10.0);
        let left = Point::new(cell.x + 2.0, cell.y + 10.0);
        let right = Point::new(cell.x + geometry.cell.width - 2.0, cell.y + 10.0);
        assert_eq!(geometry.target_at(centre, 10), Some(Target::Onto(2)));
        assert_eq!(geometry.target_at(left, 10), Some(Target::Before(2)));
        assert_eq!(geometry.target_at(right, 10), Some(Target::Before(3)));
    }

    #[test]
    fn empty_cells_append() {
        let geometry = geometry();
        let cell = geometry.cell_origin(0, 20);
        assert_eq!(
            geometry.target_at(Point::new(cell.x + 5.0, cell.y + 5.0), 4),
            Some(Target::Before(4))
        );
    }
}
