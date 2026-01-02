// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use futures::channel::mpsc::UnboundedSender;
use ui::{MainWindow, ViewWindow};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn init(
    main: &MainWindow,
    view: &ViewWindow,
    pipeline: &gst::Pipeline,
    bus_sender: UnboundedSender<gst::Message>,
    preview_enabled: Arc<AtomicBool>,
    output_enabled: Arc<AtomicBool>,
) -> gst::Element {
    #[cfg(not(target_os = "linux"))]
    return super::egl::init(main, view, pipeline, bus_sender, preview_enabled, output_enabled);
    #[cfg(target_os = "linux")]
    return super::egl::init(main, view, pipeline, bus_sender, preview_enabled, output_enabled);
}
