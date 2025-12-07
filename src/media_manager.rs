use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use mp4::Mp4Reader;
use openh264::OpenH264API;
use openh264::decoder::{Decoder, DecoderConfig, Flush};
use rfd::FileDialog;
use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{
    Color, ComponentHandle, Image, ModelRc, Rgb8Pixel, SharedPixelBuffer, SharedString,
    ToSharedString, Weak,
};
use tracing::error;
use ui::{MainWindow, ViewData, ViewFontData, ViewState, ViewWindow};

use crate::bitstream_converter::Mp4BitstreamConverter;
use crate::settings::SourceMedia;
use crate::user_data::UserData;

const IMAGE_FORMATS: &[&str] = &[
    "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpeg", "jpg", "png", "pnm", "qoi",
    "tga", "tiff", "tif", "webp",
];
const ALL_MEDIA_FORMATS: &[&str] = &[
    // Image formats
    "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpeg", "jpg", "png", "pnm", "qoi",
    "tga", "tiff", "tif", "webp", // Video formats
    "mov", "mp4", "m4a", "m4v", "m4b", "m4r", "m4p", "3gp", "3g2", "mj2", "qt",
];

pub struct MediaManager {
    data: Arc<UserData>,
    window: Weak<MainWindow>,
    view_window: Weak<ViewWindow>,
    media_list: Arc<Mutex<SourceMedia>>,

    preview_video_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    preview_video_playing: Arc<AtomicBool>,

