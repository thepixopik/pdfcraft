//! Home tab: recommended tools, open card, pinned folders and recent files.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};
use pdfcraft_engine::catalog;

use crate::theme::{self, Tokens};
use crate::{PdfCraftApp, icons, panels::human_size, widgets};

pub mod folders_ui;

const RECOMMENDED: [&str; 5] = ["organize", "comment", "form", "edit", "protect"];

pub(crate) fn card_lift(ui: &egui::Ui, t: &Tokens, rect: Rect, press: theme::Press) {
    let lift = press.hover * (1.0 - press.down);
    if lift > 0.04 {
        let shadow = egui::Shadow { offset: [0, 3], blur: (14.0 * lift) as u8, spread: 0, color: t.page_shadow.gamma_multiply(0.85 * lift) };
        ui.painter().add(shadow.as_shape(rect, CornerRadius::same(10)));
    }
}

fn tool_card(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, g: &'static catalog::ToolGroup, allow_unpin: bool) -> Option<&'static str> {
    let (rect, resp) = ui.allocate_exact_size(vec2(190.0, 104.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!(g.label)));
    let press = theme::Press::track(ui, &resp);
    card_lift(ui, t, rect, press);
    let fill = press.fill(t.card, t.hover, t.pressed);
    ui.painter().rect(rect, CornerRadius::same(10), fill, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
    let body = rect.translate(press.offset());
    let color = egui::Color32::from_rgb(g.hue[0], g.hue[1], g.hue[2]);
    icons::paint(ui, Rect::from_min_size(body.min + vec2(14.0, 14.0), vec2(22.0, 22.0)), g.icon, 21.0, color);

    let mut unpin_clicked = false;
    if allow_unpin {
        let unpin_rect = Rect::from_center_size(body.right_top() + vec2(-18.0, 18.0), vec2(22.0, 22.0));
        let unpin_resp = ui.interact(unpin_rect, ui.scope_id().with(("unpin_card_btn", g.id)), Sense::click());
        unpin_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Unpin tool")));
        if unpin_resp.hovered() {
            ui.painter().rect_filled(unpin_rect, CornerRadius::same(4), t.pressed);
        }
        icons::paint(ui, unpin_rect.shrink(4.0), "x", 14.0, t.icon);
        if unpin_resp.on_hover_text(tl!("Unpin from Home")).clicked() {
            unpin_clicked = true;
        }
    }

    let mut title_job = egui::text::LayoutJob::simple_singleline(tl!(g.label).to_owned(), theme::semibold(13.5), t.text);
    title_job.wrap = egui::text::TextWrapping { max_width: body.width() - 58.0, max_rows: 1, break_anywhere: true, ..Default::default() };
    let title = ui.fonts_mut(|f| f.layout_job(title_job));
    let title_elided = title.elided;
    ui.painter().galley(body.min + vec2(44.0, 25.0 - title.size().y * 0.5), title, t.text);
    let blurb = g.sections.first().map(|s| s.items.iter().take(3).map(|i| tl!(i.label)).collect::<Vec<_>>().join(" · ")).unwrap_or_default();
    let mut job = egui::text::LayoutJob::simple(blurb.clone(), theme::regular(11.5), t.text_muted, body.width() - 28.0);
    job.wrap.max_rows = 2;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let blurb_elided = galley.elided;
    ui.painter().galley(body.min + vec2(14.0, 46.0), galley, t.text_muted);
    ui.painter().text(body.left_bottom() + vec2(14.0, -14.0), Align2::LEFT_CENTER, tl!("Use now"), theme::medium(12.0), t.accent_text);
    let resp = if title_elided || blurb_elided { resp.on_hover_text(format!("{}\n{blurb}", tl!(g.label))) } else { resp };
    if unpin_clicked {
        Some(g.id)
    } else {
        if theme::hand(resp).clicked() {
            app.select_tool(g.id);
        }
        None
    }
}

pub fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin { left: 36, right: 36, top: 28, bottom: 28 }).show(ui, |ui| {
            ui.label(egui::RichText::new("Welcome to The AllCraft PDF").font(theme::semibold(24.0)));
            ui.label(
                egui::RichText::new(tl!("An open-source PDF workbench — local, private, and scriptable."))
                    .color(t.text_muted)
                    .font(theme::regular(14.0)),
            );
            ui.add_space(14.0);
            if !app.community_card_closed {
                egui::Frame::NONE
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.border))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            widgets::artcraft_mark(ui, 28.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("The AllCraft Studio").font(theme::semibold(15.0)));
                                ui.label(egui::RichText::new("Switch between Vector, Photo, Film & PDF tools or browse source on GitHub.").color(t.text_muted));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if icons::button(ui, "x", 24.0, false, tl!("Close")).clicked() {
                                    app.community_card_closed = true;
                                }
                            });
                        });
                        ui.add_space(8.0);
                        if let Some(cmd) = widgets::community_links(ui) {
                            app.execute(cmd);
                        }
                    });
                ui.add_space(22.0);
            }

            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(tl!("Recommended tools")).font(theme::semibold(15.0)));
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                        for id in RECOMMENDED {
                            if app.is_tool_pinned(id) {
                                continue;
                            }
                            let Some(g) = catalog::group(id) else { continue };
                            tool_card(app, ui, &t, g, false);
                        }
                        let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 104.0), Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Open file")));
                        let press = theme::Press::track(ui, &resp);
                        card_lift(ui, &t, rect, press);
                        ui.painter().rect(
                            rect,
                            CornerRadius::same(10),
                            press.fill(t.pasteboard, t.hover, t.pressed),
                            Stroke::new(1.0, t.divider),
                            egui::StrokeKind::Inside,
                        );
                        let body = rect.translate(press.offset());
                        icons::paint(ui, Rect::from_center_size(body.center() - vec2(0.0, 16.0), vec2(28.0, 28.0)), "folder-open", 26.0, t.icon);
                        ui.painter().text(body.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, tl!("Open file"), theme::semibold(13.0), t.text);
                        if theme::hand(resp).clicked() {
                            app.open_dialog();
                        }
                    });
                });

            ui.add_space(26.0);
            let mut unpin_tool = None;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tl!("Pinned Tools")).font(theme::semibold(17.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::ghost_button(ui, "pin", tl!("Pin a tool…")).clicked() {
                        app.dialog = Some(crate::Dialog::PinTool);
                    }
                });
            });
            ui.add_space(4.0);
            if app.pinned_tools.is_empty() {
                ui.label(egui::RichText::new(tl!("Pin your favorite tools here for quick access.")).color(t.text_muted));
            } else {
                ui.add_space(6.0);
                let pinned = app.pinned_tools.clone();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                    for id in &pinned {
                        let Some(g) = catalog::group(id) else { continue };
                        if let Some(to_unpin) = tool_card(app, ui, &t, g, true) {
                            unpin_tool = Some(to_unpin);
                        }
                    }
                });
            }
            if let Some(id) = unpin_tool {
                app.unpin_tool(id);
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(26.0);
                crate::folders_ui::section(app, ui);
            }

            let mut clear = false;
            if app.recent_limit > 0 {
                ui.add_space(26.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tl!("Recent")).font(theme::semibold(17.0)));
                    if !app.recent.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let resp = widgets::ghost_button(ui, "trash-2", tl!("Clear"));
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Clear Recent Files")));
                            clear = resp.on_hover_text(tl!("Clear Recent Files")).clicked();
                        });
                    }
                });
                ui.add_space(8.0);
                if app.recent.is_empty() {
                    ui.label(egui::RichText::new(tl!("Files you open in PdfCraft appear here. Drop a PDF anywhere to open it.")).color(t.text_muted));
                }
            }
            let mut open = None;
            let mut remove = None;
            #[cfg(not(target_arch = "wasm32"))]
            let mut reveal = None;
            for r in app.recent.iter().take(app.recent_limit) {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &r.name));
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let by_key =
                        resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Menu) || (i.modifiers.shift && i.key_pressed(egui::Key::F10)));
                    let mut popup = egui::Popup::context_menu(&resp);
                    if by_key {
                        popup = popup.open_memory(egui::SetOpenCommand::Bool(true)).anchor(rect).align(egui::RectAlign::BOTTOM_START);
                    }
                    popup.show(|ui| {
                        let label = if cfg!(target_os = "macos") { tl!("Reveal in Finder") } else { tl!("Reveal in File Explorer") };
                        if ui.button(label).clicked() {
                            reveal = Some(r.path.clone());
                            ui.close();
                        }
                    });
                }
                let press = theme::Press::track(ui, &resp);
                let hovered = ui.rect_contains_pointer(rect);
                if hovered && press.hover < 0.04 && press.down < 0.04 {
                    ui.painter().rect_filled(rect, CornerRadius::same(8), t.hover);
                } else {
                    press.wash(ui, rect, 8, false);
                }
                if hovered {
                    let x_rect = Rect::from_center_size(rect.right_center() - vec2(26.0, 0.0), vec2(28.0, 28.0));
                    let x = ui.interact(x_rect, ui.scope_id().with(("remove-recent", &r.path)), Sense::click());
                    let label = crate::i18n::fmt(tl!("Remove {name} from Recent"), &[("name", &r.name)]);
                    x.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                    if x.hovered() {
                        ui.painter().rect_filled(x_rect, CornerRadius::same(6), t.pressed);
                    }
                    icons::paint(ui, x_rect.shrink(6.0), "x", 16.0, t.icon);
                    if x.on_hover_text(tl!("Remove from Recent")).clicked() {
                        remove = Some(r.path.clone());
                    }
                }
                let body = rect.translate(press.offset());
                icons::paint(
                    ui,
                    Rect::from_min_size(body.min + vec2(10.0, 11.0), vec2(24.0, 24.0)),
                    "file-text",
                    22.0,
                    egui::Color32::from_rgb(0x10, 0xB9, 0x81),
                );
                let detail = ui.painter().text(
                    body.right_center() - vec2(48.0, 0.0),
                    Align2::RIGHT_CENTER,
                    format!("{}  ·  {}", crate::i18n::trn(crate::i18n::current(), r.pages as u64, "{n} page", "{n} pages"), human_size(r.size)),
                    theme::regular(12.0),
                    t.text_muted,
                );
                let width = detail.left() - 16.0 - (body.left() + 46.0);
                widgets::row_text(ui, body.min + vec2(46.0, 15.0), r.name.as_str(), theme::medium(13.5), t.text, width);
                widgets::row_text(ui, body.min + vec2(46.0, 32.0), r.path.as_str(), theme::regular(11.0), t.text_faint, width);
                if theme::hand(resp).on_hover_text(&r.path).clicked() {
                    open = Some(r.path.clone());
                }
            }
            if let Some(p) = open {
                app.open_recent(&p);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(p) = reveal {
                app.reveal_path(std::path::Path::new(&p));
            }
            if let Some(p) = remove {
                app.recent.retain(|r| r.path != p);
            }
            if clear {
                app.execute("file.clear_recent");
            }
            ui.add_space(20.0);
            widgets::section_title(ui, tl!("Privacy"));
            ui.label(
                egui::RichText::new("The AllCraft PDF works offline in your browser. No telemetry, no account, and zero cloud uploads.")
                    .color(t.text_muted),
            );
        });
    });
}
