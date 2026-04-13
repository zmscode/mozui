use crate::window::{FrameCallback, NativeCallbackDispatcher};
use crate::{App, Window};
use std::rc::Rc;

pub(super) fn schedule_native_callback_no_args<Event: 'static>(
    handler: Rc<dyn Fn(&Event, &mut Window, &mut App)>,
    make_event: impl Fn() -> Event + 'static,
    dispatcher: NativeCallbackDispatcher,
) -> Box<dyn Fn()> {
    Box::new(move || {
        let handler = handler.clone();
        let event = make_event();
        let callback: FrameCallback = Box::new(move |window, cx| {
            handler(&event, window, cx);
        });
        dispatcher.dispatch(callback);
    })
}

pub(super) fn schedule_native_callback<P: 'static, Event: 'static>(
    handler: Rc<dyn Fn(&Event, &mut Window, &mut App)>,
    make_event: impl Fn(P) -> Event + 'static,
    dispatcher: NativeCallbackDispatcher,
) -> Box<dyn Fn(P)> {
    Box::new(move |param| {
        let handler = handler.clone();
        let event = make_event(param);
        let callback: FrameCallback = Box::new(move |window, cx| {
            handler(&event, window, cx);
        });
        dispatcher.dispatch(callback);
    })
}
