//! # rgui CRUD Example - Contact Manager
//!
//! html! 声明式渲染表单 + 手动 PaintLayerData 渲染动态联系人列表。
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p crud
//! ```

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{
    AppMessage, Color, PaintContext, PaintLayerData, Rect,
    Size, WidgetId, WidgetView, build_scene_from_view, compute_view_layout, html,
};
use std::sync::{Arc, Mutex};

// ============================================================================
// Data structures
// ============================================================================

#[derive(Debug, Clone)]
struct Contact {
    name: String,
    email: String,
    phone: String,
}

impl Contact {
    fn new(name: &str, email: &str, phone: &str) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            phone: phone.into(),
        }
    }
}

// ============================================================================
// App state
// ============================================================================

struct AppState {
    contacts: Vec<Contact>,
    selected_index: Option<usize>,
    edit_name: String,
    edit_email: String,
    edit_phone: String,
    message: String,
}

impl AppState {
    fn new() -> Self {
        let contacts = vec![
            Contact::new("Alice", "alice@example.com", "138-0001-0001"),
            Contact::new("Bob", "bob@example.com", "138-0002-0002"),
        ];
        Self {
            contacts,
            selected_index: None,
            edit_name: String::new(),
            edit_email: String::new(),
            edit_phone: String::new(),
            message: "Welcome to Contact Manager".into(),
        }
    }

    fn add_contact(&mut self) {
        let name = if self.edit_name.is_empty() {
            "New Contact".into()
        } else {
            self.edit_name.clone()
        };
        let email = self.edit_email.clone();
        let phone = self.edit_phone.clone();
        self.contacts.push(Contact::new(&name, &email, &phone));
        self.selected_index = Some(self.contacts.len() - 1);
        self.message = format!("Added: {name}");
        self.edit_name.clear();
        self.edit_email.clear();
        self.edit_phone.clear();
    }

    fn select(&mut self, idx: usize) {
        self.selected_index = Some(idx);
    }

    fn edit_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.contacts.len() {
                if !self.edit_name.is_empty() {
                    self.contacts[idx].name = self.edit_name.clone();
                }
                if !self.edit_email.is_empty() {
                    self.contacts[idx].email = self.edit_email.clone();
                }
                if !self.edit_phone.is_empty() {
                    self.contacts[idx].phone = self.edit_phone.clone();
                }
                self.message = format!("Updated: {}", self.contacts[idx].name);
            }
        }
    }

    fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.contacts.len() {
                let name = self.contacts[idx].name.clone();
                self.contacts.remove(idx);
                self.selected_index = None;
                self.message = format!("Deleted: {name}");
            }
        }
    }
}

// ============================================================================
// Message type
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum CrudMsg {
    Add,
    Edit,
    Delete,
    Clear,
}

// ============================================================================
// Layout constants
// ============================================================================

