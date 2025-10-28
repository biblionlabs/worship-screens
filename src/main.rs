use std::os::fd::OwnedFd;
use std::sync::{Arc, Mutex};

use ashpd::desktop::PersistMode;
use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType, Stream};
use pipewire::{self as pw, properties::properties, spa};
use slint::{ComponentHandle, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use ui::*;

struct UserData {
    format: spa::param::video::VideoInfoRaw,
}

async fn open_portal() -> ashpd::Result<(Vec<Stream>, OwnedFd)> {
    let proxy = Screencast::new().await?;
    let session = proxy.create_session().await?;
    proxy
        .select_sources(
            &session,
            CursorMode::Embedded,
            SourceType::Monitor | SourceType::Window,
            true,
            None,
            PersistMode::Application,
        )
        .await?;

    let response = proxy.start(&session, None).await?.response()?;
    let fd = proxy.open_pipe_wire_remote(&session).await?;

    Ok((response.streams().to_vec(), fd))
}

fn convert_to_rgba(
    data: &[u8],
    format: &spa::param::video::VideoInfoRaw,
    width: u32,
    height: u32,
) -> Vec<u8> {
    use pw::spa::param::video::VideoFormat;

    let size = (width * height * 4) as usize;
    let mut rgba = vec![0u8; size];

    match format.format() {
        VideoFormat::RGBA => {
            let copy_size = size.min(data.len());
            rgba[..copy_size].copy_from_slice(&data[..copy_size]);
        }
        VideoFormat::RGBx => {
            for i in 0..(width * height) as usize {
                if i * 4 + 2 >= data.len() {
                    break;
                }
                let src_idx = i * 4;
                let dst_idx = i * 4;
                rgba[dst_idx] = data[src_idx]; // R
                rgba[dst_idx + 1] = data[src_idx + 1]; // G
                rgba[dst_idx + 2] = data[src_idx + 2]; // B
                rgba[dst_idx + 3] = 255; // A
            }
        }
        VideoFormat::BGRx => {
            for i in 0..(width * height) as usize {
                if i * 4 + 2 >= data.len() {
                    break;
                }
                let src_idx = i * 4;
                let dst_idx = i * 4;
                rgba[dst_idx] = data[src_idx + 2]; // R (era B)
                rgba[dst_idx + 1] = data[src_idx + 1]; // G
                rgba[dst_idx + 2] = data[src_idx]; // B (era R)
                rgba[dst_idx + 3] = 255; // A
            }
        }
        VideoFormat::RGB => {
            for i in 0..(width * height) as usize {
                if i * 3 + 2 >= data.len() {
                    break;
                }
                let src_idx = i * 3;
                let dst_idx = i * 4;
                rgba[dst_idx] = data[src_idx]; // R
                rgba[dst_idx + 1] = data[src_idx + 1]; // G
                rgba[dst_idx + 2] = data[src_idx + 2]; // B
                rgba[dst_idx + 3] = 255; // A
            }
        }
        VideoFormat::BGR => {
            for i in 0..(width * height) as usize {
                if i * 3 + 2 >= data.len() {
                    break;
                }
                let src_idx = i * 3;
                let dst_idx = i * 4;
                rgba[dst_idx] = data[src_idx + 2]; // R
                rgba[dst_idx + 1] = data[src_idx + 1]; // G
                rgba[dst_idx + 2] = data[src_idx]; // B
                rgba[dst_idx + 3] = 255; // A
            }
        }
        _ => {
            println!("⚠️ Formato no soportado: {:?}", format.format());
        }
    }

    rgba
}

fn start_streaming(
    node_id: u32,
    fd: OwnedFd,
    callback: Arc<Mutex<impl FnMut(u32, u32, Vec<u8>) + Send + 'static>>,
) -> Result<(), pw::Error> {
    std::thread::spawn(move || {
        pw::init();

        let mainloop = pw::main_loop::MainLoop::new(None).unwrap();
        let context = pw::context::Context::new(&mainloop).unwrap();
        let core = context.connect_fd(fd, None).unwrap();

        let data = UserData {
            format: Default::default(),
        };

        let stream = pw::stream::Stream::new(
            &core,
            "Slint Screen Capture",
            properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        )
        .unwrap();

        let callback = callback.clone();

        let _listener = stream
            .add_local_listener_with_user_data(data)
            .state_changed(|stream, _, old, new| {
                println!("📡 Estado: {:?} -> {:?}", old, new);

                if new == pw::stream::StreamState::Paused {
                    println!("▶️ Activando stream...");
                    stream
                        .set_active(true)
                        .expect("No se pudo activar el stream");
                }
            })
            .param_changed(|_stream, user_data, id, param| {
                let Some(param) = param else {
                    return;
                };

                println!(
                    "🔧 Param changed: {:?}",
                    pw::spa::param::ParamType::from_raw(id)
                );

                if id != pw::spa::param::ParamType::Format.as_raw() {
                    return;
                }

                let (media_type, media_subtype) =
                    match pw::spa::param::format_utils::parse_format(param) {
                        Ok(v) => v,
                        Err(e) => {
                            println!("❌ Error parseando formato: {}", e);
                            return;
                        }
                    };

                if media_type != pw::spa::param::format::MediaType::Video
                    || media_subtype != pw::spa::param::format::MediaSubtype::Raw
                {
                    println!("⚠️ Media type/subtype no es Video/Raw");
                    return;
                }

                match user_data.format.parse(param) {
                    Ok(_) => {
                        println!("✅ Formato de video configurado:");
                        println!("   Formato: {:?}", user_data.format.format());
                        println!(
                            "   Tamaño: {}x{}",
                            user_data.format.size().width,
                            user_data.format.size().height
                        );
                        println!(
                            "   FPS: {}/{}",
                            user_data.format.framerate().num,
                            user_data.format.framerate().denom
                        );
                    }
                    Err(e) => {
                        println!("❌ Error parseando VideoInfoRaw: {}", e);
                    }
                }
            })
            .process(move |stream, user_data| match stream.dequeue_buffer() {
                None => {
                    println!("⚠️ Sin buffers disponibles");
                }
                Some(mut buffer) => {
                    let datas = buffer.datas_mut();
                    if datas.is_empty() {
                        println!("⚠️ Buffer sin datos");
                        return;
                    }

                    let data_chunk = &mut datas[0];
                    let chunk = data_chunk.chunk();
                    let stride = chunk.stride();
                    let chunk_size = chunk.size() as usize;
                    let chunk_offset = chunk.offset() as usize;

                    println!(
                        "📦 Frame recibido - stride: {}, size: {}, offset: {}",
                        stride, chunk_size, chunk_offset
                    );

                    // Intentar obtener datos mapeados primero
                    let data_slice = if let Some(slice) = data_chunk.data() {
                        slice
                    } else {
                        println!("⚠️ No se pudo mapear el buffer");
                        return;
                    };

                    let width = user_data.format.size().width;
                    let height = user_data.format.size().height;

                    if width == 0 || height == 0 {
                        println!("⚠️ Dimensiones inválidas: {}x{}", width, height);
                        return;
                    }

                    let bytes_per_pixel = match user_data.format.format() {
                        pw::spa::param::video::VideoFormat::RGB
                        | pw::spa::param::video::VideoFormat::BGR => 3,
                        _ => 4,
                    };

                    // Usar el tamaño del chunk si es válido, sino calcular
                    let actual_size = if chunk_size > 1 {
                        chunk_size
                    } else {
                        (height as usize) * (stride as usize)
                    };

                    if chunk_offset + actual_size > data_slice.len() {
                        println!(
                            "⚠️ Buffer muy pequeño: offset={}, size={}, len={}",
                            chunk_offset,
                            actual_size,
                            data_slice.len()
                        );
                        return;
                    }

                    // Extraer datos desde el offset correcto
                    let source_data = &data_slice[chunk_offset..chunk_offset + actual_size];

                    let expected_stride = (width * bytes_per_pixel) as i32;

                    let frame_data = if stride == expected_stride {
                        source_data.to_vec()
                    } else {
                        let mut result =
                            Vec::with_capacity((width * height * bytes_per_pixel) as usize);
                        for y in 0..height {
                            let offset = (y as i32 * stride) as usize;
                            let row_size = (width * bytes_per_pixel) as usize;
                            if offset + row_size <= source_data.len() {
                                result.extend_from_slice(&source_data[offset..offset + row_size]);
                            }
                        }
                        result
                    };

                    if frame_data.is_empty() {
                        println!("⚠️ frame_data está vacío");
                        return;
                    }

                    let rgba_data = convert_to_rgba(&frame_data, &user_data.format, width, height);

                    if let Ok(mut cb) = callback.lock() {
                        cb(width, height, rgba_data);
                    }
                }
            })
            .register()
            .unwrap();

        stream
            .connect(
                spa::utils::Direction::Input,
                Some(node_id),
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
                &mut [],
            )
            .unwrap();

        println!("🔌 Stream conectado al nodo {}", node_id);

        mainloop.run();
    });

    Ok(())
}

#[tokio::main]
async fn main() {
    let main_window = MainWindow::new().unwrap();

    main_window.on_select_sources({
        let main_window = main_window.as_weak();
        move || {
            let main_window_clone = main_window.clone();

            tokio::spawn(async move {
                match open_portal().await {
                    Ok((streams, fd)) => {
                        println!("🚀 Portal abierto con {} stream(s)", streams.len());

                        for stream in streams {
                            let (width, height) = stream.size().unwrap();
                            let node_id = stream.pipe_wire_node_id();
                            println!("📺 Stream: {}x{} (nodo: {})", width, height, node_id);

                            let main_window_ref = main_window_clone.clone();

                            let callback =
                                Arc::new(Mutex::new(move |w: u32, h: u32, data: Vec<u8>| {
                                    let main_window_ref = main_window_ref.clone();
                                    let data_clone = data.clone();

                                    println!("🎨 Actualizando imagen en UI...");
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(window) = main_window_ref.upgrade() {
                                            let mut buff =
                                                SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
                                            buff.make_mut_bytes().copy_from_slice(&data_clone);

                                            let image = slint::Image::from_rgba8(buff);
                                            window.set_sources(ModelRc::new(VecModel::from(vec![
                                                image,
                                            ])));
                                            println!("✅ Imagen actualizada");
                                        }
                                    });
                                }));

                            if let Err(e) =
                                start_streaming(node_id, fd.try_clone().unwrap(), callback)
                            {
                                eprintln!("❌ Error iniciando streaming: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error abriendo portal: {}", e);
                    }
                }
            });
        }
    });

    main_window.run().unwrap();
}
