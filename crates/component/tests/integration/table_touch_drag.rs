//! A finger resizes and moves a data table's columns, also when the header
//! scrolls sideways.

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Pixels, Render,
    Styled as _, TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div,
    point, px,
};
use gpui_component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};

const COLUMNS: usize = 30;
const WIDTH: f32 = 120.;

struct Delegate;

impl TableDelegate for Delegate {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS
    }

    fn rows_count(&self, _: &App) -> usize {
        20
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        Column::new(format!("c{col_ix}"), format!("Column {col_ix}")).width(WIDTH)
    }

    fn render_td(
        &mut self,
        _: usize,
        _: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().child("cell")
    }
}

struct Screen {
    table: Entity<TableState<Delegate>>,
}

impl Render for Screen {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}

struct Harness {
    table: Entity<TableState<Delegate>>,
    events: Rc<RefCell<Vec<TableEvent>>>,
    cx: VisualTestContext,
}

impl Harness {
    /// A table thirty columns wide, wider than the test window, so
    /// its header scrolls sideways.
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_component::init);
        let window = cx.add_window(|window, cx| Screen {
            table: cx.new(|cx| TableState::new(Delegate, window, cx)),
        });
        let table = window
            .update(cx, |screen, _, _| screen.table.clone())
            .unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        cx.update(|cx| {
            cx.subscribe(&table, move |_, event: &TableEvent, _| {
                log.borrow_mut().push(event.clone())
            })
            .detach()
        });
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        Self { table, events, cx }
    }

    fn touch(&mut self, phase: TouchPhase, x: f32, y: f32) {
        self.cx.simulate_event(TouchEvent {
            id: TouchId(1),
            phase,
            position: point(px(x), px(y)),
            ..Default::default()
        });
        self.cx.run_until_parked();
    }

    fn col_width(&mut self, col_ix: usize) -> Pixels {
        self.cx
            .update(|_, cx| self.table.read(cx).col_width(col_ix).unwrap())
    }
}

/// In the header row, away from the table's top edge.
const HEADER_Y: f32 = 12.;

#[gpui::test]
fn a_finger_on_a_resize_handle_resizes_the_column(cx: &mut TestAppContext) {
    let mut t = Harness::new(cx);
    let edge = f32::from(t.col_width(0));

    t.touch(TouchPhase::Started, edge - 1., HEADER_Y);
    for step in 1..=6 {
        t.touch(TouchPhase::Moved, edge - 1. + 10. * step as f32, HEADER_Y);
    }
    t.touch(TouchPhase::Ended, edge + 59., HEADER_Y);

    assert_eq!(t.col_width(0), px(WIDTH + 60.));
    assert!(
        t.events
            .borrow()
            .iter()
            .any(|event| matches!(event, TableEvent::ColumnWidthsChanged(_))),
        "no width change reported"
    );
}

#[gpui::test]
fn a_long_press_on_a_header_cell_picks_the_column_up_to_move_it(cx: &mut TestAppContext) {
    let mut t = Harness::new(cx);
    let middle_of_second = WIDTH * 1.5;

    t.touch(TouchPhase::Started, middle_of_second, HEADER_Y);
    t.cx.executor().advance_clock(Duration::from_secs(1));
    t.cx.run_until_parked();
    for x in [middle_of_second - 40., 60., 20., 10.] {
        t.touch(TouchPhase::Moved, x, HEADER_Y);
    }
    t.touch(TouchPhase::Ended, 10., HEADER_Y);

    assert!(
        t.events
            .borrow()
            .iter()
            .any(|event| matches!(event, TableEvent::MoveColumn(1, 0))),
        "column 1 did not move to 0"
    );
}
