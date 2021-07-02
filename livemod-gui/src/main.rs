use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::mpsc::{self, Sender};

use glium::glutin;
use livemod::{TrackedDataRepr, TrackedDataValue};
use nanoserde::{DeBin, SerBin};

fn create_display(event_loop: &glutin::event_loop::EventLoop<()>) -> glium::Display {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 300.0,
            height: 600.0,
        })
        .with_title("Livemod Variables");

    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, &event_loop).unwrap()
}

fn main() {
    let (sender, recv) = mpsc::channel();
    std::thread::spawn(|| reader_thread(sender));

    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(&&event_loop);

    let mut egui = egui_glium::EguiGlium::new(&display);

    let mut cached_shapes = None;
    let mut current_variables = Vec::new();
    let mut values = Values::default();
    let mut modified_variables = Vec::new();

    event_loop.run(move |event, _, control_flow| match event {
        glutin::event::Event::MainEventsCleared => {
            while let Ok(msg) = recv.try_recv() {
                fn recursive_namespaced_insert(
                    namespaced_name: String,
                    value: TrackedDataValue,
                    values: &mut Values,
                ) {
                    match value {
                        TrackedDataValue::SignedInt(i) => {
                            values.i64.insert(namespaced_name, i);
                        }
                        TrackedDataValue::UnsignedInt(u) => {
                            values.u64.insert(namespaced_name, u);
                        }
                        TrackedDataValue::Float(f) => {
                            values.f64.insert(namespaced_name, f);
                        }
                        TrackedDataValue::Bool(b) => {
                            values.bool.insert(namespaced_name, b);
                        }
                        TrackedDataValue::String(s) => {
                            values.str.insert(namespaced_name, s);
                        }
                        TrackedDataValue::Struct(fields) => {
                            for (field_name, field_value) in fields {
                                let name = format!("{}:{}", namespaced_name, field_name);
                                recursive_namespaced_insert(name, field_value, values);
                            }
                        }
                        TrackedDataValue::Trigger => {},
                    }
                }

                match msg {
                    Message::NewData(name, data, initial_value) => {
                        recursive_namespaced_insert(
                            format!(":{}", &name),
                            initial_value,
                            &mut values,
                        );
                        current_variables.push((name, data));
                    }
                    Message::UpdateData(name, value) => {
                        recursive_namespaced_insert(format!(":{}", name), value, &mut values);
                    }
                    Message::RemoveData(name) => {
                        current_variables.remove(current_variables.iter().position(|(n, _)| name == *n).unwrap());
                    }
                }
            }

            egui.begin_frame(&display);

            egui::CentralPanel::default().show(&egui.ctx(), |ui| {
                egui::Grid::new("base_grid")
                    .striped(true)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        recursive_ui(
                            ui,
                            &mut values,
                            &mut modified_variables,
                            current_variables.iter().map(|(s, v)| (s.clone(), v)),
                            String::new(),
                        )
                    });
            });

            for (name, value) in modified_variables.drain(..) {
                if let TrackedDataValue::Trigger = value {
                    println!(
                        "t{}",
                        name,
                    )
                } else {
                    println!(
                        "s{};{}",
                        name,
                        base64::encode_config(value.serialize_bin(), base64::STANDARD_NO_PAD)
                    );
                }
            }

            let (needs_repaint, shapes) = egui.end_frame(&display);

            *control_flow = if needs_repaint {
                display.gl_window().window().request_redraw();
                cached_shapes = Some(shapes);
                glutin::event_loop::ControlFlow::Poll
            } else {
                glutin::event_loop::ControlFlow::Wait
            };
        }
        glutin::event::Event::RedrawRequested(_) => {
            if let Some(shapes) = cached_shapes.take() {
                use glium::Surface as _;
                let mut target = display.draw();

                let clear_color = egui::Rgba::from_rgb(0.1, 0.1, 0.1);
                target.clear_color(
                    clear_color[0],
                    clear_color[1],
                    clear_color[2],
                    clear_color[3],
                );

                egui.paint(&display, &mut target, shapes);

                target.finish().unwrap();
            }
        }
        glutin::event::Event::WindowEvent { event, .. } => {
            egui.on_event(event, control_flow);
            display.gl_window().window().request_redraw();
        }

        _ => (),
    });
}

