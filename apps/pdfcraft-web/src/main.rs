//! PdfCraft in the browser. Build: `cd apps/pdfcraft-web && trunk build --release`

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(any(target_arch = "wasm32", test))]
mod api;

#[cfg(target_arch = "wasm32")]
fn main() {
    use pdfcraft_ui_egui::PdfCraftApp;

    let options = eframe::WebOptions { renderer: eframe::Renderer::Glow, ..Default::default() };
    wasm_bindgen_futures::spawn_local(async move {
        let Some(canvas) = app_canvas() else { return };
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(|cc| {
                    let mut app = PdfCraftApp::new();
                    if let Some(json) = cc.storage.and_then(|s| s.get_string("pdfcraft").or_else(|| s.get_string("printcraft"))) {
                        app.restore(&json);
                    }
                    if let Some(url) = query_param("file") {
                        let inbox = app.startup_inbox.clone();
                        let failed = app.failed_inbox.clone();
                        let ctx = cc.egui_ctx.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let name = url.rsplit('/').next().unwrap_or("document.pdf").split('?').next().unwrap_or("document.pdf").to_string();
                            match fetch_bytes(&url).await {
                                Ok(bytes) => {
                                    if let Ok(mut q) = inbox.lock() {
                                        q.push((name, bytes));
                                    }
                                }
                                Err(e) => {
                                    eframe::web_sys::console::error_1(&format!("PdfCraft: could not fetch {url}: {e}").into());
                                    if let Ok(mut q) = failed.lock() {
                                        q.push((name, e));
                                    }
                                }
                            }
                            ctx.request_repaint();
                        });
                    }
                    warn_before_unload(app.unsaved_flag.clone());
                    if let Some(bridge) = app_canvas().as_ref().and_then(host::Bridge::connect) {
                        if let Some(author) = query_param("author").map(|a| a.trim().to_string()).filter(|a| !a.is_empty())
                            && app.comment_prefs.author == "Guest"
                        {
                            app.comment_prefs.author = author.chars().take(200).collect();
                        }
                        bridge.listen(app.startup_inbox.clone(), cc.egui_ctx.clone());
                        app.host_save = Some(bridge.save_fn());
                        app.host_dirty = Some(bridge.dirty_fn());
                        bridge.post_ready();
                    } else if query_param("host").as_deref() == Some("parent") {
                        eframe::web_sys::console::warn_1(
                            &"PdfCraft: ?host=parent needs a framing page on the same origin; saves download instead".into(),
                        );
                    }
                    if query_param("control").is_some() {
                        api::install(app.attach_control(&cc.egui_ctx));
                    }
                    Ok(Box::new(app))
                }),
            )
            .await;
        if let Some(el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("pdfcraft_loading")) {
            el.remove();
        }
        if let Err(e) = started {
            eframe::web_sys::console::error_1(&e);
            if let Some(bridge) = app_canvas().as_ref().and_then(host::Bridge::connect) {
                bridge.post_failed(&e.as_string().unwrap_or_else(|| format!("{e:?}")));
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn warn_before_unload(unsaved: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use eframe::wasm_bindgen::{JsCast, closure::Closure};
    let Some(window) = web_sys::window() else { return };
    let handler = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        if unsaved.load(std::sync::atomic::Ordering::Relaxed) {
            event.prevent_default();
        }
    });
    if window.add_event_listener_with_callback("beforeunload", handler.as_ref().unchecked_ref()).is_ok() {
        handler.forget();
    }
}

#[cfg(target_arch = "wasm32")]
fn query_param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search).ok()?.get(key)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    use eframe::wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("no window")?;
    let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await.map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "not a response")?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let buf = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?).await.map_err(|e| format!("{e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

#[cfg(target_arch = "wasm32")]
fn app_canvas() -> Option<web_sys::HtmlCanvasElement> {
    use eframe::wasm_bindgen::JsCast;
    web_sys::window()?.document()?.get_element_by_id("pdfcraft").and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
}

#[cfg(target_arch = "wasm32")]
mod host {
    use std::rc::Rc;

    use craft_web_host::web::{FileData, Host, Message};

    const APP: &str = "pdfcraft";
    const DEFAULT_NAME: &str = "document.pdf";
    const MAX_OPEN_BYTES: u64 = 512 << 20;

    #[derive(Clone)]
    pub struct Bridge {
        host: Rc<Host>,
    }

    impl Bridge {
        pub fn connect(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
            Some(Self { host: Rc::new(Host::connect(APP, canvas)?) })
        }

        pub fn post_ready(&self) {
            let _ = self.host.post_ready(env!("CARGO_PKG_VERSION"), false);
        }

        pub fn post_failed(&self, error: &str) {
            let _ = self.host.post_failed(error, None);
        }

        pub fn listen(&self, inbox: pdfcraft_ui_egui::Inbox, ctx: egui::Context) {
            self.host.listen(move |message| {
                if let Message::Open { name, data } = message {
                    queue_open(&name, &data, &inbox, &ctx);
                }
            });
        }

        pub fn save_fn(&self) -> pdfcraft_ui_egui::HostSaveFn {
            let host = self.host.clone();
            Box::new(move |name: &str, bytes: &[u8], save_as: bool| host.post_save(name, bytes, None, save_as))
        }

        pub fn dirty_fn(&self) -> pdfcraft_ui_egui::HostDirtyFn {
            let host = self.host.clone();
            let mut last = false;
            Box::new(move |dirty: bool| {
                if dirty != last {
                    last = dirty;
                    host.post_dirty(dirty);
                }
            })
        }
    }

    fn queue_open(name: &str, data: &FileData, inbox: &pdfcraft_ui_egui::Inbox, ctx: &egui::Context) {
        let name = if name == "document" { DEFAULT_NAME } else { name }.to_string();
        let FileData::Bytes(view) = data else {
            eframe::web_sys::console::warn_1(&format!("PdfCraft: host file {name} came as a File; PdfCraft reads bytes only; not opened").into());
            return;
        };
        if data.size() > MAX_OPEN_BYTES as f64 {
            eframe::web_sys::console::warn_1(
                &format!("PdfCraft: host file is {} bytes, over the {} MiB host open limit; not opened", data.size(), MAX_OPEN_BYTES >> 20).into(),
            );
            return;
        }
        if let Ok(mut q) = inbox.lock() {
            q.push((name, view.to_vec()));
        }
        ctx.request_repaint();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("pdfcraft-web targets wasm32: run `trunk serve` or `trunk build --release` in apps/pdfcraft-web");
}
