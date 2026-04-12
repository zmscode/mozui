use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::os::raw::c_void;
use std::sync::Once;

/// Column definition for the table.
#[derive(Clone)]
pub struct TableColumn {
    pub identifier: String,
    pub title: String,
    pub width: f64,
    pub min_width: f64,
    pub max_width: f64,
}

impl TableColumn {
    pub fn new(identifier: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            title: title.into(),
            width: 100.0,
            min_width: 40.0,
            max_width: 1000.0,
        }
    }

    pub fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    pub fn min_width(mut self, min: f64) -> Self {
        self.min_width = min;
        self
    }

    pub fn max_width(mut self, max: f64) -> Self {
        self.max_width = max;
        self
    }
}

/// A row of string values, one per column.
pub type TableRow = Vec<String>;

/// Configuration for a native table view.
pub struct TableConfig {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
    pub on_select: Option<Box<dyn Fn(usize) + 'static>>,
    pub allows_multiple_selection: bool,
    pub alternating_rows: bool,
}

impl TableConfig {
    pub fn new(columns: Vec<TableColumn>, rows: Vec<TableRow>) -> Self {
        Self {
            columns,
            rows,
            on_select: None,
            allows_multiple_selection: false,
            alternating_rows: true,
        }
    }

    pub fn on_select(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    pub fn allows_multiple_selection(mut self, allows: bool) -> Self {
        self.allows_multiple_selection = allows;
        self
    }

    pub fn alternating_rows(mut self, alternating: bool) -> Self {
        self.alternating_rows = alternating;
        self
    }
}

/// Install a native `NSTableView` as a subview of the window's content view.
///
/// Returns the raw `NSScrollView` (containing the table) for further manipulation.
pub fn install_table_view(window: &Window, config: TableConfig) -> id {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        // Create NSTableView
        let table: id = msg_send![class!(NSTableView), alloc];
        let table: id = msg_send![table, init];

        let _: () =
            msg_send![table, setUsesAlternatingRowBackgroundColors: config.alternating_rows];
        let _: () = msg_send![table, setAllowsMultipleSelection: config.allows_multiple_selection];
        // NSTableViewStyleAutomatic = 0
        let _: () = msg_send![table, setStyle: 0_isize];

        // Add columns
        for col_def in &config.columns {
            let ns_id = CocoaNSString::alloc(nil).init_str(&col_def.identifier);
            let column: id = msg_send![class!(NSTableColumn), alloc];
            let column: id = msg_send![column, initWithIdentifier: ns_id];

            let ns_title = CocoaNSString::alloc(nil).init_str(&col_def.title);
            let header: id = msg_send![column, headerCell];
            let _: () = msg_send![header, setStringValue: ns_title];

            let _: () = msg_send![column, setWidth: col_def.width];
            let _: () = msg_send![column, setMinWidth: col_def.min_width];
            let _: () = msg_send![column, setMaxWidth: col_def.max_width];

            let _: () = msg_send![table, addTableColumn: column];
        }

        // Set up data source and delegate
        let delegate = create_table_delegate(config.columns, config.rows, config.on_select);
        let _: () = msg_send![table, setDataSource: delegate];
        let _: () = msg_send![table, setDelegate: delegate];

        // Wrap in scroll view
        let scroll: id = msg_send![class!(NSScrollView), alloc];
        let scroll: id = msg_send![scroll, init];
        let _: () = msg_send![scroll, setDocumentView: table];
        let _: () = msg_send![scroll, setHasVerticalScroller: true];
        let _: () = msg_send![scroll, setHasHorizontalScroller: false];
        let _: () = msg_send![scroll, setTranslatesAutoresizingMaskIntoConstraints: false];

        let parent: id = msg_send![ns_view, superview];
        let _: () = msg_send![parent, addSubview: scroll];

        let _: () = msg_send![table, reloadData];

        scroll
    }
}

// --- Table data source / delegate ---

static REGISTER_TABLE_DELEGATE: Once = Once::new();
static mut TABLE_DELEGATE_CLASS: *const Class = std::ptr::null();

