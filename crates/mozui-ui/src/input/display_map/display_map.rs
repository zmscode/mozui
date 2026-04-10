#![allow(dead_code)]
/// DisplayMap: Public facade for Input display mapping.
///
/// This wraps WrapMap to provide a unified API:
/// - BufferPoint ↔ DisplayPoint conversion
/// - Automatic projection updates on text/layout changes
use std::ops::Range;

use mozui::{App, Font, Pixels};
use ropey::Rope;

use super::text_wrapper::{LineItem, WrapDisplayPoint};
use super::wrap_map::WrapMap;
use super::{BufferPoint, DisplayPoint};
use crate::input::display_map::WrapPoint;

use crate::input::Point as TreeSitterPoint;

/// DisplayMap is the main interface for Input coordinate mapping.
///
/// It manages the wrapping projection:
/// Buffer → Wrap (soft-wrapping) → Display
///
/// Without folding, wrap rows map 1:1 to display rows.
pub struct DisplayMap {
    wrap_map: WrapMap,
}

impl DisplayMap {
    pub fn new(font: Font, font_size: Pixels, wrap_width: Option<Pixels>) -> Self {
        Self {
            wrap_map: WrapMap::new(font, font_size, wrap_width),
        }
    }

    // ==================== Core Coordinate Mapping ====================

    /// Convert buffer position to display position
    pub fn buffer_pos_to_display_pos(&self, pos: BufferPoint) -> DisplayPoint {
        let wrap_pos = self.wrap_map.buffer_pos_to_wrap_pos(pos);
        DisplayPoint::new(wrap_pos.row, wrap_pos.col)
    }

    /// Convert display position to buffer position
    pub fn display_pos_to_buffer_pos(&self, pos: DisplayPoint) -> BufferPoint {
        let wrap_pos = WrapPoint::new(pos.row, pos.col);
        self.wrap_map.wrap_pos_to_buffer_pos(wrap_pos)
    }

    /// Get total number of visible display rows
    #[inline]
    pub fn display_row_count(&self) -> usize {
        self.wrap_map.wrap_row_count()
    }

    /// Get the buffer line for a given display row
    pub fn display_row_to_buffer_line(&self, display_row: usize) -> usize {
        self.wrap_map.wrap_row_to_buffer_line(display_row)
    }

    /// Get the display row range for a buffer line: [start, end)
    pub fn buffer_line_to_display_row_range(&self, line: usize) -> Option<Range<usize>> {
        let wrap_row_range = self.wrap_map.buffer_line_to_wrap_row_range(line);
        let start = wrap_row_range.start;
        let end = wrap_row_range.end;
        if start >= end { None } else { Some(start..end) }
    }

    /// Check if a buffer line is completely hidden (always false without folding)
    #[inline]
    pub fn is_buffer_line_hidden(&self, _line: usize) -> bool {
        false
    }

    // ==================== Text and Layout Updates ====================

    /// Update text (incremental or full)
    pub fn on_text_changed(
        &mut self,
        changed_text: &Rope,
        range: &Range<usize>,
        new_text: &Rope,
        cx: &mut App,
    ) {
        self.wrap_map
            .on_text_changed(changed_text, range, new_text, cx);
    }

    /// Update layout parameters (wrap width or font)
    pub fn on_layout_changed(&mut self, wrap_width: Option<Pixels>, cx: &mut App) {
        self.wrap_map.on_layout_changed(wrap_width, cx);
    }

    /// Set font parameters
    pub fn set_font(&mut self, font: Font, font_size: Pixels, cx: &mut App) {
        self.wrap_map.set_font(font, font_size, cx);
    }

    /// Ensure text is prepared (initializes wrapper if needed)
    pub fn ensure_text_prepared(&mut self, text: &Rope, cx: &mut App) {
        self.wrap_map.ensure_text_prepared(text, cx);
    }

    /// Initialize with text
    pub fn set_text(&mut self, text: &Rope, cx: &mut App) {
        self.wrap_map.set_text(text, cx);
    }

    // ==================== Wrap Display Point Operations ====================

    /// Convert byte offset to wrap display point (with soft wrap info).
    #[inline]
    pub(crate) fn offset_to_wrap_display_point(&self, offset: usize) -> WrapDisplayPoint {
        self.wrap_map.wrapper().offset_to_display_point(offset)
    }

    /// Convert wrap display point to byte offset.
    #[inline]
    pub(crate) fn wrap_display_point_to_offset(&self, point: WrapDisplayPoint) -> usize {
        self.wrap_map.wrapper().display_point_to_offset(point)
    }

    /// Convert wrap display point to TreeSitterPoint (buffer line/col).
    #[inline]
    pub(crate) fn wrap_display_point_to_point(&self, point: WrapDisplayPoint) -> TreeSitterPoint {
        self.wrap_map.wrapper().display_point_to_point(point)
    }

    /// Wrap row to display row (identity without folding).
    #[inline]
    pub fn wrap_row_to_display_row(&self, wrap_row: usize) -> Option<usize> {
        Some(wrap_row)
    }

    /// Find the nearest visible display row for a given wrap row (identity without folding).
    #[inline]
    pub fn nearest_visible_display_row(&self, wrap_row: usize) -> usize {
        wrap_row
    }

    /// Convert a display row to a wrap row (identity without folding).
    #[inline]
    pub fn display_row_to_wrap_row(&self, display_row: usize) -> Option<usize> {
        Some(display_row)
    }

    /// Get the longest row index (by byte length).
    #[inline]
    pub(crate) fn longest_row(&self) -> usize {
        self.wrap_map.wrapper().longest_row.row
    }

    // ==================== Access Methods ====================

    /// Get access to line items (for rendering)
    #[inline]
    pub(crate) fn lines(&self) -> &[LineItem] {
        self.wrap_map.lines()
    }

    /// Get the rope text
    #[inline]
    pub fn text(&self) -> &Rope {
        self.wrap_map.text()
    }

    /// Get the visible wrap row count for a buffer line (all rows visible without folding)
    #[inline]
    pub fn visible_wrap_row_count_for_buffer_line(&self, line: usize) -> usize {
        let range = self.wrap_map.buffer_line_to_wrap_row_range(line);
        range.end - range.start
    }

    /// Get the wrap row count
    #[inline]
    pub fn wrap_row_count(&self) -> usize {
        self.wrap_map.wrap_row_count()
    }

    /// Get the buffer line count (logical lines)
    #[inline]
    pub fn buffer_line_count(&self) -> usize {
        self.wrap_map.buffer_line_count()
    }
}
