use scribe_core::capture::{MonitorPlacement, PendingCapture, PixelRect};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{
    AppHandle, Manager, Runtime, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::screenshots::capture::ScreenshotService;
use crate::screenshots::capture_gate::RegionRefusal;
use crate::screenshots::placement::covering;
use crate::screenshots::window_level::raise_above_menu_bar;

const WINDOW_LABEL: &str = "region-selector";
const WINDOW_ROUTE: &str = "index.html#region-selector";
const WINDOW_TITLE: &str = "Region capture";

#[derive(Debug, Serialize)]
pub struct RegionContext {
    frame_width: u32,
    frame_height: u32,
    outlines: Vec<PixelRect>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RegionCompletion {
    Captured,
    TooSmall,
}

pub fn open<R: Runtime>(app: &AppHandle<R>, placement: &MonitorPlacement) -> tauri::Result<()> {
    let window = build_hidden(app)?;
    let bounds = covering(placement);
    window.set_position(bounds.position)?;
    window.set_size(bounds.size)?;
    raise_above_menu_bar(&window);
    cancel_on_blur(app, &window);
    window.show()?;
    window.set_focus()
}

pub fn close<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        if let Err(error) = window.close() {
            tracing::warn!(%error, "could not close the region selector");
        }
    }
}

fn cancel<R: Runtime>(app: &AppHandle<R>) {
    app.state::<ScreenshotService>().cancel_region();
    close(app);
}

fn build_hidden<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App(WINDOW_ROUTE.into()))
        .title(WINDOW_TITLE)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .shadow(false)
        .focused(true)
        .visible(false)
        .build()
}

fn cancel_on_blur<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    let app = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            cancel(&app);
        }
    });
}

#[tauri::command]
pub async fn region_frame(service: State<'_, ScreenshotService>) -> Result<Response, String> {
    service
        .read_pending(|pending| Response::new(pending.shot().frame.rgba().to_vec()))
        .ok_or_else(|| RegionRefusal::NothingPending.to_string())
}

#[tauri::command]
pub async fn region_context(
    service: State<'_, ScreenshotService>,
) -> Result<RegionContext, String> {
    service
        .read_pending(context_of)
        .ok_or_else(|| RegionRefusal::NothingPending.to_string())
}

fn context_of(pending: &PendingCapture) -> RegionContext {
    let shot = pending.shot();
    RegionContext {
        frame_width: shot.frame.width(),
        frame_height: shot.frame.height(),
        outlines: shot.outlines.iter().map(|outline| outline.bounds).collect(),
    }
}

#[tauri::command]
pub async fn complete_region<R: Runtime>(
    app: AppHandle<R>,
    area: PixelRect,
) -> Result<RegionCompletion, String> {
    match app.state::<ScreenshotService>().complete_region(&app, area) {
        Ok(()) => Ok(RegionCompletion::Captured),
        Err(RegionRefusal::SelectionRefused) => Ok(RegionCompletion::TooSmall),
        Err(refusal) => Err(refusal.to_string()),
    }
}

#[tauri::command]
pub async fn cancel_region<R: Runtime>(app: AppHandle<R>) {
    cancel(&app);
}