const TABLE_COLUMNS_IVAR: &str = "_tableColumns";
const TABLE_ROWS_IVAR: &str = "_tableRows";
const TABLE_ON_SELECT_IVAR: &str = "_tableOnSelect";

fn create_table_delegate(
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    on_select: Option<Box<dyn Fn(usize) + 'static>>,
) -> id {
    unsafe {
        REGISTER_TABLE_DELEGATE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiTableDelegate", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(TABLE_COLUMNS_IVAR);
            decl.add_ivar::<*mut c_void>(TABLE_ROWS_IVAR);
            decl.add_ivar::<*mut c_void>(TABLE_ON_SELECT_IVAR);

            // numberOfRowsInTableView:
            extern "C" fn number_of_rows(this: &Object, _sel: Sel, _table: id) -> isize {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(TABLE_ROWS_IVAR);
                    let rows = &*(ptr as *const Vec<TableRow>);
                    rows.len() as isize
                }
            }

            // tableView:objectValueForTableColumn:row:
            extern "C" fn object_value(
                this: &Object,
                _sel: Sel,
                _table: id,
                column: id,
                row: isize,
            ) -> id {
                unsafe {
                    let cols_ptr: *mut c_void = *this.get_ivar(TABLE_COLUMNS_IVAR);
                    let cols = &*(cols_ptr as *const Vec<TableColumn>);
                    let rows_ptr: *mut c_void = *this.get_ivar(TABLE_ROWS_IVAR);
                    let rows = &*(rows_ptr as *const Vec<TableRow>);

                    let col_id: id = msg_send![column, identifier];
                    let utf8: *const i8 = msg_send![col_id, UTF8String];
                    let col_str = std::ffi::CStr::from_ptr(utf8).to_str().unwrap_or("");

                    let col_idx = cols
                        .iter()
                        .position(|c| c.identifier == col_str)
                        .unwrap_or(0);
                    let row_idx = row as usize;

                    if row_idx < rows.len() && col_idx < rows[row_idx].len() {
                        CocoaNSString::alloc(nil).init_str(&rows[row_idx][col_idx])
                    } else {
                        CocoaNSString::alloc(nil).init_str("")
                    }
                }
            }

            // tableViewSelectionDidChange:
            extern "C" fn selection_changed(this: &Object, _sel: Sel, notification: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(TABLE_ON_SELECT_IVAR);
                    if !ptr.is_null() {
                        let callback = &*(ptr as *const Box<dyn Fn(usize)>);
                        let table: id = msg_send![notification, object];
                        let row: isize = msg_send![table, selectedRow];
                        if row >= 0 {
                            callback(row as usize);
                        }
                    }
                }
            }

            decl.add_method(
                sel!(numberOfRowsInTableView:),
                number_of_rows as extern "C" fn(&Object, Sel, id) -> isize,
            );
            decl.add_method(
                sel!(tableView:objectValueForTableColumn:row:),
                object_value as extern "C" fn(&Object, Sel, id, id, isize) -> id,
            );
            decl.add_method(
                sel!(tableViewSelectionDidChange:),
                selection_changed as extern "C" fn(&Object, Sel, id),
            );

            TABLE_DELEGATE_CLASS = decl.register();
        });

        let cls = TABLE_DELEGATE_CLASS;
        let delegate: id = msg_send![cls, alloc];
        let delegate: id = msg_send![delegate, init];

        let cols_box = Box::new(columns);
        (*delegate).set_ivar(TABLE_COLUMNS_IVAR, Box::into_raw(cols_box) as *mut c_void);

        let rows_box = Box::new(rows);
        (*delegate).set_ivar(TABLE_ROWS_IVAR, Box::into_raw(rows_box) as *mut c_void);

        if let Some(cb) = on_select {
            let cb_box = Box::new(cb);
            (*delegate).set_ivar(TABLE_ON_SELECT_IVAR, Box::into_raw(cb_box) as *mut c_void);
        } else {
            (*delegate).set_ivar(TABLE_ON_SELECT_IVAR, std::ptr::null_mut::<c_void>());
        }

        delegate
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
