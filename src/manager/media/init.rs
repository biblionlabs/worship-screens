// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use futures::channel::mpsc::UnboundedSender;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use ui::ViewWindow;

pub fn init(
    view: &ViewWindow,
    pipeline: &gst::Pipeline,
    bus_sender: UnboundedSender<gst::Message>,
    output_enabled: Arc<AtomicBool>,
) -> gst::Element {
    #[cfg(not(target_os = "linux"))]
    return super::egl::init(view, pipeline, bus_sender, output_enabled);
    #[cfg(target_os = "linux")]
    return super::egl::init(view, pipeline, bus_sender, output_enabled);
}
