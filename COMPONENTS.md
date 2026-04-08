# mozui Component Implementation Plan

This plan covers implementing all components from [gpui-component](https://github.com/longbridge/gpui-component), adapted to mozui's architecture (signals, builder pattern, `dyn Element` trait, `&mut dyn Any` context).

**Architecture mapping (GPUI -> mozui):**
- `Entity<T>` -> signal-based state (`Signal<T>`, `SetSignal<T>`)
- `cx: &mut Window, &mut App` -> `cx: &mut dyn Any` (downcast to `Context`)
- `Styled` trait -> builder methods on each element
- `RenderOnce`/`Render` -> `Element` trait (`layout` + `paint`)
- `Sizable`/`Disableable`/`Selectable`/`Collapsible` -> already in `mozui-elements/src/styled.rs`
- Theme access -> `cx.downcast_ref::<Context>().theme()` or passed as parameter

**Crate:** All components go in `mozui-elements/src/` with re-exports through the `mozui` facade.

---

## Phase 1: Core Primitives (Foundation)

These are the building blocks that most other components depend on.

### 1.1 — Icon (Phosphor Icons)

**Crate:** `mozui-icons` (new crate for icon SVG data)
**File:** `mozui-elements/src/icon.rs` (element), `mozui-icons/src/lib.rs` (data)

**Purpose:** Render [Phosphor Icons](https://phosphoricons.com/) as SVGs. Required by buttons, alerts, menus, tooltips, notifications, etc.

**Icon source:** Phosphor Icons — MIT licensed, 6 weights (thin/light/regular/bold/fill/duotone), 1500+ icons.

**Architecture:**
1. **`mozui-icons` crate** — Bundles Phosphor SVG data as `&'static str` constants via `include_str!`. Each icon variant maps to its SVG file. Start with a curated core set (~80 icons), extensible.
2. **SVG rasterizer** — Uses `resvg` + `tiny-skia` to rasterize SVGs to alpha bitmaps on demand. Rasterized at the exact pixel size needed.
3. **Texture atlas caching** — Rasterized icons are uploaded to the existing glyph texture atlas and cached. Same rendering path as text glyphs (tinted alpha textures).
4. **`DrawCommand::Icon`** — New draw command for icon rendering, handled by the glyph pipeline.

**API:**
```rust
// mozui-icons crate
pub enum IconName {
    Check, CheckCircle, ChevronDown, ChevronRight, ChevronLeft, ChevronUp,
    X, XCircle, Plus, Minus, MagnifyingGlass, Gear, Info, Warning,
    WarningCircle, Copy, Trash, PencilSimple, Eye, EyeSlash,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    List, DotsThree, DotsThreeVertical,
    Star, StarFill, Sun, Moon, ArrowsOut, ArrowsIn, Minus as Minimize,
    ArrowSquareOut, LinkSimple, ClipboardText, Clock, Calendar,
    Envelope, User, House, File, Folder, DownloadSimple, UploadSimple,
    ArrowClockwise, Funnel, SortAscending, SortDescending,
    Lock, LockOpen, Bell, Heart, SpinnerGap, CaretDown, CaretUp,
    CaretLeft, CaretRight, SignOut, Sidebar as SidebarIcon,
    // ... extensible to full Phosphor set
}

pub enum IconWeight {
    Thin, Light, Regular, Bold, Fill, Duotone,
}

impl IconName {
    pub fn svg(&self, weight: IconWeight) -> &'static str; // raw SVG string
    pub fn path_data(&self, weight: IconWeight) -> &'static str; // SVG path `d` attribute
}

// mozui-elements crate
pub struct Icon { .. }

pub fn icon(name: IconName) -> Icon;
```

**Builder:** `.weight(IconWeight)`, `.size(ComponentSize)`, `.color(Color)`, `.rotation(f32)`, `.size_px(f32)`

**New dependencies:**
- `resvg` — SVG rendering
- `tiny-skia` — CPU rasterization backend for resvg

**Implementation notes:**
- Icons are monochrome — SVG `currentColor` replaced with white, rasterized to alpha mask
- Alpha mask uploaded to texture atlas, rendered with tint color (same as glyphs)
- Cache key: `(IconName, IconWeight, size_in_px)` — re-rasterize on size change
- Default weight: Regular. Default size: 16px (matches ComponentSize::Medium)
- Rotation applies a transform at paint time

**Dependencies:** mozui-icons, resvg, tiny-skia, texture atlas

---

### 1.2 — Label

**File:** `mozui-elements/src/label.rs`

**Purpose:** Styled text with optional highlight ranges for search matching. Used by list items, menus, selects, etc.

**API:**
```rust
pub struct Label { .. }

pub fn label(text: impl Into<String>) -> Label;
```

**Builder:** `.secondary()`, `.masked()`, `.highlights(ranges, mode)`, `.font_size(f32)`, `.color(Color)`, `.bold()`, `.italic()`

**Key types:**
```rust
pub enum HighlightMatch {
    Prefix,  // highlight from start
    Full,    // highlight exact ranges
}
```

**Implementation notes:**
- Wraps the existing `Text` element with additional highlight rendering
- Highlights render as colored background rects behind matched text ranges
- `.masked()` replaces text with bullet characters (for passwords)

**Dependencies:** Text element

---

### 1.3 — Divider

**File:** `mozui-elements/src/divider.rs`

**Purpose:** Visual separator line. Used in menus, lists, forms, etc.

**API:**
```rust
pub struct Divider { .. }

pub fn divider() -> Divider;
```

**Builder:** `.vertical()`, `.horizontal()`, `.dashed()`, `.label(text)`, `.color(Color)`

**Implementation notes:**
- Horizontal: full-width rect of 1px height
- Vertical: full-height rect of 1px width
- Dashed: series of short rects with gaps
- Label: centered text with divider lines on both sides

**Dependencies:** Text element

---

### 1.4 — Kbd (Keyboard Shortcut Display)

**File:** `mozui-elements/src/kbd.rs`

**Purpose:** Display keyboard shortcuts with platform-appropriate symbols. Used by tooltips, menus.

**API:**
```rust
pub struct Kbd { .. }

pub fn kbd(combo: &str) -> Kbd;
```

**Builder:** `.outline()`, `.font_size(f32)`

**Implementation notes:**
- Parse combo string (e.g., "cmd-s") using existing `KeyCombo::parse`
- Format with platform symbols: macOS uses ⌘⇧⌥⌃, others use Ctrl+Shift+Alt
- Render as small bordered badge

**Dependencies:** KeyCombo (mozui-app)

---

## Phase 2: Form Controls

### 2.1 — Button

**File:** `mozui-elements/src/button.rs`

**Purpose:** The fundamental interactive control. Required by dialogs, forms, menus, pagination, etc.

**API:**
```rust
pub enum ButtonVariant {
    Default, Primary, Secondary, Danger, Ghost, Link, Text,
    Success, Warning, Info,
    Custom { bg: Color, fg: Color, hover: Color, active: Color },
}

pub struct Button { .. }

pub fn button(label: impl Into<String>) -> Button;
```

**Builder:**
- Content: `.label(text)`, `.icon(IconName)`, `.icon_right(IconName)`
- Variant: `.primary()`, `.secondary()`, `.danger()`, `.ghost()`, `.link()`, `.with_variant(ButtonVariant)`
- State: `.disabled(bool)`, `.selected(bool)`, `.loading(bool)`
- Size: `.xsmall()`, `.small()`, `.large()`, `.with_size(ComponentSize)`
- Style: `.rounded(f32)`, `.compact()`, `.outline()`, `.w(f32)`, `.h(f32)`
- Events: `.on_click(handler)`, `.tooltip(text)`

**Implementation notes:**
- Renders a styled `Div` with icon + label children
- Theme-driven colors per variant
- Loading state shows a spinner icon, disables click
- Compact mode reduces horizontal padding
- Outline variant uses border instead of filled background
- Implements `Sizable`, `Disableable`, `Selectable`

**Dependencies:** Icon, Text, Spinner (for loading state)

---

### 2.2 — ButtonGroup

**File:** `mozui-elements/src/button.rs` (same file)

**API:**
```rust
pub struct ButtonGroup { .. }

pub fn button_group() -> ButtonGroup;
```

**Builder:** `.child(Button)`, `.children(Vec<Button>)`, `.compact()`, `.outline()`, `.with_variant(ButtonVariant)`, `.on_click(handler)`

**Implementation notes:**
- Flex row of buttons with merged border radius (first gets left radius, last gets right)
- Propagates variant, size, compact, outline to children

---

### 2.3 — Checkbox

**File:** `mozui-elements/src/checkbox.rs`

**API:**
```rust
pub struct Checkbox { .. }

pub fn checkbox() -> Checkbox;
```

**Builder:** `.label(text)`, `.checked(bool)`, `.on_click(handler)`, `.disabled(bool)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Renders a small bordered square with check icon when checked
- Label text to the right
- Theme colors: primary for checked, border for unchecked
- Check icon uses the Icon element
- Focusable — registers with InteractionMap for keyboard (Space to toggle)

**Dependencies:** Icon, Label

---

### 2.4 — Radio / RadioGroup

**File:** `mozui-elements/src/radio.rs`

**API:**
```rust
pub struct Radio { .. }
pub struct RadioGroup { .. }

pub fn radio(label: impl Into<String>) -> Radio;
pub fn radio_group() -> RadioGroup;
```

**Radio builder:** `.checked(bool)`, `.on_click(handler)`, `.disabled(bool)`, `.with_size(ComponentSize)`

**RadioGroup builder:** `.child(Radio)`, `.children(Vec<Radio>)`, `.selected_index(usize)`, `.on_click(handler)`, `.vertical()`, `.horizontal()`, `.disabled(bool)`

**Implementation notes:**
- Radio renders a circle with filled inner circle when selected
- RadioGroup manages mutual exclusion — clicking one deselects others
- on_click callback receives the selected index

**Dependencies:** Label

---

### 2.5 — Switch

**File:** `mozui-elements/src/switch.rs`

**API:**
```rust
pub struct Switch { .. }

pub fn switch() -> Switch;
```

**Builder:** `.checked(bool)`, `.label(text)`, `.label_side(Side)`, `.on_click(handler)`, `.disabled(bool)`, `.color(Color)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Track (rounded rect) + thumb (circle) that slides left/right
- Animate thumb position when toggled (use animation system)
- Theme: primary bg when on, muted bg when off
- Label positioned to left or right

**Dependencies:** Label

---

### 2.6 — Slider

**File:** `mozui-elements/src/slider.rs`

**API:**
```rust
pub enum SliderValue {
    Single(f32),
    Range(f32, f32),
}

pub struct SliderState {
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: SliderValue,
}

pub struct Slider { .. }

pub fn slider(state: SliderState) -> Slider;
```

**Builder:** `.on_change(handler)`, `.disabled(bool)`, `.vertical()`, `.horizontal()`, `.with_size(ComponentSize)`

**Implementation notes:**
- Track (thin rect) + thumb (circle) positioned by value
- Drag interaction: register click handler, track mouse position to compute value
- Range mode: two thumbs with filled region between
- Step snapping: round to nearest step value
- Controlled component pattern (like TextInput)

**Dependencies:** None (self-contained drawing)

---

### 2.7 — Select / Dropdown

**File:** `mozui-elements/src/select.rs`

**API:**
```rust
pub trait SelectItem {
    fn label(&self) -> &str;
    fn value(&self) -> &str;
}

pub struct Select<T: SelectItem> { .. }

pub fn select<T: SelectItem>(items: Vec<T>) -> Select<T>;
```

**Builder:** `.selected_index(Option<usize>)`, `.on_change(handler)`, `.placeholder(text)`, `.searchable(bool)`, `.disabled(bool)`, `.with_size(ComponentSize)`, `.menu_width(f32)`, `.menu_max_height(f32)`

**Implementation notes:**
- Trigger: styled div showing selected item or placeholder + chevron icon
- Dropdown: popover with list of items (uses Popover positioning)
- Search: TextInput at top of dropdown, filters items
- Keyboard: ArrowUp/Down to navigate, Enter to select, Escape to close
- Click outside closes dropdown

**Dependencies:** Popover, TextInput (for search), Icon, Label, VirtualList (for large item sets)

---

### 2.8 — ColorPicker

**File:** `mozui-elements/src/color_picker.rs`

**Purpose:** Color selection with HSLA sliders, hex input, and palette grid.

**API:**
```rust
pub struct ColorPicker { .. }

pub fn color_picker() -> ColorPicker;
```

**Builder:** `.value(Color)`, `.on_change(handler)`, `.disabled(bool)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Trigger: small color swatch showing current color
- Popover content:
  - Hue slider (rainbow gradient track)
  - Saturation/Lightness sliders
  - Alpha slider
  - Hex input (TextInput)
  - Preset color palette grid
- This is one of the most complex components — build last in this phase

**Dependencies:** Popover, Slider (x4), TextInput, TabBar, Divider, Tooltip

---

### 2.9 — Stepper

**File:** `mozui-elements/src/stepper.rs`

**API:**
```rust
pub struct StepperItem { .. }
pub struct Stepper { .. }

pub fn stepper() -> Stepper;
```

**Builder:** `.item(StepperItem)`, `.items(Vec<StepperItem>)`, `.selected_index(usize)`, `.on_click(handler)`, `.vertical()`, `.disabled(bool)`, `.with_size(ComponentSize)`

**StepperItem builder:** `.label(text)`, `.description(text)`, `.icon(IconName)`

**Implementation notes:**
- Numbered circles connected by lines
- Active step highlighted, completed steps checked
- Horizontal or vertical layout
- Click to jump to step (if allowed)

**Dependencies:** Icon, Label

---

## Phase 3: Data Display

### 3.1 — Badge

**File:** `mozui-elements/src/badge.rs`

**API:**
```rust
pub struct Badge { .. }

pub fn badge() -> Badge;
```

**Builder:** `.count(u32)`, `.dot()`, `.icon(IconName)`, `.max(u32)`, `.color(Color)`, `.child(element)`

**Implementation notes:**
- Absolute-positioned indicator on top-right of child element
- Dot mode: small colored circle
- Count mode: rounded pill with number (shows "99+" when exceeding max)
- Icon mode: small icon badge

**Dependencies:** Icon

---

### 3.2 — Tag

**File:** `mozui-elements/src/tag.rs`

**API:**
```rust
pub enum TagVariant {
    Primary, Secondary, Danger, Success, Warning, Info,
    Color(ColorName),
    Custom { bg: Color, fg: Color, border: Color },
}

pub struct Tag { .. }

pub fn tag(label: impl Into<String>) -> Tag;
```

**Builder:** `.with_variant(TagVariant)`, `.primary()`, `.danger()`, `.success()`, etc., `.outline()`, `.rounded_full()`, `.with_size(ComponentSize)`

**Implementation notes:**
- Small colored pill/chip
- Outline mode uses border instead of fill
- Colors derived from theme per variant

**Dependencies:** Label

---

### 3.3 — Avatar / AvatarGroup

**File:** `mozui-elements/src/avatar.rs`

**API:**
```rust
pub struct Avatar { .. }
pub struct AvatarGroup { .. }

pub fn avatar() -> Avatar;
pub fn avatar_group() -> AvatarGroup;
```

**Avatar builder:** `.src(path)`, `.name(text)`, `.placeholder(IconName)`, `.with_size(ComponentSize)`

**AvatarGroup builder:** `.child(Avatar)`, `.children(Vec<Avatar>)`, `.limit(usize)`

**Implementation notes:**
- Circle clip with image, initials, or placeholder icon
- Initials: take first letter of first two words of name
- Deterministic color from name hash
- AvatarGroup: overlapping circles with negative margin, "+N" overflow

**Dependencies:** Icon, Text

---

### 3.4 — Tooltip

**File:** `mozui-elements/src/tooltip.rs`

**API:**
```rust
pub struct Tooltip { .. }

pub fn tooltip(text: impl Into<String>) -> Tooltip;
```

**Builder:** `.action(text)`, `.keybinding(combo)`, `.child(element)`, `.placement(Placement)`

**Implementation notes:**
- Hover trigger on child element
- Delayed show (200ms default)
- Popover positioned above/below/left/right of trigger
- Optional keyboard shortcut badge (uses Kbd)
- Dismiss on mouse leave

**Dependencies:** Popover, Kbd, Text

---

### 3.5 — Skeleton

**File:** `mozui-elements/src/skeleton.rs`

**API:**
```rust
pub struct Skeleton { .. }

pub fn skeleton() -> Skeleton;
```

**Builder:** `.w(f32)`, `.h(f32)`, `.rounded(f32)`, `.rounded_full()`

**Implementation notes:**
- Gray rectangle with shimmer/pulse animation
- Animation uses the timer system for periodic opacity cycling
- Theme: skeleton color from theme tokens

**Dependencies:** None

---

### 3.6 — Spinner

**File:** `mozui-elements/src/spinner.rs`

**API:**
```rust
pub struct Spinner { .. }

pub fn spinner() -> Spinner;
```

**Builder:** `.color(Color)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Rotating arc/circle animation
- Draw partial circle arc using multiple small rects or a series of draw commands
- Rotation updated via timer
- Size scales with ComponentSize

**Dependencies:** Timer system (mozui-executor)

---

### 3.7 — Progress / ProgressCircle

**File:** `mozui-elements/src/progress.rs`

**API:**
```rust
pub struct Progress { .. }
pub struct ProgressCircle { .. }

pub fn progress() -> Progress;
pub fn progress_circle() -> ProgressCircle;
```

**Progress builder:** `.value(f32)` (0-100), `.loading()`, `.color(Color)`, `.with_size(ComponentSize)`

**ProgressCircle builder:** `.value(f32)`, `.loading()`, `.color(Color)`, `.with_size(ComponentSize)`, `.child(element)`

**Implementation notes:**
- Progress: track + filled bar, width proportional to value
- Loading mode: indeterminate animation (bar slides back and forth)
- ProgressCircle: circular arc, child content renders inside (e.g., percentage text)

**Dependencies:** Timer system for animations

---

### 3.8 — Rating

**File:** `mozui-elements/src/rating.rs`

**API:**
```rust
pub struct Rating { .. }

pub fn rating() -> Rating;
```

**Builder:** `.value(f32)`, `.max(u32)`, `.on_click(handler)`, `.color(Color)`, `.disabled(bool)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Row of star icons (filled/unfilled based on value)
- Hover preview: temporarily show hovered rating
- Half-star support via fractional values
- Click sets new value

**Dependencies:** Icon (Star, StarFilled)

---

### 3.9 — Link

**File:** `mozui-elements/src/link.rs`

**API:**
```rust
pub struct Link { .. }

pub fn link(label: impl Into<String>) -> Link;
```

**Builder:** `.href(url)`, `.on_click(handler)`, `.disabled(bool)`, `.color(Color)`

**Implementation notes:**
- Renders underlined text in link color
- Click handler fires on_click or opens href
- Hover: color change + hand cursor
- Disabled: muted color, no interaction

**Dependencies:** Text

---

## Phase 4: Navigation

### 4.1 — Breadcrumb

**File:** `mozui-elements/src/breadcrumb.rs`

**API:**
```rust
pub struct BreadcrumbItem { .. }
pub struct Breadcrumb { .. }

pub fn breadcrumb() -> Breadcrumb;
pub fn breadcrumb_item(label: impl Into<String>) -> BreadcrumbItem;
```

**Breadcrumb builder:** `.child(BreadcrumbItem)`, `.children(Vec<BreadcrumbItem>)`

**BreadcrumbItem builder:** `.on_click(handler)`, `.disabled(bool)`, `.icon(IconName)`

**Implementation notes:**
- Horizontal flex row with separator chevron icons between items
- Last item is non-clickable (current page)
- Items are clickable links (hand cursor, hover color)

**Dependencies:** Icon (ChevronRight), Label

---

### 4.2 — Pagination

**File:** `mozui-elements/src/pagination.rs`

**API:**
```rust
pub struct Pagination { .. }

pub fn pagination() -> Pagination;
```

**Builder:** `.current_page(usize)`, `.total_pages(usize)`, `.on_click(handler)`, `.visible_pages(usize)`, `.compact(bool)`, `.disabled(bool)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Prev/Next buttons + numbered page buttons
- Ellipsis ("...") for collapsed ranges
- Compact mode: just prev/next with page indicator
- on_click receives clicked page number

**Dependencies:** Button, Icon (ChevronLeft, ChevronRight)

---

### 4.3 — Tab / TabBar

**File:** `mozui-elements/src/tab.rs`

**API:**
```rust
pub enum TabVariant {
    Default,    // underline indicator
    Outline,    // bordered
    Pill,       // rounded pill background
    Segmented,  // segmented control style
}

pub struct Tab { .. }
pub struct TabBar { .. }

pub fn tab(label: impl Into<String>) -> Tab;
pub fn tab_bar() -> TabBar;
```

**Tab builder:** `.icon(IconName)`, `.selected(bool)`, `.disabled(bool)`, `.on_click(handler)`, `.with_variant(TabVariant)`, `.with_size(ComponentSize)`

**TabBar builder:** `.child(Tab)`, `.children(Vec<Tab>)`, `.selected_index(usize)`, `.on_click(handler)`, `.with_variant(TabVariant)`, `.with_size(ComponentSize)`, `.prefix(element)`, `.suffix(element)`

**Implementation notes:**
- TabBar propagates variant and size to child Tabs
- Default variant: bottom border indicator on selected tab
- Pill variant: rounded background on selected tab
- Segmented variant: connected buttons with shared background
- on_click receives tab index

**Dependencies:** Icon, Label

---

### 4.4 — Tree

**File:** `mozui-elements/src/tree.rs`

**API:**
```rust
pub struct TreeItem { .. }
pub struct TreeState { .. }
pub struct Tree { .. }

pub fn tree_item(label: impl Into<String>) -> TreeItem;
pub fn tree(items: Vec<TreeItem>) -> Tree;
```

**TreeItem builder:** `.child(TreeItem)`, `.children(Vec<TreeItem>)`, `.icon(IconName)`, `.expanded(bool)`, `.disabled(bool)`, `.on_click(handler)`

**Tree builder:** `.on_select(handler)`, `.with_size(ComponentSize)`

**TreeState:** Tracks expanded/collapsed state per node, selected node.

**Implementation notes:**
- Indented list with expand/collapse chevrons
- Uses VirtualList for efficient rendering of large trees
- Flatten tree to visible items based on expanded state
- Keyboard: ArrowUp/Down to navigate, ArrowRight to expand, ArrowLeft to collapse

**Dependencies:** VirtualList, Icon (ChevronRight, ChevronDown), Label

---

## Phase 5: Layout & Structure

### 5.1 — Accordion

**File:** `mozui-elements/src/accordion.rs`

**API:**
```rust
pub struct AccordionItem { .. }
pub struct Accordion { .. }

pub fn accordion() -> Accordion;
pub fn accordion_item(title: impl Into<String>) -> AccordionItem;
```

**Accordion builder:** `.child(AccordionItem)`, `.children(Vec<AccordionItem>)`, `.multiple(bool)`, `.bordered(bool)`, `.on_toggle(handler)`, `.disabled(bool)`, `.with_size(ComponentSize)`

**AccordionItem builder:** `.title(text)`, `.icon(IconName)`, `.open(bool)`, `.disabled(bool)`, `.child(element)`

**Implementation notes:**
- Each item has a clickable header and collapsible content
- `multiple(false)`: opening one closes others (default)
- Animated expand/collapse (height transition)
- Chevron icon rotates when open

**Dependencies:** Icon (ChevronDown), Label

---

### 5.2 — Collapsible

**File:** `mozui-elements/src/collapsible.rs`

**API:**
```rust
pub struct CollapsibleContainer { .. }

pub fn collapsible() -> CollapsibleContainer;
```

**Builder:** `.open(bool)`, `.child(element)`, `.content(element_fn)`

**Implementation notes:**
- Generic container that shows/hides content based on `open` state
- Trigger child always visible, content conditionally rendered
- Simpler than Accordion (no header styling, just show/hide)

**Dependencies:** None

---

### 5.3 — DescriptionList

**File:** `mozui-elements/src/description_list.rs`

**API:**
```rust
pub struct DescriptionItem { .. }
pub struct DescriptionList { .. }

pub fn description_list() -> DescriptionList;
pub fn description_item(label: impl Into<String>) -> DescriptionItem;
```

**DescriptionList builder:** `.child(DescriptionItem)`, `.children(Vec<DescriptionItem>)`, `.vertical()`, `.horizontal()`, `.bordered(bool)`, `.label_width(f32)`, `.columns(usize)`, `.with_size(ComponentSize)`

**DescriptionItem builder:** `.value(text)`, `.value_element(element)`, `.span(usize)`

**Implementation notes:**
- Key-value pair display (like HTML `<dl>`)
- Horizontal: label left, value right (side by side)
- Vertical: label above value (stacked)
- Bordered mode adds dividers between items
- Multi-column layout supported

**Dependencies:** Label, Divider

---

### 5.4 — GroupBox

**File:** `mozui-elements/src/group_box.rs`

**API:**
```rust
pub struct GroupBox { .. }

pub fn group_box() -> GroupBox;
```

**Builder:** `.title(text)`, `.child(element)`, `.children(elements)`, `.fill()`, `.outline()`

**Implementation notes:**
- Bordered container with title text positioned on the top border
- Fill variant: colored background in content area
- Outline variant: just border, transparent background

**Dependencies:** Label

---

### 5.5 — Form / Field

**File:** `mozui-elements/src/form.rs`

**API:**
```rust
pub struct Field { .. }
pub struct Form { .. }

pub fn form() -> Form;
pub fn field() -> Field;
```

**Form builder:** `.vertical()`, `.horizontal()`, `.label_width(f32)`, `.child(Field)`, `.children(Vec<Field>)`, `.columns(usize)`, `.with_size(ComponentSize)`

**Field builder:** `.label(text)`, `.required(bool)`, `.description(text)`, `.child(element)`, `.col_span(usize)`, `.visible(bool)`

**Implementation notes:**
- Form lays out fields in grid pattern
- Horizontal: label left, input right
- Vertical: label above input
- Required indicator (asterisk)
- Description text below input

**Dependencies:** Label, Text

---

### 5.6 — Resizable Panels

**File:** `mozui-elements/src/resizable.rs`

**API:**
```rust
pub struct ResizablePanel { .. }
pub struct ResizablePanelGroup { .. }

pub fn resizable_panel() -> ResizablePanel;
pub fn h_resizable() -> ResizablePanelGroup;
pub fn v_resizable() -> ResizablePanelGroup;
```

**ResizablePanel builder:** `.child(element)`, `.min_size(f32)`, `.max_size(f32)`, `.default_size(f32)`

**ResizablePanelGroup builder:** `.child(ResizablePanel)`, `.on_resize(handler)`

**Implementation notes:**
- Drag handles between panels
- Mouse drag adjusts relative sizes
- Min/max size constraints
- Horizontal or vertical layout
- Cursor changes to resize cursor on handle hover

**Dependencies:** Divider (for handles)

---

### 5.7 — Sidebar

**File:** `mozui-elements/src/sidebar.rs`

**API:**
```rust
pub struct Sidebar { .. }
pub struct SidebarItem { .. }
pub struct SidebarGroup { .. }

pub fn sidebar() -> Sidebar;
pub fn sidebar_item(label: impl Into<String>) -> SidebarItem;
pub fn sidebar_group(title: impl Into<String>) -> SidebarGroup;
```

**Sidebar builder:** `.side(Side)`, `.collapsed(bool)`, `.header(element)`, `.footer(element)`, `.child(SidebarGroup)`, `.width(f32)`, `.collapsed_width(f32)`

**SidebarItem builder:** `.icon(IconName)`, `.selected(bool)`, `.on_click(handler)`, `.badge(text)`

**SidebarGroup builder:** `.child(SidebarItem)`, `.children(Vec<SidebarItem>)`, `.collapsible(bool)`

**Implementation notes:**
- Fixed-width panel on left or right side
- Collapse to icon-only mode
- Groups with optional titles
- Items with icon, label, optional badge
- Selected item highlight

**Dependencies:** Icon, Label, Badge, Tooltip (for collapsed icon tooltips)

**Constants:** DEFAULT_WIDTH = 255.0, COLLAPSED_WIDTH = 48.0

---

## Phase 6: Overlays & Popups

### 6.1 — PopupMenu / ContextMenu / DropdownMenu

**File:** `mozui-elements/src/menu.rs`

**API:**
```rust
pub enum MenuItem {
    Item { label: String, icon: Option<IconName>, action: Option<String>, disabled: bool, on_click: Option<ClickHandler> },
    Separator,
    Label(String),
    Submenu { label: String, icon: Option<IconName>, items: Vec<MenuItem> },
}

pub struct PopupMenu { .. }
pub struct ContextMenu { .. }
pub struct DropdownMenu { .. }

pub fn popup_menu() -> PopupMenu;
pub fn menu_item(label: impl Into<String>) -> MenuItem;
pub fn menu_separator() -> MenuItem;
```

**PopupMenu builder:** `.item(MenuItem)`, `.items(Vec<MenuItem>)`, `.anchor(Anchor)`, `.on_select(handler)`

**Implementation notes:**
- Popover-positioned menu with items
- Keyboard navigation: ArrowUp/Down, Enter to select, Escape to close
- Submenu opens on hover/ArrowRight
- Separator items render as dividers
- Items show icon, label, and optional keyboard shortcut (Kbd)
- ContextMenu: triggered by right-click on a target element
- DropdownMenu: triggered by clicking a button/trigger

**Dependencies:** Popover, Icon, Label, Kbd, Divider, focus trapping

---

### 6.2 — Dialog / AlertDialog

**File:** `mozui-elements/src/dialog.rs`

**API:**
```rust
pub struct Dialog { .. }
pub struct AlertDialog { .. }

pub fn dialog() -> Dialog;
pub fn alert_dialog() -> AlertDialog;
```

**Dialog builder:** `.title(text)`, `.content(element_fn)`, `.footer(element_fn)`, `.width(f32)`, `.on_close(handler)`, `.on_ok(handler)`, `.on_cancel(handler)`, `.overlay(bool)`, `.overlay_closable(bool)`, `.close_button(bool)`

**AlertDialog builder:** `.title(text)`, `.description(text)`, `.icon(IconName)`, `.on_ok(handler)`, `.on_cancel(handler)`, `.ok_text(text)`, `.cancel_text(text)`

**Implementation notes:**
- Modal overlay with centered content panel
- Focus trap: Tab cycles within dialog only
- Escape closes dialog
- Overlay dims background (uses theme.overlay color)
- AlertDialog is an opinionated wrapper with icon/title/description/buttons
- Open/close managed through Root element's dialog layer system

**Dependencies:** Root (layer management), Button, Icon, Label, focus trapping

---

### 6.3 — Sheet

**File:** `mozui-elements/src/sheet.rs`

**API:**
```rust
pub struct Sheet { .. }

pub fn sheet() -> Sheet;
```

**Builder:** `.placement(Placement)`, `.title(text)`, `.content(element_fn)`, `.footer(element_fn)`, `.size(f32)`, `.resizable(bool)`, `.overlay(bool)`, `.overlay_closable(bool)`, `.on_close(handler)`

**Implementation notes:**
- Slide-in panel from any edge (top/right/bottom/left)
- Overlay behind the sheet
- Close on Escape or overlay click (if closable)
- Optional resize handle on the open edge
- Open/close managed through Root element's sheet layer system

**Dependencies:** Root (layer management), Button (close), Icon, Resizable (for resize handle)

---

### 6.4 — Notification / Toast

**File:** `mozui-elements/src/notification.rs`

**API:**
```rust
pub enum NotificationType {
    Info, Success, Warning, Error,
}

pub struct NotificationItem { .. }

pub fn notification(message: impl Into<String>) -> NotificationItem;
```

**Builder:** `.title(text)`, `.icon(IconName)`, `.with_type(NotificationType)`, `.info()`, `.success()`, `.warning()`, `.error()`, `.autohide(Duration)`, `.action(label, handler)`, `.on_click(handler)`

**Implementation notes:**
- Toast notifications positioned in a corner (configurable via theme)
- Stack of notifications with stagger
- Auto-dismiss after duration (uses timer system)
- Action button for user interaction
- Icon + title + message layout
- Managed through Root element's notification layer

**Dependencies:** Root (layer management), Icon, Label, Button, Timer system

---

### 6.5 — HoverCard

**File:** `mozui-elements/src/hover_card.rs`

**API:**
```rust
pub struct HoverCard { .. }

pub fn hover_card() -> HoverCard;
```

**Builder:** `.trigger(element)`, `.content(element_fn)`, `.anchor(Anchor)`, `.open_delay(Duration)`, `.close_delay(Duration)`

**Implementation notes:**
- Hover over trigger shows content popover after delay
- Content stays while hovering over content itself
- Close after mouse leaves both trigger and content (with delay)
- Uses timer system for delays

**Dependencies:** Popover, Timer system

---

## Phase 7: Lists & Tables

### 7.1 — List / ListItem

**File:** `mozui-elements/src/list.rs`

**API:**
```rust
pub struct ListItem { .. }
pub struct List { .. }

pub fn list() -> List;
pub fn list_item(label: impl Into<String>) -> ListItem;
```

**List builder:** `.child(ListItem)`, `.children(Vec<ListItem>)`, `.on_select(handler)`, `.selected_index(Option<usize>)`, `.with_size(ComponentSize)`, `.virtualized(bool)`

**ListItem builder:** `.icon(IconName)`, `.suffix(element)`, `.description(text)`, `.on_click(handler)`, `.disabled(bool)`, `.selected(bool)`, `.separator(bool)`

**Implementation notes:**
- Simple list of items with icon, label, optional description
- Selected item highlight
- Keyboard navigation: ArrowUp/Down, Enter to select
- Virtualized mode: uses VirtualList for large datasets
- Separator items render as dividers

**Dependencies:** VirtualList, Icon, Label, Divider

---

### 7.2 — Table / DataTable

**File:** `mozui-elements/src/table.rs`

**API:**
```rust
pub struct Column { .. }
pub struct Table { .. }

pub fn column(label: impl Into<String>) -> Column;
pub fn table() -> Table;
```

**Column builder:** `.width(f32)`, `.min_width(f32)`, `.max_width(f32)`, `.sortable(bool)`, `.render(fn)`, `.align_right()`

**Table builder:** `.columns(Vec<Column>)`, `.rows(usize)`, `.render_row(fn)`, `.on_row_click(handler)`, `.selected_row(Option<usize>)`, `.striped(bool)`, `.bordered(bool)`, `.with_size(ComponentSize)`, `.sortable(bool)`, `.on_sort(handler)`, `.loading(bool)`

**Implementation notes:**
- Header row with column labels and optional sort indicators
- Body with data rows (virtualized for large datasets)
- Striped mode: alternating row colors (table_even)
- Sort: click column header to toggle sort direction, icon indicator
- Selected row highlight
- Loading state: skeleton rows
- Theme: extensive use of table_* tokens

**Dependencies:** VirtualList, Icon (sort arrows), Label, Skeleton, Divider

---

## Phase 8: Scrolling & Advanced

### 8.1 — Scrollbar / ScrollContainer

**File:** `mozui-elements/src/scroll.rs`

**API:**
```rust
pub enum ScrollbarShow {
    Scrolling,  // show only while scrolling
    Hover,      // show on container hover
    Always,     // always visible
}

pub struct ScrollContainer { .. }

pub fn scroll_container() -> ScrollContainer;
```

**Builder:** `.child(element)`, `.show(ScrollbarShow)`, `.vertical(bool)`, `.horizontal(bool)`, `.thumb_color(Color)`

**Implementation notes:**
- Wrapper element that clips content and shows scrollbar overlay
- Track scroll offset via signals
- Scrollbar thumb: proportional size to content/viewport ratio
- Drag thumb to scroll
- Scroll wheel events update offset
- Auto-hide scrollbar after scroll stops (timer-based fade)
- Edge masks: subtle gradient at scroll edges to indicate more content

**Dependencies:** Timer system (for fade), Signal system

---

### 8.2 — Clipboard Button

**File:** `mozui-elements/src/clipboard_button.rs`

**API:**
```rust
pub struct ClipboardButton { .. }

pub fn clipboard_button(value: impl Into<String>) -> ClipboardButton;
```

**Builder:** `.on_copied(handler)`, `.with_size(ComponentSize)`

**Implementation notes:**
- Button that copies text to clipboard on click
- Shows copy icon, switches to check icon for 2 seconds after copy
- Uses platform clipboard integration

**Dependencies:** Button, Icon (Copy, Check), Timer system, Clipboard (platform)

---

## Implementation Order

Recommended build sequence, respecting dependencies:

```
Phase 1 (primitives):     Icon -> Label -> Divider -> Kbd
Phase 2 (form controls):  Button -> Checkbox -> Radio -> Switch -> Slider -> Select -> Stepper
Phase 3 (data display):   Badge -> Tag -> Avatar -> Tooltip -> Skeleton -> Spinner -> Progress -> Rating -> Link
Phase 4 (navigation):     Breadcrumb -> Pagination -> Tab/TabBar -> Tree
Phase 5 (layout):         Accordion -> Collapsible -> DescriptionList -> GroupBox -> Form -> Resizable -> Sidebar
Phase 6 (overlays):       PopupMenu -> Dialog -> Sheet -> Notification -> HoverCard
Phase 7 (lists/tables):   List -> Table
Phase 8 (scroll/misc):    ScrollContainer -> ClipboardButton -> ColorPicker
```

**Total components: 45+**

**Estimated scope:**
- Phase 1: ~4 files, ~600 lines
- Phase 2: ~7 files, ~2500 lines
- Phase 3: ~9 files, ~1500 lines
- Phase 4: ~4 files, ~1200 lines
- Phase 5: ~7 files, ~2000 lines
- Phase 6: ~5 files, ~2000 lines
- Phase 7: ~2 files, ~1500 lines
- Phase 8: ~3 files, ~800 lines

---

## Key Patterns to Follow

1. **Builder pattern everywhere.** Every component uses `fn method(mut self, ...) -> Self`.

2. **Controlled components.** State lives in signals managed by the user. Components take current state + on_change callbacks (like TextInput today).

3. **Theme-driven.** Every component reads colors, spacing, radii, and font sizes from the active `Theme`. No hardcoded colors.

4. **Size-aware.** Components implement `Sizable` and use `ComponentSize` to determine heights, paddings, font sizes, and icon sizes.

5. **Variant enums.** Components with visual variants (Button, Tag, Alert, Tab) use an enum + convenience methods (`.primary()`, `.ghost()`, etc.).

6. **Composability.** Complex components compose simpler ones: Dialog uses Button + Icon + focus trapping. Select uses Popover + TextInput + VirtualList.

7. **Keyboard accessible.** Interactive components register as focusable and handle keyboard events (Space/Enter to activate, Escape to dismiss, Arrow keys to navigate).

8. **Event callbacks.** All callbacks use `Box<dyn Fn(&mut dyn Any)>` pattern for Context downcast, consistent with existing mozui elements.
