use crate::capture;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};
use pipewire as pw;
use pw::properties::properties;
use spa::pod::Pod;
use std::collections::HashMap;
use std::io::Cursor;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::task;
use tokio::time;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const SCREENCAST_DEST: &str = "org.gnome.Mutter.ScreenCast";
const SCREENCAST_PATH: &str = "/org/gnome/Mutter/ScreenCast";
const SCREENCAST_IFACE: &str = "org.gnome.Mutter.ScreenCast";
const SESSION_IFACE: &str = "org.gnome.Mutter.ScreenCast.Session";
const STREAM_IFACE: &str = "org.gnome.Mutter.ScreenCast.Stream";
const CURSOR_MODE_EMBEDDED: u32 = 1;

#[derive(Clone, Debug)]
pub struct ScreencastSession {
    pub session_path: OwnedObjectPath,
}

#[derive(Debug)]
pub struct ScreenshotResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct StartedScreencast {
    pub node_id: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub session: ScreencastSession,
}

pub async fn screenshot(
    conn: &zbus::Connection,
    monitor_connector: &str,
) -> Result<ScreenshotResult> {
    match screenshot_via_pipewire(conn, monitor_connector).await {
        Ok(result) => Ok(result),
        Err(error) => {
            let path = capture::fallback_screenshot(None).await.with_context(|| {
                format!("PipeWire screenshot failed and fallback failed: {error:#}")
            })?;
            Ok(ScreenshotResult {
                path,
                width: 0,
                height: 0,
            })
        }
    }
}

pub async fn start_screencast(
    conn: &zbus::Connection,
    monitor_connector: &str,
    framerate: u32,
) -> Result<StartedScreencast> {
    let (session_path, stream_path) = create_monitor_session(conn, monitor_connector).await?;
    let stream_proxy = zbus::Proxy::new(conn, SCREENCAST_DEST, stream_path.as_str(), STREAM_IFACE)
        .await
        .context("creating screencast stream proxy")?;
    let mut signal_stream = stream_proxy
        .receive_signal("PipeWireStreamAdded")
        .await
        .context("subscribing to PipeWireStreamAdded")?;

    let session_proxy =
        zbus::Proxy::new(conn, SCREENCAST_DEST, session_path.as_str(), SESSION_IFACE)
            .await
            .context("creating screencast session proxy")?;
    let _: () = session_proxy
        .call("Start", &())
        .await
        .context("starting Mutter screencast session")?;

    let message = time::timeout(Duration::from_secs(5), signal_stream.next())
        .await
        .context("timed out waiting for PipeWireStreamAdded")?
        .ok_or_else(|| anyhow!("PipeWireStreamAdded stream ended unexpectedly"))?;
    let node_id = message
        .body()
        .deserialize::<(u32,)>()
        .context("deserializing PipeWireStreamAdded signal")?
        .0;

    let (width, height) = stream_dimensions(&stream_proxy).await?;
    let _ = framerate;

    Ok(StartedScreencast {
        node_id,
        width,
        height,
        session: ScreencastSession { session_path },
    })
}

pub async fn stop_screencast(conn: &zbus::Connection, session: &ScreencastSession) -> Result<()> {
    let session_proxy = zbus::Proxy::new(
        conn,
        SCREENCAST_DEST,
        session.session_path.as_str(),
        SESSION_IFACE,
    )
    .await
    .context("creating screencast session proxy")?;
    let _: () = session_proxy
        .call("Stop", &())
        .await
        .context("stopping screencast")?;
    Ok(())
}

async fn screenshot_via_pipewire(
    conn: &zbus::Connection,
    monitor_connector: &str,
) -> Result<ScreenshotResult> {
    let started = start_screencast(conn, monitor_connector, 1).await?;
    let node_id = started.node_id;
    let session = started.session.clone();

    let capture_result = task::spawn_blocking(move || capture_single_frame(node_id))
        .await
        .context("joining PipeWire screenshot task")??;

    let stop_result = stop_screencast(conn, &session).await;
    if let Err(error) = stop_result {
        return Err(error).context("captured frame but failed to stop screencast session");
    }

    Ok(capture_result)
}

