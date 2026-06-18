//! # rgui CRUD Example - Contact Manager using `html!` declarative syntax
//!
//! Demonstrates CRUD operations with rgui framework using html! macro.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p crud
//! ```

use rgui::app::{App, AppConfig};
use rgui::prelude::*;
use rgui::{
    AppMessage, Button, ButtonState, Color, Label, LabelState, PaintContext, PaintLayerData, Rect,
    WidgetId, WidgetView, html,
};
use std::sync::{Arc, Mutex};

// ============================================================================
// Step 1: Define data structures
// ============================================================================

/// A contact.
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
// Step 2: Define global shared state
// ============================================================================

/// Application global state.
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

    fn contact_count(&self) -> usize {
        self.contacts.len()
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
                self.edit_name.clear();
                self.edit_email.clear();
                self.edit_phone.clear();
            }
        } else {
            self.message = "Please select a contact first".into();
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
        } else {
            self.message = "Please select a contact first".into();
        }
    }

    fn select(&mut self, index: usize) {
        if index < self.contacts.len() {
            self.selected_index = Some(index);
            let c = &self.contacts[index];
            self.edit_name = c.name.clone();
            self.edit_email = c.email.clone();
            self.edit_phone = c.phone.clone();
            self.message = format!("Selected: {}", c.name);
        }
    }
}

// ============================================================================
// Step 2b: Message type (for html! declarative syntax)
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum CrudMsg {
    Add,
    Edit,
    Delete,
    Clear,
    SelectRow(usize),
}

// ============================================================================
// Step 3: UI layout constants
// ============================================================================

const INPUT_Y: f64 = 60.0;
const BUTTON_Y: f64 = 120.0;
const LIST_TOP: f64 = 170.0;
const LIST_ROW_H: f64 = 28.0;
const LEFT: f64 = 20.0;
const LABEL_W: f64 = 40.0;
const INPUT_W: f64 = 160.0;

