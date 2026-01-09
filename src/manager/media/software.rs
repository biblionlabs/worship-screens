// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

use std::sync::{Arc, atomic::AtomicBool};

use futures::channel::mpsc::UnboundedSender;
use gst::prelude::*;
use gst_app;
use gst_video::video_frame::VideoFrameExt;
use slint::ComponentHandle;
use ui::{ViewState, ViewWindow};

pub fn init(
    app: &ViewWindow,
    pipeline: &gst::Pipeline,
    bus_sender: UnboundedSender<gst::Message>,
    output_enabled: Arc<AtomicBool>,
) -> gst::Element {
    pipeline.bus().unwrap().set_sync_handler(move |_, message| {
        let _ = bus_sender.unbounded_send(message.to_owned());
        gst::BusSyncReply::Drop
    });

    // Create an appsink that produces RGB frames (software path).
    let appsink = gst_app::AppSink::builder()
        .caps(
            &gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgb)
                .build(),
        )
        .enable_last_sample(false)
        .max_buffers(1u32)
        .build();

    let _ = pipeline.set_property("video-sink", &appsink);

    let app_weak = app.as_weak();
    let output_enabled_clone = output_enabled.clone();

    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample = match appsink.pull_sample() {
                    Ok(s) => s,
                    Err(_) => return Err(gst::FlowError::Eos),
                };

                let buffer = sample.buffer_owned().ok_or(gst::FlowError::Error)?;
                let caps = sample.caps().ok_or(gst::FlowError::NotNegotiated)?;
                let video_info = gst_video::VideoInfo::from_caps(&caps)
                    .map_err(|_| gst::FlowError::NotNegotiated)?;
                let video_frame = gst_video::VideoFrame::from_buffer_readable(buffer, &video_info)
                    .map_err(|_| gst::FlowError::Error)?;

                let slint_frame = try_gstreamer_video_frame_to_pixel_buffer(&video_frame);

                if output_enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    let app_weak = app_weak.clone();
                    app_weak
                        .upgrade_in_event_loop(move |app| {
                            let state = app.global::<ViewState>();
                            let mut shared = state.get_shared_view();
                            shared.img_bg = slint::Image::from_rgb8(slint_frame);
                            shared.show_img = true;
                            state.set_shared_view(shared);
                        })
                        .ok();
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    appsink.into()
}

fn try_gstreamer_video_frame_to_pixel_buffer(
    frame: &gst_video::VideoFrame<gst_video::video_frame::Readable>,
) -> slint::SharedPixelBuffer<slint::Rgb8Pixel> {
    match frame.format() {
        gst_video::VideoFormat::Rgb => {
            let mut slint_pixel_buffer =
                slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(frame.width(), frame.height());
            frame
                .buffer()
                .copy_to_slice(0, slint_pixel_buffer.make_mut_bytes())
                .expect("Unable to copy to slice!"); // Copies!
            slint_pixel_buffer
        }
        other => panic!(
            "Cannot convert frame to a slint RGB frame because it is format {}",
            other.to_str()
        ),
    }
}