#[derive(Default, Debug)]
struct Values {
    i64: HashMap<String, i64>,
    u64: HashMap<String, u64>,
    f64: HashMap<String, f64>,
    bool: HashMap<String, bool>,
    str: HashMap<String, String>,
}

enum Message {
    NewData(String, TrackedDataRepr, TrackedDataValue),
    UpdateData(String, TrackedDataValue),
    RemoveData(String),
}

fn recursive_ui<'a>(
    ui: &mut egui::Ui,
    values: &mut Values,
    modified_variables: &mut Vec<(String, TrackedDataValue)>,
    variables: impl Iterator<Item = (String, &'a TrackedDataRepr)>,
    namespace: String,
) {
    for (name, var) in variables {
        ui.label(name.clone());
        let namespaced_name = format!("{}:{}", namespace, name);
        match &var {
            livemod::TrackedDataRepr::Struct { name, fields } => {
                ui.collapsing(name, |ui| {
                    egui::Grid::new(format!("{}_grid", &namespaced_name))
                        .striped(true)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            recursive_ui(
                                ui,
                                values,
                                modified_variables,
                                fields
                                    .iter()
                                    .map(|field| (field.name.clone(), &field.data_type))
                                    .collect::<Vec<_>>()
                                    .into_iter(),
                                namespaced_name,
                            )
                        });
                });
            }
            livemod::TrackedDataRepr::SignedSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => {
                ui.add(
                    egui::Slider::from_get_set(
                        (*suggested_min) as f64..=(*suggested_max) as f64,
                        |val| *match val {
                            Some(val) => {
                                values.i64.insert(
                                    namespaced_name.clone(),
                                    (val as i64).clamp(*storage_min, *storage_max),
                                );
                                let v = values.i64.get(&namespaced_name).unwrap();
                                modified_variables.push((
                                    namespaced_name.clone(),
                                    livemod::TrackedDataValue::SignedInt(*v),
                                ));
                                v
                            }
                            None => values.i64.entry(namespaced_name.clone()).or_default(),
                        } as f64,
                    )
                    .integer(),
                );
            }
            livemod::TrackedDataRepr::UnsignedSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => {
                ui.add(
                    egui::Slider::from_get_set(
                        (*suggested_min) as f64..=(*suggested_max) as f64,
                        |val| *match val {
                            Some(val) => {
                                values.u64.insert(
                                    namespaced_name.clone(),
                                    (val as u64).clamp(*storage_min, *storage_max),
                                );
                                let v = values.u64.get(&namespaced_name).unwrap();
                                modified_variables.push((
                                    namespaced_name.clone(),
                                    livemod::TrackedDataValue::UnsignedInt(*v),
                                ));
                                v
                            }
                            None => values.u64.entry(namespaced_name.clone()).or_default(),
                        } as f64,
                    )
                    .integer(),
                );
            }
            TrackedDataRepr::FloatSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => {
                ui.add(egui::Slider::from_get_set(
                    (*suggested_min) as f64..=(*suggested_max) as f64,
                    |val| *match val {
                        Some(val) => {
                            values.f64.insert(
                                namespaced_name.clone(),
                                val.clamp(*storage_min, *storage_max),
                            );
                            let v = values.f64.get(&namespaced_name).unwrap();
                            modified_variables.push((
                                namespaced_name.clone(),
                                livemod::TrackedDataValue::Float(*v),
                            ));
                            v
                        }
                        None => values.f64.entry(namespaced_name.clone()).or_default(),
                    },
                ));
            }
            TrackedDataRepr::SignedInteger { min, max } => {
                if ui
                    .add(
                        egui::DragValue::new(
                            values.i64.entry(namespaced_name.clone()).or_default(),
                        )
                        .clamp_range(*min..=*max),
                    )
                    .changed()
                {
                    modified_variables.push((
                        namespaced_name.clone(),
                        livemod::TrackedDataValue::SignedInt(
                            *values.i64.entry(namespaced_name.clone()).or_default(),
                        ),
                    ));
                }
            }
            TrackedDataRepr::UnsignedInteger { min, max } => {
                if ui
                    .add(
                        egui::DragValue::new(
                            values.u64.entry(namespaced_name.clone()).or_default(),
                        )
                        .clamp_range(*min..=*max),
                    )
                    .changed()
                {
                    modified_variables.push((
                        namespaced_name.clone(),
                        livemod::TrackedDataValue::UnsignedInt(
                            *values.u64.entry(namespaced_name.clone()).or_default(),
                        ),
                    ));
                }
            }
            TrackedDataRepr::Float { min, max } => {
                if ui
                    .add(
                        egui::DragValue::new(
                            values.f64.entry(namespaced_name.clone()).or_default(),
                        )
                        .clamp_range(*min..=*max),
                    )
                    .changed()
                {
                    modified_variables.push((
                        namespaced_name.clone(),
                        livemod::TrackedDataValue::Float(
                            *values.f64.entry(namespaced_name.clone()).or_default(),
                        ),
                    ));
                }
            }
            TrackedDataRepr::Bool => {
                if ui
                    .checkbox(values.bool.entry(namespaced_name.clone()).or_default(), "")
                    .changed()
                {
                    modified_variables.push((
                        namespaced_name.clone(),
                        livemod::TrackedDataValue::Bool(
                            *values.bool.entry(namespaced_name.clone()).or_default(),
                        ),
                    ));
                }
            }
            TrackedDataRepr::Trigger { name } => {
                if ui.button(&name).clicked() {
                    modified_variables
                        .push((format!("{}:{}", namespaced_name.clone(), name), livemod::TrackedDataValue::Trigger));
                }
            }
            TrackedDataRepr::String { multiline } => {
                if if *multiline {
                    ui.text_edit_multiline(values.str.entry(namespaced_name.clone()).or_default())
                } else {
                    ui.text_edit_singleline(values.str.entry(namespaced_name.clone()).or_default())
                }
                .changed()
                {
                    modified_variables.push((
                        namespaced_name.clone(),
                        livemod::TrackedDataValue::String(
                            values
                                .str
                                .entry(namespaced_name.clone())
                                .or_default()
                                .clone(),
                        ),
                    ));
                }
            }
        }
        ui.end_row();
    }
}