async fn create_monitor_session(
    conn: &zbus::Connection,
    monitor_connector: &str,
) -> Result<(OwnedObjectPath, OwnedObjectPath)> {
    let screencast_proxy =
        zbus::Proxy::new(conn, SCREENCAST_DEST, SCREENCAST_PATH, SCREENCAST_IFACE)
            .await
            .context("creating Mutter screencast proxy")?;

    let create_properties = vardict(&[("disable-animations", Value::from(false))]);
    let session_path: OwnedObjectPath = screencast_proxy
        .call("CreateSession", &(create_properties))
        .await
        .context("creating Mutter screencast session")?;

    let session_proxy =
        zbus::Proxy::new(conn, SCREENCAST_DEST, session_path.as_str(), SESSION_IFACE)
            .await
            .context("creating screencast session proxy")?;
    let record_properties = vardict(&[
        ("cursor-mode", Value::from(CURSOR_MODE_EMBEDDED)),
        ("is-recording", Value::from(false)),
    ]);
    let stream_path: OwnedObjectPath = session_proxy
        .call("RecordMonitor", &(monitor_connector, record_properties))
        .await
        .with_context(|| format!("recording monitor {monitor_connector}"))?;

    Ok((session_path, stream_path))
}

fn capture_single_frame(node_id: u32) -> Result<ScreenshotResult> {
    pw::init();

    let mainloop = pw::main_loop::MainLoop::new(None).context("creating PipeWire main loop")?;
    let context = pw::context::Context::new(&mainloop).context("creating PipeWire context")?;
    let core = context.connect(None).context("connecting to PipeWire")?;

    #[derive(Default)]
    struct UserData {
        format: spa::param::video::VideoInfoRaw,
        sent: bool,
    }

    let (sender, receiver) = mpsc::sync_channel::<Result<ScreenshotResult>>(1);
    let mainloop_weak = mainloop.downgrade();

    let stream = pw::stream::Stream::new(
        &core,
        "deskbrid-screenshot",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .context("creating PipeWire stream")?;

    let _listener = stream
        .add_local_listener_with_user_data(UserData::default())
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }
            let _ = user_data.format.parse(param);
        })
        .process(move |stream, user_data| {
            if user_data.sent {
                return;
            }

            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };

            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let data = &mut datas[0];
            let width = user_data.format.size().width.max(0) as u32;
            let height = user_data.format.size().height.max(0) as u32;
            let result = frame_to_png(data, user_data.format.format(), width, height);
            let _ = sender.send(result);
            user_data.sent = true;

            if let Some(mainloop) = mainloop_weak.upgrade() {
                mainloop.quit();
            }
        })
        .register()
        .context("registering PipeWire stream listener")?;

    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            pw::spa::param::video::VideoFormat::BGRx,
            pw::spa::param::video::VideoFormat::BGRx,
            pw::spa::param::video::VideoFormat::RGBx,
            pw::spa::param::video::VideoFormat::RGBA,
            pw::spa::param::video::VideoFormat::RGB
        )
    );
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .context("serializing video format pod")?
    .0
    .into_inner();
    let mut params = [Pod::from_bytes(&values).context("creating format pod")?];

    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .context("connecting PipeWire stream")?;

    mainloop.run();
    receiver.recv().context("waiting for PipeWire frame")?
}

fn frame_to_png(
    data: &mut spa::buffer::Data,
    format: spa::param::video::VideoFormat,
    width: u32,
    height: u32,
) -> Result<ScreenshotResult> {
    if width == 0 || height == 0 {
        return Err(anyhow!("PipeWire video format did not report a valid size"));
    }

    let bytes = mapped_bytes(data)?;
    let stride = if data.chunk().stride() > 0 {
        data.chunk().stride() as usize
    } else {
        (width as usize) * bytes_per_pixel(format)?
    };
    let rgba = convert_frame(bytes, format, width as usize, height as usize, stride)?;

    let path = screenshot_path_sync()?;
    let file =
        std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    let encoder = PngEncoder::new(file);
    encoder
        .write_image(&rgba, width, height, ColorType::Rgba8.into())
        .context("encoding screenshot PNG")?;

    Ok(ScreenshotResult {
        path: path.display().to_string(),
        width,
        height,
    })
}