// ============================================================================
// Step 4: Build UI and run
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui CRUD Example - Contact Manager (html!)")
            .window_size(820.0, 600.0),
    );
    app.register_defaults();

    let state = Arc::new(Mutex::new(AppState::new()));

    // --- Register interactions ---

    let s1 = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(101),
        Rect::new(LEFT + LABEL_W, INPUT_Y, INPUT_W, 28.0),
        "edit_name",
        move |_| {
            let _s = s1.lock().unwrap();
            println!("[Action] Clicked name input");
        },
    );

    let s2 = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(102),
        Rect::new(LEFT + LABEL_W, INPUT_Y + 32.0, INPUT_W, 28.0),
        "edit_email",
        move |_| {
            let _s = s2.lock().unwrap();
            println!("[Action] Clicked email input");
        },
    );

    let s3 = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(103),
        Rect::new(LEFT + LABEL_W, INPUT_Y + 64.0, INPUT_W, 28.0),
        "edit_phone",
        move |_| {
            let _s = s3.lock().unwrap();
            println!("[Action] Clicked phone input");
        },
    );

    let s_add = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(201),
        Rect::new(20.0, BUTTON_Y, 80.0, 32.0),
        "add",
        move |_| {
            let mut guard = s_add.lock().unwrap_or_else(|e| e.into_inner());
            guard.add_contact();
            println!("Contact count: {}", guard.contact_count());
        },
    );

    let s_edit = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(202),
        Rect::new(110.0, BUTTON_Y, 80.0, 32.0),
        "edit",
        move |_| {
            let mut guard = s_edit.lock().unwrap_or_else(|e| e.into_inner());
            guard.edit_selected();
        },
    );

    let s_del = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(203),
        Rect::new(200.0, BUTTON_Y, 80.0, 32.0),
        "delete",
        move |_| {
            let mut guard = s_del.lock().unwrap_or_else(|e| e.into_inner());
            guard.delete_selected();
        },
    );

    let s_clear = Arc::clone(&state);
    app.register_interaction(
        WidgetId::from_u64(204),
        Rect::new(290.0, BUTTON_Y, 100.0, 32.0),
        "clear",
        move |_| {
            let mut guard = s_clear.lock().unwrap_or_else(|e| e.into_inner());
            guard.contacts.clear();
            guard.selected_index = None;
            guard.message = "All contacts cleared".into();
        },
    );

    for i in 0..12_usize {
        let s_row = Arc::clone(&state);
        let y = LIST_TOP + i as f64 * LIST_ROW_H;
        app.register_interaction(
            WidgetId::from_u64(300 + i as u64),
            Rect::new(LEFT, y, 780.0, LIST_ROW_H),
            "select_row",
            move |_| {
                let mut guard = s_row.lock().unwrap_or_else(|e| e.into_inner());
                if i < guard.contacts.len() {
                    guard.select(i);
                }
            },
        );
    }

    // --- Scene builder with html! declarative syntax ---

    let scene_state = Arc::clone(&state);
    app.set_scene_builder(move |_frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;
        let guard = scene_state.lock().unwrap();
        let mut layers: Vec<PaintLayerData> = Vec::new();

        // html! declarative UI definition (demonstrates the syntax)
        let _view: WidgetView<CrudMsg> = html! {
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

        // --- Background ---
        let mut bg_ctx = PaintContext::new(Rect::new(0.0, 0.0, w, h));
        bg_ctx.fill_rect(
            Rect::new(0.0, 0.0, w, h),
            Color::new(14.0 / 255.0, 18.0 / 255.0, 28.0 / 255.0, 1.0),
            0.0,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(0),
            -1,
            Rect::new(0.0, 0.0, w, h),
            bg_ctx.into_operations(),
        ));

        // --- Title ---
        let title_bounds = Rect::new(20.0, 10.0, w - 40.0, 30.0);
        let mut title_ctx = PaintContext::new(title_bounds);
        Label.paint(
            &LabelState {
                text: "Contact Manager".into(),
            },
            title_bounds,
            &mut title_ctx,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(900),
            0,
            title_bounds,
            title_ctx.into_operations(),
        ));

        // --- Input labels ---
        let input_labels = ["Name:", "Email:", "Phone:"];
        let input_values = [&guard.edit_name, &guard.edit_email, &guard.edit_phone];
        for (i, lbl) in input_labels.iter().enumerate() {
            let y = INPUT_Y + i as f64 * 32.0;
            let lb = Rect::new(LEFT, y, LABEL_W, 24.0);
            let mut lc = PaintContext::new(lb);
            Label.paint(
                &LabelState {
                    text: lbl.to_string(),
                },
                lb,
                &mut lc,
            );
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(910 + i as u64),
                0,
                lb,
                lc.into_operations(),
            ));

            let vb = Rect::new(LEFT + LABEL_W, y, INPUT_W, 24.0);
            let mut vc = PaintContext::new(vb);
            let val = input_values[i];
            let display = if val.is_empty() {
                "(click to input)"
            } else {
                val
            };
            Label.paint(
                &LabelState {
                    text: display.to_string(),
                },
                vb,
                &mut vc,
            );
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(920 + i as u64),
                0,
                vb,
                vc.into_operations(),
            ));
        }

        // --- Button row ---
        let buttons = [
            (201_u64, "Add", 20.0),
            (202_u64, "Edit", 110.0),
            (203_u64, "Delete", 200.0),
            (204_u64, "Clear All", 290.0),
        ];
        for (id, label, x) in &buttons {
            let bb = Rect::new(*x, BUTTON_Y, 80.0, 32.0);
            let mut bc = PaintContext::new(bb);
            Button.paint(&ButtonState::new(label.to_string()), bb, &mut bc);
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(*id),
                1,
                bb,
                bc.into_operations(),
            ));
        }

        // --- Contact list (up to 12 rows) ---
        for i in 0..12_usize {
            let y = LIST_TOP + i as f64 * LIST_ROW_H;
            let row_bounds = Rect::new(LEFT, y, 780.0, LIST_ROW_H);

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
                    Rect::new(4.0, 2.0, 760.0, 24.0),
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

        // --- Status message ---
        let msg_bounds = Rect::new(20.0, h - 40.0, w - 40.0, 24.0);
        let mut msg_ctx = PaintContext::new(msg_bounds);
        msg_ctx.draw_text(
            &guard.message,
            msg_bounds,
            Color::WHITE.with_alpha(0.6),
            12.0,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(999),
            5,
            msg_bounds,
            msg_ctx.into_operations(),
        ));

        drop(guard);
        layers
    });

    // Run the app
    println!("\n=== rgui CRUD Example (html! syntax) ===");
    println!("Contact manager app started. UI declared with html! macro.");
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