const LEFT: f64 = 20.0;
const BUTTON_Y: f64 = 155.0;
const LIST_TOP: f64 = 200.0;
const LIST_ROW_H: f64 = 28.0;

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — CRUD Contact Manager")
            .window_size(800.0, 600.0),
    );
    app.register_defaults();

    let state = Arc::new(Mutex::new(AppState::new()));

    // ── Interaction registrations ──
    let s_add = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(201),
        Rect::new(20.0, BUTTON_Y, 80.0, 32.0),
        "add",
        move |_| {
            s_add.lock().unwrap().add_contact();
        },
    );
    let s_edit = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(202),
        Rect::new(110.0, BUTTON_Y, 80.0, 32.0),
        "edit",
        move |_| {
            s_edit.lock().unwrap().edit_selected();
        },
    );
    let s_del = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(203),
        Rect::new(200.0, BUTTON_Y, 80.0, 32.0),
        "delete",
        move |_| {
            s_del.lock().unwrap().delete_selected();
        },
    );
    let s_clear = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(204),
        Rect::new(290.0, BUTTON_Y, 100.0, 32.0),
        "clear",
        move |_| {
            let mut guard = s_clear.lock().unwrap();
            guard.contacts.clear();
            guard.selected_index = None;
            guard.message = "All contacts cleared".into();
        },
    );

    // Row selection interactions (up to 12 rows)
    for i in 0..12_usize {
        let s_row = Arc::clone(&state);
        let y = LIST_TOP + i as f64 * LIST_ROW_H;
        app.register_interaction(
            WidgetId::from_u64(300 + i as u64),
            Rect::new(LEFT, y, 760.0, LIST_ROW_H),
            "select_row",
            move |_| {
                s_row.lock().unwrap().select(i);
            },
        );
    }

    // ── Scene builder: html! form + manual contact list ──
    let scene_state = Arc::clone(&state);
    let paint_fn = default_paint_fn::<CrudMsg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let w = width as f64;
            let _h = height as f64;
            let guard = scene_state.lock().unwrap();

            // --- Part 1: Form via html! declarative syntax ---
            let mut form_view: WidgetView<CrudMsg> = html! {
                <Column>
                    <Label text="Contact Manager" />
                    <Row>
                        <Column>
                            <Label text="Name:" />
                            <Label text="Email:" />
                            <Label text="Phone:" />
                        </Column>
                        <Column>
                            <TextField id="101" placeholder="Name" />
                            <TextField id="102" placeholder="Email" />
                            <TextField id="103" placeholder="Phone" />
                        </Column>
                    </Row>
                    <Row gap="8.0">
                        <Button id="201" label="Add" on:click={CrudMsg::Add} />
                        <Button id="202" label="Edit" on:click={CrudMsg::Edit} />
                        <Button id="203" label="Delete" on:click={CrudMsg::Delete} />
                        <Button id="204" label="Clear All" on:click={CrudMsg::Clear} />
                    </Row>
                </Column>
            };

            let form_layout = compute_view_layout(&mut form_view, Size::new(w, 600.0));
            let mut scene =
                build_scene_from_view(&form_view, &form_layout, &paint_fn, frame, Some(tr));

            // --- Part 2: Dynamic contact list via manual PaintLayerData ---
            let mut layers: Vec<PaintLayerData> = Vec::new();

            // Message line
            let msg_bounds = Rect::new(LEFT, 188.0, 760.0, 20.0);
            let mut msg_ctx = PaintContext::new(msg_bounds);
            msg_ctx.draw_text(
                &guard.message,
                msg_bounds,
                Color::WHITE.with_alpha(0.7),
                13.0,
            );
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(500),
                0,
                msg_bounds,
                msg_ctx.into_operations(),
            ));

            // Contact list rows
            for i in 0..12_usize {
                let y = LIST_TOP + i as f64 * LIST_ROW_H;
                let row_bounds = Rect::new(LEFT, y, 760.0, LIST_ROW_H);
                if i < guard.contacts.len() {
                    let c = &guard.contacts[i];
                    let is_selected = guard.selected_index == Some(i);
                    let mut row_ctx = PaintContext::new(row_bounds);
                    if is_selected {
                        row_ctx.fill_rect(row_bounds, Color::new(0.15, 0.20, 0.35, 0.6), 4.0);
                    }
                    let info = format!("{}. {} | {} | {}", i + 1, c.name, c.email, c.phone);
                    row_ctx.draw_text(
                        &info,
                        Rect::new(4.0, 2.0, 752.0, 24.0),
                        Color::WHITE.with_alpha(0.9),
                        14.0,
                    );
                    layers.push(PaintLayerData::new(
                        WidgetId::from_u64(300 + i as u64),
                        if is_selected { 2 } else { 0 },
                        row_bounds,
                        row_ctx.into_operations(),
                    ));
                }
            }

            drop(guard);

            // Merge manual layers into the html! scene
            let list_scene = rgui::build_scene_from_paint_data(&layers, frame, Some(tr));
            scene.layers.extend(list_scene.layers);
            scene
        },
    );

    println!("\n=== rgui CRUD Example (html! 声明式渲染) ===");
    println!("Contact manager app started.");
    println!("Form rendered via html! + build_scene_from_view.");
    println!("Dynamic contact list rendered via PaintLayerData.");
    println!();
    println!("Instructions:");
    println!("  1. Click [Add] -> Create new contact");
    println!("  2. Click a row in the list -> Select");
    println!("  3. After selecting, click [Edit] -> Update contact name");
    println!("  4. After selecting, click [Delete] -> Remove contact");
    println!("  5. Click [Clear All] -> Clear all contacts");
    println!();

    app.run()
}