fn mapped_bytes(data: &mut spa::buffer::Data) -> Result<&[u8]> {
    if let Some(bytes) = data.data() {
        return Ok(bytes);
    }

    let raw = data.as_raw();
    let fd = raw.fd;
    if fd < 0 {
        return Err(anyhow!("PipeWire buffer has no CPU-accessible memory"));
    }

    let size = usize::try_from(raw.maxsize).context("invalid PipeWire buffer size")?;
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return Err(anyhow!("mmap on PipeWire buffer fd failed"));
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), size) };
    Ok(slice)
}

fn convert_frame(
    bytes: &[u8],
    format: spa::param::video::VideoFormat,
    width: usize,
    height: usize,
    stride: usize,
) -> Result<Vec<u8>> {
    let mut rgba = vec![0_u8; width * height * 4];
    for y in 0..height {
        let src_row = &bytes[y * stride..];
        let dst_row = &mut rgba[y * width * 4..(y + 1) * width * 4];
        match format {
            spa::param::video::VideoFormat::BGRx => {
                for x in 0..width {
                    let src = &src_row[x * 4..x * 4 + 4];
                    let dst = &mut dst_row[x * 4..x * 4 + 4];
                    dst[0] = src[2];
                    dst[1] = src[1];
                    dst[2] = src[0];
                    dst[3] = 255;
                }
            }
            spa::param::video::VideoFormat::RGBx => {
                for x in 0..width {
                    let src = &src_row[x * 4..x * 4 + 4];
                    let dst = &mut dst_row[x * 4..x * 4 + 4];
                    dst[0] = src[0];
                    dst[1] = src[1];
                    dst[2] = src[2];
                    dst[3] = 255;
                }
            }
            spa::param::video::VideoFormat::RGBA => {
                let len = width * 4;
                dst_row[..len].copy_from_slice(&src_row[..len]);
            }
            spa::param::video::VideoFormat::RGB => {
                for x in 0..width {
                    let src = &src_row[x * 3..x * 3 + 3];
                    let dst = &mut dst_row[x * 4..x * 4 + 4];
                    dst[0] = src[0];
                    dst[1] = src[1];
                    dst[2] = src[2];
                    dst[3] = 255;
                }
            }
            other => return Err(anyhow!("unsupported PipeWire video format: {other:?}")),
        }
    }
    Ok(rgba)
}

fn bytes_per_pixel(format: spa::param::video::VideoFormat) -> Result<usize> {
    match format {
        spa::param::video::VideoFormat::BGRx
        | spa::param::video::VideoFormat::RGBx
        | spa::param::video::VideoFormat::RGBA => Ok(4),
        spa::param::video::VideoFormat::RGB => Ok(3),
        other => Err(anyhow!("unsupported PipeWire video format: {other:?}")),
    }
}

async fn stream_dimensions(stream_proxy: &zbus::Proxy<'_>) -> Result<(Option<u32>, Option<u32>)> {
    let parameters: HashMap<String, OwnedValue> = stream_proxy
        .get_property("Parameters")
        .await
        .context("reading screencast stream parameters")?;
    let Some(size) = parameters.get("size") else {
        return Ok((None, None));
    };
    let (width, height): (i32, i32) = size
        .try_clone()
        .context("cloning screencast size value")?
        .try_into()
        .context("parsing screencast size")?;
    Ok((Some(width.max(0) as u32), Some(height.max(0) as u32)))
}

fn vardict(entries: &[(&str, Value<'_>)]) -> HashMap<String, Value<'_>> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

fn screenshot_path_sync() -> Result<PathBuf> {
    let directory = PathBuf::from("/tmp/deskbrid");
    std::fs::create_dir_all(&directory).context("creating screenshot output dir")?;
    Ok(directory.join(format!(
        "screenshot_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    )))
}

#[allow(dead_code)]
fn _owned_fd_identity(fd: OwnedFd) -> OwnedFd {
    fd
}