fn reader_thread(sender: Sender<Message>) {
    for line in BufReader::new(std::io::stdin()).lines() {
        let line = line.unwrap();
        let line = line.as_bytes();
        let segments = line[1..].split(|c| *c == b';').collect::<Vec<_>>();
        match line[0] {
            b'\0' => break,
            b'n' => {
                // New variable to track
                sender
                    .send(Message::NewData(
                        String::from_utf8(segments[0].to_owned()).unwrap(),
                        TrackedDataRepr::deserialize_bin(
                            &base64::decode_config(&segments[1], base64::STANDARD_NO_PAD).unwrap(),
                        )
                        .unwrap(),
                        TrackedDataValue::deserialize_bin(
                            &base64::decode_config(&segments[2], base64::STANDARD_NO_PAD).unwrap(),
                        )
                        .unwrap(),
                    ))
                    .unwrap();
            }
            b'u' => {
                // Variable was updated
                sender
                    .send(Message::UpdateData(
                        String::from_utf8(segments[0].to_owned()).unwrap(),
                        TrackedDataValue::deserialize_bin(
                            &base64::decode_config(&segments[1], base64::STANDARD_NO_PAD).unwrap(),
                        )
                        .unwrap(),
                    ))
                    .unwrap();
            }
            b'r' => {
                // Variable was dropped
                sender.send(Message::RemoveData(String::from_utf8(segments[0].to_owned()).unwrap())).unwrap()
            }
            _ => {
                debug_assert!(false, "Unexpected input")
            }
        }
    }
}
