// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use futures::channel::mpsc::UnboundedSender;
use ui::{MainWindow, ViewWindow};

pub fn init(
    main: &MainWindow,
    view: &ViewWindow,
    pipeline: &gst::Pipeline,
    bus_sender: UnboundedSender<gst::Message>,
) -> gst::Element {
    #[cfg(not(target_os = "linux"))]
    return super::egl::init(main, view, pipeline, bus_sender);
    #[cfg(target_os = "linux")]
    return super::egl::init(main, view, pipeline, bus_sender);
}
