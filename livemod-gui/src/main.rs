use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::{self, Sender};

use glium::glutin;
use hashlink::LinkedHashMap;
use livemod::{Namespaced, Parameter, Repr, Value};

#[derive(Default)]
struct State {
    tracked_vars: LinkedHashMap<String, Namespaced<Repr>>,
    tracked_data: HashMap<String, TrackedParameter>,
    modified_data: Vec<String>,
}

enum TrackedParameter {
    SignedInt(i64),
    UnsignedInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Namespaced {
        name: Vec<String>,
        params: HashSet<String>,
    },
}

impl TrackedParameter {
    fn serialize(&self) -> String {
        match self {
            TrackedParameter::SignedInt(i) => format!("{:+}", i),
            TrackedParameter::UnsignedInt(i) => format!("{}", i),
            TrackedParameter::Float(f) => format!("d{}", f),
            TrackedParameter::Bool(true) => "t".to_owned(),
            TrackedParameter::Bool(false) => "f".to_owned(),
            TrackedParameter::String(s) => format!("\"{}\"", s),
            TrackedParameter::Namespaced { name, params } => {
                if params.is_empty() {
                    format!("{}{{}}", name.join(":"))
                } else {
                    todo!()
                }
            }
        }
    }

    fn as_signed_int(&self) -> Option<&i64> {
        if let Self::SignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_signed_int_mut(&mut self) -> Option<&mut i64> {
        if let Self::SignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_unsigned_int(&self) -> Option<&u64> {
        if let Self::UnsignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_unsigned_int_mut(&mut self) -> Option<&mut u64> {
        if let Self::UnsignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_float(&self) -> Option<&f64> {
        if let Self::Float(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_float_mut(&mut self) -> Option<&mut f64> {
        if let Self::Float(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_bool(&self) -> Option<&bool> {
        if let Self::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_bool_mut(&mut self) -> Option<&mut bool> {
        if let Self::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_string_mut(&mut self) -> Option<&mut String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

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

    glium::Display::new(window_builder, context_builder, event_loop).unwrap()
}

fn main() {
    let (sender, recv) = mpsc::channel();
    std::thread::spawn(|| reader_thread(sender));

    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(&event_loop);

    let mut egui = egui_glium::EguiGlium::new(&display);

    let mut cached_shapes = None;
    let mut state = State::default();
    let mut quit = false;

    event_loop.run(move |event, _, control_flow| match event {
        glutin::event::Event::MainEventsCleared => {
            while let Ok(msg) = recv.try_recv() {
                fn recursive_insert(name: String, param: Parameter<Value>, state: &mut State) {
                    let parameter = match param {
                        Parameter::SignedInt(value) => TrackedParameter::SignedInt(value),
                        Parameter::UnsignedInt(value) => TrackedParameter::UnsignedInt(value),
                        Parameter::Float(value) => TrackedParameter::Float(value),
                        Parameter::Bool(value) => TrackedParameter::Bool(value),
                        Parameter::String(value) => TrackedParameter::String(value),
                        Parameter::Namespaced(namespaced) => TrackedParameter::Namespaced {
                            name: namespaced.name,
                            params: namespaced
                                .parameters
                                .into_iter()
                                .map(|(k, v)| {
                                    recursive_insert(format!("{}.{}", name, k), v, state);
                                    k
                                })
                                .collect(),
                        },
                    };
                    state.tracked_data.insert(name, parameter);
                }

                match msg {
                    Message::NewData(name, data, initial_value) => {
                        recursive_insert(format!(".{}", name), initial_value, &mut state);
                        state.tracked_vars.insert(name, data);
                    }
                    Message::UpdateRepr(name, data, value) => {
                        recursive_insert(format!(".{}", name), value, &mut state);
                        *state.tracked_vars.get_mut(&name).unwrap() = data;
                    }
                    Message::UpdateData(name, value) => {
                        recursive_insert(name, value, &mut state);
                    }
                    Message::RemoveData(name) => {
                        state.tracked_vars.remove(&name);
                    }
                    Message::Quit => {
                        quit = true;
                    }
                }
            }

            egui.begin_frame(&display);

            egui::CentralPanel::default().show(egui.ctx(), |ui| {
                egui::Grid::new("base_grid")
                    .striped(true)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        draw_repr(
                            ui,
                            &Namespaced::new(
                                vec!["livemod".to_owned(), "fields".to_owned()],
                                //TODO: Optimize this
                                state
                                    .tracked_vars
                                    .iter()
                                    .map(|(k, v)| (k.to_owned(), Parameter::Namespaced(v.clone())))
                                    .collect(),
                            ),
                            "".to_owned(),
                            &mut state,
                        )
                    });
            });

            for name in state.modified_data.drain(..) {
                let value = state.tracked_data[&name].serialize();
                println!("s{};{}-{}", &name[1..], value.as_bytes().len(), value);
            }

            let (needs_repaint, shapes) = egui.end_frame(&display);

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if needs_repaint {
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
            egui.on_event(&event);
            display.gl_window().window().request_redraw();
        }
        _ => (),
    });
}

/// Dispatch and draw the given `repr` to the given `ui`.
///
/// # Parameters
/// * `ui`: The `ui` to draw to.
/// * `repr`: The `repr` to draw.
/// * `namespace`: The namespace or name to store data under.
/// * `state`: The currently stored data.
fn draw_repr(ui: &mut egui::Ui, repr: &Namespaced<Repr>, namespace: String, state: &mut State) {
    if repr.name[0] == "livemod" {
        match repr.name[1].as_str() {
            "fields" => {
                for (name, field) in &repr.parameters {
                    let field_namespace = format!("{}.{}", namespace, name);
                    let field = field.as_namespaced().unwrap();
                    ui.label(name);
                    draw_repr(ui, field, field_namespace, state);
                    ui.end_row();
                }
            }
            "struct" => {
                ui.collapsing(repr.parameters["name"].as_string().unwrap(), |ui| {
                    egui::Grid::new(&namespace)
                        .striped(true)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            draw_repr(
                                ui,
                                repr.parameters["fields"].as_namespaced().unwrap(),
                                namespace,
                                state,
                            );
                        });
                });
            }
            "vec" => {
                ui.collapsing("Vec", |ui| {
                    egui::Grid::new(&namespace)
                        .striped(true)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Length");
                            let len_field = format!("{}.len", namespace);
                            let mut len = state.tracked_data.entry(len_field.clone()).or_insert(
                                TrackedParameter::UnsignedInt(
                                    repr.parameters["len"].as_unsigned_int().copied().unwrap(),
                                ),
                            );
                            ui.add(
                                egui::DragValue::new(len.as_unsigned_int_mut().unwrap()).speed(0.1),
                            )
                            .changed()
                            .then(|| {
                                state.modified_data.push(len_field);
                            });
                            ui.end_row();
                            for (i, field) in &repr.parameters {
                                let i = match i.parse::<usize>() {
                                    Ok(i) => i,
                                    Err(_) => continue,
                                };
                                let field_namespace = format!("{}.{}", namespace, i);
                                let field = field.as_namespaced().unwrap();
                                ui.label(format!("{}", i));
                                draw_repr(ui, field, field_namespace, state);
                                //TODO: Add remove button, insert button, etc.
                                ui.end_row();
                            }
                        });
                });
            }
            "bool" => {
                ui.checkbox(
                    state
                        .tracked_data
                        .entry(namespace.clone())
                        .or_insert(TrackedParameter::Bool(false))
                        .as_bool_mut()
                        .unwrap(),
                    "",
                )
                .changed()
                .then(|| state.modified_data.push(namespace));
            }
            "trigger" => {
                ui.button(
                    repr.parameters
                        .get("name")
                        .and_then(|param| param.as_string().map(|s| s.as_str()))
                        .unwrap_or("Call"),
                )
                .clicked()
                .then(|| {
                    state.tracked_data.insert(
                        namespace.clone(),
                        TrackedParameter::Namespaced {
                            name: vec!["livemod".to_owned(), "trigger".to_owned()],
                            params: HashSet::new(),
                        },
                    );
                    state.modified_data.push(namespace);
                });
            }
            "string" => {
                if repr
                    .parameters
                    .get("multiline")
                    .and_then(|p| p.as_bool().cloned())
                    .unwrap_or(false)
                {
                    ui.text_edit_multiline(
                        state
                            .tracked_data
                            .entry(namespace.clone())
                            .or_insert(TrackedParameter::String("".to_owned()))
                            .as_string_mut()
                            .unwrap(),
                    )
                } else {
                    ui.text_edit_singleline(
                        state
                            .tracked_data
                            .entry(namespace.clone())
                            .or_insert(TrackedParameter::String("".to_owned()))
                            .as_string_mut()
                            .unwrap(),
                    )
                }
                .changed()
                .then(|| state.modified_data.push(namespace));
            }
            "sint" => {
                let min = repr.parameters["min"].as_signed_int().copied().unwrap();
                let max = repr.parameters["max"].as_signed_int().copied().unwrap();
                let suggested_min = repr
                    .parameters
                    .get("suggested_min")
                    .and_then(|p| p.as_signed_int().copied());
                let suggested_max = repr
                    .parameters
                    .get("suggested_max")
                    .and_then(|p| p.as_signed_int().copied());
                if let (Some(suggested_min), Some(suggested_max)) = (suggested_min, suggested_max) {
                    ui.add(
                        egui::Slider::from_get_set(
                            suggested_min as f64..=suggested_max as f64,
                            |val| match val {
                                Some(val) => {
                                    //TODO: Clamp *before* casting?
                                    let new_val = (val as i64).clamp(min, max);
                                    state.tracked_data.insert(
                                        namespace.clone(),
                                        TrackedParameter::SignedInt(new_val),
                                    );
                                    new_val as f64
                                }
                                None => state.tracked_data.entry(namespace.clone()).or_insert(TrackedParameter::SignedInt(0)).as_signed_int().copied().unwrap() as f64,
                            } as f64,
                        )
                        .integer()
                    )
                } else {
                    ui.add(
                        egui::DragValue::new(
                            state.tracked_data.entry(namespace.clone()).or_insert(TrackedParameter::SignedInt(0)).as_signed_int_mut().unwrap(),
                        )
                        .clamp_range(min..=max),
                    )
                }.changed().then(|| state.modified_data.push(namespace));
            }
            "uint" => {
                let min = repr.parameters["min"].as_unsigned_int().copied().unwrap();
                let max = repr.parameters["max"].as_unsigned_int().copied().unwrap();
                let suggested_min = repr
                    .parameters
                    .get("suggested_min")
                    .and_then(|p| p.as_unsigned_int().copied());
                let suggested_max = repr
                    .parameters
                    .get("suggested_max")
                    .and_then(|p| p.as_unsigned_int().copied());
                if let (Some(suggested_min), Some(suggested_max)) = (suggested_min, suggested_max) {
                    ui.add(
                        egui::Slider::from_get_set(
                            suggested_min as f64..=suggested_max as f64,
                            |val| match val {
                                Some(val) => {
                                    let new_val = (val as u64).clamp(min, max);
                                    state.tracked_data.insert(
                                        namespace.clone(),
                                        TrackedParameter::UnsignedInt(new_val),
                                    );
                                    new_val as f64
                                }
                                None => state.tracked_data.entry(namespace.clone()).or_insert(TrackedParameter::UnsignedInt(0)).as_unsigned_int().copied().unwrap() as f64,
                            },
                        )
                        .integer()
                    )
                } else {
                    ui.add(
                        egui::DragValue::new(
                            state.tracked_data.entry(namespace.clone()).or_insert(TrackedParameter::UnsignedInt(0)).as_unsigned_int_mut().unwrap(),
                        )
                        .clamp_range(min..=max),
                    )
                }.changed().then(|| state.modified_data.push(namespace));
            }
            "float" => {
                let min = repr.parameters["min"].as_float().copied().unwrap();
                let max = repr.parameters["max"].as_float().copied().unwrap();
                let suggested_min = repr
                    .parameters
                    .get("suggested_min")
                    .and_then(|p| p.as_float().copied());
                let suggested_max = repr
                    .parameters
                    .get("suggested_max")
                    .and_then(|p| p.as_float().copied());
                if let (Some(suggested_min), Some(suggested_max)) = (suggested_min, suggested_max) {
                    ui.add(
                        egui::Slider::from_get_set(
                            suggested_min..=suggested_max,
                            |val| match val {
                                Some(val) => {
                                    let new_val = val.clamp(min, max);
                                    state.tracked_data.insert(
                                        namespace.clone(),
                                        TrackedParameter::Float(new_val),
                                    );
                                    new_val
                                }
                                None => state.tracked_data.entry(namespace.clone()).or_insert(TrackedParameter::Float(0.0)).as_float().copied().unwrap(),
                            },
                        )
                    )
                } else {
                    ui.add(
                        egui::DragValue::new(
                            state.tracked_data.entry(namespace.clone()).or_insert(TrackedParameter::Float(0.0)).as_float_mut().unwrap(),
                        )
                        .clamp_range(min..=max),
                    )
                }.changed().then(|| state.modified_data.push(namespace));
            }
            name => panic!("Unknown livemod builtin: {}", name),
        }
    }
}

enum Message {
    NewData(String, Namespaced<Repr>, Parameter<Value>),
    UpdateRepr(String, Namespaced<Repr>, Parameter<Value>),
    UpdateData(String, Parameter<Value>),
    RemoveData(String),
    Quit,
}

fn reader_thread(sender: Sender<Message>) {
    #[cfg(feature = "io_tee")]
    use io_tee::ReadExt;

    let stream = std::io::stdin();
    #[cfg(not(feature = "io_tee"))]
    let mut reader = BufReader::new(stream.lock());
    #[cfg(feature = "io_tee")]
    let mut reader = BufReader::new(stream.lock()).tee_dbg();

    loop {
        let message_type = {
            let mut message_type = [0u8];
            reader.read_exact(&mut message_type).unwrap();
            message_type[0]
        };

        match message_type {
            b'\0' => {
                sender.send(Message::Quit).unwrap();
                break; // And exit the thread
            }
            b'n' => {
                let name = {
                    let mut name = Vec::new();
                    reader.read_until(b';', &mut name).unwrap();
                    name.pop(); // Pop delimiter
                    String::from_utf8(name).unwrap()
                };

                let len_repr = {
                    let mut len = Vec::new();
                    reader.read_until(b'-', &mut len).unwrap();
                    len.pop(); // Pop delimiter
                    String::from_utf8(len).unwrap().parse::<usize>().unwrap()
                };
                let repr = {
                    let mut repr = vec![0u8; len_repr];
                    reader.read_exact(&mut repr).unwrap();
                    Namespaced::deserialize(std::str::from_utf8(&repr).unwrap()).unwrap()
                };
                reader.fill_buf().unwrap();
                reader.consume(1); // Consume ';' delimiter

                let len_value = {
                    let mut len = Vec::new();
                    reader.read_until(b'-', &mut len).unwrap();
                    len.pop(); // Pop delimiter
                    String::from_utf8(len).unwrap().parse::<usize>().unwrap()
                };
                let value = {
                    let mut value = vec![0u8; len_value];
                    reader.read_exact(&mut value).unwrap();
                    Parameter::deserialize(std::str::from_utf8(&value).unwrap()).unwrap()
                };
                sender.send(Message::NewData(name, repr, value)).unwrap();
            }
            b's' => {
                let name = {
                    let mut name = Vec::new();
                    reader.read_until(b';', &mut name).unwrap();
                    name.pop(); // Pop delimiter
                    String::from_utf8(name).unwrap()
                };

                let len_value = {
                    let mut len = Vec::new();
                    reader.read_until(b'-', &mut len).unwrap();
                    len.pop(); // Pop delimiter
                    String::from_utf8(len).unwrap().parse::<usize>().unwrap()
                };
                let value = {
                    let mut value = vec![0u8; len_value];
                    reader.read_exact(&mut value).unwrap();
                    Parameter::deserialize(std::str::from_utf8(&value).unwrap()).unwrap()
                };
                sender.send(Message::UpdateData(name, value)).unwrap();
            }
            b'u' => {
                let name = {
                    let mut name = Vec::new();
                    reader.read_until(b';', &mut name).unwrap();
                    name.pop(); // Pop delimiter
                    String::from_utf8(name).unwrap()
                };

                let len_repr = {
                    let mut len = Vec::new();
                    reader.read_until(b'-', &mut len).unwrap();
                    len.pop(); // Pop delimiter
                    String::from_utf8(len).unwrap().parse::<usize>().unwrap()
                };
                let repr = {
                    let mut repr = vec![0u8; len_repr];
                    reader.read_exact(&mut repr).unwrap();
                    Namespaced::deserialize(std::str::from_utf8(&repr).unwrap()).unwrap()
                };
                reader.fill_buf().unwrap();
                reader.consume(1); // Consume ';' delimiter

                let len_value = {
                    let mut len = Vec::new();
                    reader.read_until(b'-', &mut len).unwrap();
                    len.pop(); // Pop delimiter
                    String::from_utf8(len).unwrap().parse::<usize>().unwrap()
                };
                let value = {
                    let mut value = vec![0u8; len_value];
                    reader.read_exact(&mut value).unwrap();
                    Parameter::deserialize(std::str::from_utf8(&value).unwrap()).unwrap()
                };
                sender.send(Message::UpdateRepr(name, repr, value)).unwrap();
            }
            b'r' => {
                let name = {
                    let mut name = String::new();
                    reader.read_line(&mut name).unwrap();
                    name
                };
                sender.send(Message::RemoveData(name)).unwrap();
            }
            _ => {}
        }
    }
}