    output_video_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    output_video_playing: Arc<AtomicBool>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaItem {
    tmp: bool,
    #[serde(default)]
    pub is_logo: bool,
    pub path: Option<String>,
    pub color: ViewBackgroundColor,
    pub fit: ImageFit,
    #[serde(default)]
    pub font: FontData,
    #[serde(default)]
    pub verse_font: FontData,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewBackgroundColor {
    pub a: [u8; 4], // RGBA
    pub b: [u8; 4], // RGBA
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FontData {
    pub color: [u8; 4],   // RGBA
    pub stroke: [u8; 4],  // RGBA
    pub stroke_size: f32, // pixels
    pub font_size: f32,   // pixels
}

impl Default for FontData {
    fn default() -> Self {
        Self {
            color: [255, 255, 255, 255], // White
            stroke: [0, 0, 0, 255],      // Black
            stroke_size: 2.0,
            font_size: 24.0, // Default for verse
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum ImageFit {
    Fill,
    Contain,
    Cover,
}

impl<'a> From<&'a MediaItem> for ViewData {
    fn from(value: &'a MediaItem) -> Self {
        let color = ui::ViewBackgroundColor {
            a: slint::Color::from_argb_u8(
                value.color.a[3],
                value.color.a[0],
                value.color.a[1],
                value.color.a[2],
            ),
            b: slint::Color::from_argb_u8(
                value.color.b[3],
                value.color.b[0],
                value.color.b[1],
                value.color.b[2],
            ),
        };

        let img_fit = match value.fit {
            ImageFit::Fill => i_slint_core::items::ImageFit::Fill,
            ImageFit::Contain => i_slint_core::items::ImageFit::Contain,
            ImageFit::Cover => i_slint_core::items::ImageFit::Cover,
        };

        let font = ViewFontData {
            name: SharedString::new(),
            color: Color::from_argb_u8(
                value.font.color[3],
                value.font.color[0],
                value.font.color[1],
                value.font.color[2],
            ),
            stroke: Color::from_argb_u8(
                value.font.stroke[3],
                value.font.stroke[0],
                value.font.stroke[1],
                value.font.stroke[2],
            ),
            stroke_size: value.font.stroke_size,
            font_size: value.font.font_size,
        };

        let verse_font = ViewFontData {
            name: SharedString::new(),
            color: Color::from_argb_u8(
                value.verse_font.color[3],
                value.verse_font.color[0],
                value.verse_font.color[1],
                value.verse_font.color[2],
            ),
            stroke: Color::from_argb_u8(
                value.verse_font.stroke[3],
                value.verse_font.stroke[0],
                value.verse_font.stroke[1],
                value.verse_font.stroke[2],
            ),
            stroke_size: value.verse_font.stroke_size,
            font_size: value.verse_font.font_size,
        };

        Self {
            tmp: value.tmp,
            is_logo: value.is_logo,
            path: value.path.clone().unwrap_or_default().to_shared_string(),
            show_img: ALL_MEDIA_FORMATS
                .iter()
                .any(|f| value.path.clone().unwrap_or_default().ends_with(f)),
            color,
            content: SharedString::default(),
            verse: SharedString::default(),
            img_bg: value
                .path
                .as_deref()
                .and_then(MediaManager::generate_video_thumbnail)
                .unwrap_or_default(),
            img_fit,
            font,
            verse_font,
        }
    }
}

impl From<ViewData> for MediaItem {
    fn from(value: ViewData) -> Self {
        let ca = value.color.a;
        let cb = value.color.b;

        let font_color = value.font.color;
        let font_stroke_color = value.font.stroke;

        let verse_font_color = value.verse_font.color;
        let verse_font_stroke_color = value.verse_font.stroke;

        Self {
            tmp: value.tmp,
            is_logo: value.is_logo,
            path: (!value.path.is_empty()).then_some(value.path.to_string()),
            color: ViewBackgroundColor {
                a: [ca.red(), ca.green(), ca.blue(), ca.alpha()],
                b: [cb.red(), cb.green(), cb.blue(), cb.alpha()],
            },
            fit: match value.img_fit {
                i_slint_core::items::ImageFit::Fill => ImageFit::Fill,
                i_slint_core::items::ImageFit::Cover => ImageFit::Cover,
                _ => ImageFit::Contain,
            },
            font: FontData {
                color: [
                    font_color.red(),
                    font_color.green(),
                    font_color.blue(),
                    font_color.alpha(),
                ],
                stroke: [
                    font_stroke_color.red(),
                    font_stroke_color.green(),
                    font_stroke_color.blue(),
                    font_stroke_color.alpha(),
                ],
                stroke_size: value.font.stroke_size,
                font_size: value.font.font_size,
            },
            verse_font: FontData {
                color: [
                    verse_font_color.red(),
                    verse_font_color.green(),
                    verse_font_color.blue(),
                    verse_font_color.alpha(),
                ],
                stroke: [
                    verse_font_stroke_color.red(),
                    verse_font_stroke_color.green(),
                    verse_font_stroke_color.blue(),
                    verse_font_stroke_color.alpha(),
                ],
                stroke_size: value.verse_font.stroke_size,
                font_size: value.verse_font.font_size,
            },
        }
    }
}

impl MediaManager {
    pub fn new(
        window: Weak<MainWindow>,
        view_window: Weak<ViewWindow>,
        data: Arc<UserData>,
    ) -> Self {
        let media_list = Arc::new(Mutex::new(data.load::<SourceMedia>()));

        Self {
            data,
            window,
            view_window,
            media_list,
            preview_video_thread: Arc::new(Mutex::new(None)),
            preview_video_playing: Arc::new(AtomicBool::new(false)),
            output_video_thread: Arc::new(Mutex::new(None)),
            output_video_playing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn initialize(&self) {
        let width = self.window.unwrap().window().size().width;
        {
            let mut media_list = self.media_list.lock().unwrap();
            media_list.retain(|m| !m.tmp);
            set_media_list(width, self.window.clone(), media_list.clone());
        }
    }

    fn save_permanent_items(data: &Arc<UserData>, settings: &SourceMedia) {
        let permanent_items: SourceMedia = settings
            .iter()
            .filter(|item| !item.tmp)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        data.save(&permanent_items);
    }

    fn sort_media_list(settings: &mut SourceMedia) {
        settings.sort_by(|a, b| match (a.is_logo, b.is_logo) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
    }

    fn unmark_all_logos(settings: &mut SourceMedia) {
        for item in settings.iter_mut() {
            item.is_logo = false;
        }
    }

    pub fn connect_callbacks(self: Arc<Self>) {
        let window = self.window.unwrap();
        let state = window.global::<ViewState>();

        window.window().on_winit_window_event({
            let window = self.window.clone();
            let media_list = self.media_list.clone();
            let resize_timer = Arc::new(Mutex::new(None::<JoinHandle<()>>));
            let last_resize = Arc::new(Mutex::new(Instant::now()));
            move |_, e| {
                if let WindowEvent::Resized(size) = e {
                    *last_resize.lock().unwrap() = Instant::now();
                    let mut resize_timer = resize_timer.lock().unwrap();

                    if let Some(handle) = resize_timer.take() {
                        drop(handle);
                    }

                    let window = window.clone();
                    let media_list = media_list.clone();
                    let last_resize = last_resize.clone();
                    let width = size.width;

                    let handle = std::thread::spawn(move || {
                        let t = Duration::from_millis(300);
                        std::thread::sleep(t);

                        let elapsed = last_resize.lock().unwrap().elapsed();
                        if elapsed >= t {
                            let media_list = media_list.lock().unwrap();
                            set_media_list(width, window, media_list.clone());
                        }
                    });

                    *resize_timer = Some(handle);
                }

                EventResult::Propagate
            }
        });

        state.on_show_logo({
            let instance = self.clone();
            move || {
                let settings = instance.media_list.lock().unwrap();

                if let Some(logo) = settings.iter().find(|item| item.is_logo) {
                    let logo_data = ViewData::from(logo);

                    instance.play_output_video(logo_data);
                } else {
                    error!("No logo configured");
                }
            }
        });

        state.on_preview_media({
            let instance = self.clone();
            move |media_data| {
                instance.play_preview_video(media_data);
            }
        });

        state.on_sync_and_play({
            let instance = self.clone();
            move |media_data| {
                instance.play_output_video(media_data);
            }
        });

        state.on_select_file({
            let window = self.window.clone();
            move || {
                let path = FileDialog::new()
                    .add_filter("Media", ALL_MEDIA_FORMATS)
                    .pick_file();

                if let Some(path) = path {
                    if let Some(window) = window.upgrade() {
                        let path_str = path.to_string_lossy().into_owned();
                        let state = window.global::<ViewState>();

                        let default_preview = state.get_select_media_preview();

                        let mut shared = ViewData {
                            color: ui::ViewBackgroundColor {
                                a: Color::from_rgb_u8(0, 0, 0),
                                b: Color::from_rgb_u8(0, 0, 0),
                            },
                            img_fit: i_slint_core::items::ImageFit::Contain,
                            font: default_preview.font,
                            verse_font: default_preview.verse_font,
                            ..ViewData::default()
                        };

                        let is_img = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| ALL_MEDIA_FORMATS.contains(&e))
                            .unwrap_or_default();

                        if is_img {
                            shared.path = path_str.to_shared_string();
                            shared.show_img = true;
                            shared.img_bg =
                                Self::generate_video_thumbnail(&path_str).unwrap_or_default();
                        } else {
                            shared.show_img = false;
                        }

                        state.set_select_media_preview(shared);
                    }
                }
            }
        });

        state.on_apply_changes({
            let data = self.data.clone();
            let main_window = self.window.clone();
            let media_list = self.media_list.clone();
            move |edit_mode| {
                let window = main_window.unwrap();
                let width = window.window().size().width;
                let state = window.global::<ViewState>();
                let mut settings = media_list.lock().unwrap();
                let preview = state.get_select_media_preview();

                let mut new_item = MediaItem::from(preview);
                if new_item.is_logo {
                    Self::unmark_all_logos(&mut settings);
                    new_item.tmp = false;
                }

                if edit_mode.editable {
                    let cols = if let a @ 1.. = ((width as f32 * 0.7) / 250.).floor() as usize {
                        a
                    } else {
                        6
                    };

                    let real_index = (edit_mode.row as usize * cols) + edit_mode.col as usize;

                    if real_index < settings.len() {
                        settings[real_index] = new_item;
                        Self::sort_media_list(&mut settings);
                        Self::save_permanent_items(&data, &settings);
                    }
                } else {
                    settings.push(new_item);
                    Self::sort_media_list(&mut settings);
                    Self::save_permanent_items(&data, &settings);
                }

                set_media_list(width, main_window.clone(), settings.clone());
            }
        });

        state.on_remove_media({
            let data = self.data.clone();
            let main_window = self.window.clone();
            let media_list = self.media_list.clone();
            move |row, idx| {
                let window = main_window.unwrap();
                let width = window.window().size().width;
                let mut settings = media_list.lock().unwrap();

                let cols = if let a @ 1.. = ((width as f32 * 0.7) / 250.).floor() as usize {
                    a
                } else {
                    6
                };

                let real_index = (row as usize * cols) + idx as usize;

                if real_index < settings.len() {
                    settings.remove(real_index);
                    Self::sort_media_list(&mut settings);
                    Self::save_permanent_items(&data, &settings);
                }

                set_media_list(width, main_window.clone(), settings.clone());
            }
        });
    }

    pub fn stop_preview_video(&self) {
        self.preview_video_playing.store(false, Ordering::Relaxed);
        if let Some(handle) = self.preview_video_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    pub fn stop_output_video(&self) {
        self.output_video_playing.store(false, Ordering::Relaxed);
        if let Some(handle) = self.output_video_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    pub fn play_preview_video(&self, media_data: ViewData) {
        self.stop_preview_video();

        let path = media_data.path.to_string();
        let source_path = PathBuf::from(&path);

        if Self::show_image(&source_path, media_data, Some(self.window.clone()), None) {
            return;
        }

        let target_window = self.window.clone();
        let video_playing = self.preview_video_playing.clone();
        video_playing.store(true, Ordering::Relaxed);

        let handle = std::thread::spawn(move || {
            Self::video_playback_loop(source_path, video_playing, Some(target_window), None);
        });

        *self.preview_video_thread.lock().unwrap() = Some(handle);
    }

    pub fn play_output_video(&self, media_data: ViewData) {
        self.stop_output_video();

        let path = media_data.path.to_string();
        let source_path = PathBuf::from(&path);

        if Self::show_image(
            &source_path,
            media_data,
            None,
            Some(self.view_window.clone()),
        ) {
            return;
        }

        let view_window = self.view_window.clone();
        let video_playing = self.output_video_playing.clone();

        video_playing.store(true, Ordering::Relaxed);

        let handle = std::thread::spawn(move || {
            Self::video_playback_loop(source_path, video_playing, None, Some(view_window));
        });

        *self.output_video_thread.lock().unwrap() = Some(handle);
    }

    fn show_image(
        source_path: &PathBuf,
        mut media_data: ViewData,
        target_window: Option<Weak<MainWindow>>,
        view_window: Option<Weak<ViewWindow>>,
    ) -> bool {
        if source_path
            .extension()
            .is_some_and(|e| IMAGE_FORMATS.contains(&e.to_str().unwrap_or_default()))
        {
            let Ok(image) = Image::load_from_path(source_path.as_path()) else {
                return false;
            };
            if let Some(window) = target_window {
                let Some(view_window) = window.upgrade() else {
                    return false;
                };
                let state = view_window.global::<ViewState>();
                let media = state.get_shared_view();
                media_data.img_bg = image.clone();
                media_data.show_img = true;
                media_data.content = media.content;
                state.set_shared_view(media_data.clone());
            }

            if let Some(window) = view_window {
                let Some(view_window) = window.upgrade() else {
                    return false;
                };
                let state = view_window.global::<ViewState>();
                let media = state.get_shared_view();
                media_data.img_bg = image.clone();
                media_data.show_img = true;
                media_data.content = media.content;
                state.set_shared_view(media_data);
            }
            return true;
        }

        false
    }

    fn video_playback_loop(
        source_path: PathBuf,
        video_playing: Arc<AtomicBool>,
        target_window: Option<Weak<MainWindow>>,
        view_window: Option<Weak<ViewWindow>>,
    ) {
        let Ok(src_file) = File::open(&source_path) else {
            error!("Cannot open video file: {source_path:?}");
            return;
        };

        let size = src_file.metadata().unwrap().len();
        let reader = BufReader::new(src_file);
        let Ok(mut mp4_reader) = Mp4Reader::read_header(reader, size) else {
            error!("Cannot read MP4 header: {source_path:?}");
            return;
        };

        let Some((_, track)) = mp4_reader
            .tracks()
            .iter()
            .find(|(_, t)| t.media_type().ok() == Some(mp4::MediaType::H264))
        else {
            error!("No H264 track found");
            return;
        };

        let decoder_options = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
        let Ok(mut decoder) = Decoder::with_api_config(OpenH264API::from_source(), decoder_options)
        else {
            error!("Cannot create decoder");
            return;
        };

        let sample_count = track.sample_count();
        let track_id = track.track_id();
        let width = track.width() as usize;
        let height = track.height() as usize;
        let wait_time = Duration::from_secs_f64(1.0 / track.frame_rate());
        let mut bitstream_converter = Mp4BitstreamConverter::for_mp4_track(track).unwrap();
        let mut buffer = Vec::new();

        'video_loop: loop {
            if !video_playing.load(Ordering::Relaxed) {
                break;
            }

            let frame_start = Instant::now();
            let mut frame_count = 0;

            for i in 1..=sample_count {
                if !video_playing.load(Ordering::Relaxed) {
                    break 'video_loop;
                }

                let Some(sample) = mp4_reader.read_sample(track_id, i).ok().flatten() else {
                    continue;
                };

                bitstream_converter.convert_packet(&sample.bytes, &mut buffer);

                if let Ok(Some(image)) = decoder.decode(&buffer) {
                    let mut pixels = SharedPixelBuffer::<Rgb8Pixel>::new(width as _, height as _);
                    image.write_rgb8(pixels.make_mut_bytes());

                    let target_window_clone = target_window.clone();
                    let view_window_clone = view_window.clone();
                    let video_playing = video_playing.clone();

                    let _ = slint::invoke_from_event_loop(move || {
                        if !video_playing.load(Ordering::Relaxed) {
                            return;
                        }
                        if let Some(target_window) = target_window_clone {
                            if let Some(window) = target_window.upgrade() {
                                let state = window.global::<ViewState>();
                                let mut shared = state.get_shared_view();
                                shared.img_bg = Image::from_rgb8(pixels.clone());
                                shared.show_img = true;
                                state.set_shared_view(shared);
                            }
                        }

                        if let Some(view_window) = view_window_clone {
                            if let Some(view_window) = view_window.upgrade() {
                                let state = view_window.global::<ViewState>();
                                let mut shared = state.get_shared_view();
                                shared.img_bg = Image::from_rgb8(pixels);
                                shared.show_img = true;
                                state.set_shared_view(shared);
                            }
                        }
                    });

                    frame_count += 1;
                    let expected_time = frame_start + wait_time * frame_count;
                    let now = Instant::now();

                    if now < expected_time {
                        std::thread::sleep(expected_time - now);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(16));
        }

        video_playing.store(false, Ordering::Relaxed);
    }

    fn generate_video_thumbnail(source_path: &str) -> Option<Image> {
        let source_path = PathBuf::from(source_path);
        if source_path
            .extension()
            .is_some_and(|e| IMAGE_FORMATS.contains(&e.to_str().unwrap_or_default()))
        {
            return Image::load_from_path(&source_path).ok();
        }
        let src_file = File::open(source_path)
            .inspect_err(|e| error!("Cannot open file: {e}"))
            .ok()?;
        let size = src_file
            .metadata()
            .inspect_err(|e| error!("Cannot get metadata: {e}"))
            .ok()?
            .len();
        let reader = BufReader::new(src_file);
        let mut mp4_reader = Mp4Reader::read_header(reader, size)
            .inspect_err(|e| error!("Cannot read mp4 reader: {e}"))
            .ok()?;

        let (_, track) = mp4_reader
            .tracks()
            .iter()
            .find(|(_, t)| t.media_type().ok() == Some(mp4::MediaType::H264))?;

        let decoder_options = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
        let mut decoder = Decoder::with_api_config(OpenH264API::from_source(), decoder_options)
            .inspect_err(|e| error!("Cannot decode: {e}"))
            .ok()?;

        let track_id = track.track_id();
        let width = track.width() as usize;
        let height = track.height() as usize;
        let mut bitstream_converter = Mp4BitstreamConverter::for_mp4_track(track)
            .inspect_err(|e| error!("Cannot convert mp4 bitstream: {e}"))
            .ok()?;
        let mut buffer = Vec::new();

        let sidx = track.sample_count().checked_div(2).unwrap_or(1);

        for i in sidx..=track.sample_count() {
            let Some(sample) = mp4_reader.read_sample(track_id, i).ok().flatten() else {
                continue;
            };

            bitstream_converter.convert_packet(&sample.bytes, &mut buffer);

            if let Ok(Some(image)) = decoder.decode(&buffer) {
                let mut pixels = SharedPixelBuffer::<Rgb8Pixel>::new(width as _, height as _);
                image.write_rgb8(pixels.make_mut_bytes());
                return Some(Image::from_rgb8(pixels));
            }
        }

        None
    }
}

fn set_media_list(width: u32, window: Weak<MainWindow>, media_list: SourceMedia) {
    let cols = if let a @ 1.. = ((width as f32 * 0.7) / 250.).floor() as usize {
        a
    } else {
        6
    };
    slint::invoke_from_event_loop({
        let window = window.clone();
        let media_list = media_list.clone();
        move || {
            let window = window.unwrap();
            let state = window.global::<ViewState>();
            let media_list = media_list
                .iter()
                .map(ViewData::from)
                .collect::<Vec<_>>()
                .chunks(cols)
                .map(ModelRc::from)
                .collect::<Vec<_>>();
            state.set_media_list(ModelRc::from(media_list.as_slice()));
        }
    })
    .unwrap();
}
