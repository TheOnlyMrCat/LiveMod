use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};

use glium::glutin;
use livemod::{StructData, StructDataValue};
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
    let mut current_variables = HashMap::new();
    let mut values = Values::default();
    let mut modified_variables = HashMap::new();

    event_loop.run(move |event, _, control_flow| match event {
        glutin::event::Event::MainEventsCleared => {
            while let Ok(msg) = recv.try_recv() {
                match msg {
                    Message::NewData(s) => {
                        current_variables.insert(s.name.clone(), s);
                    }
                }
            }

            egui.begin_frame(&display);

            egui::CentralPanel::default().show(&egui.ctx(), |ui| {
                egui::Grid::new("base_grid")
                    .striped(true)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        recursive_ui(ui, &mut values, &mut modified_variables, current_variables.values(), String::new())
                    });
            });

            for (name, value) in modified_variables.drain() {
                print!("s{}=", name);
                println!("{}", base64::encode_config(value.serialize_bin(), base64::STANDARD_NO_PAD));
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

#[derive(Default)]
struct Values {
    i64: HashMap<String, i64>,
    u64: HashMap<String, u64>,
}

enum Message {
    NewData(StructData),
}

fn recursive_ui<'a>(
    ui: &mut egui::Ui,
    values: &mut Values,
    modified_variables: &mut HashMap<String, StructDataValue>,
    variables: impl Iterator<Item = &'a StructData>,
    namespace: String,
) {
    for var in variables {
        ui.label(var.name.clone());
        let namespaced_name = format!("{}:{}", namespace, var.name);
        match &var.data_type {
            livemod::StructDataType::Struct { name, fields } => {
                ui.collapsing(name, |ui| {
                    egui::Grid::new(format!("{}_grid", &namespaced_name))
                        .striped(true)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            recursive_ui(ui, values, modified_variables, fields.iter(), namespaced_name)
                        });
                });
            }
            livemod::StructDataType::SignedSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => {
                ui.add(
                    egui::Slider::from_get_set(
                        (*suggested_min) as f64..=(*suggested_max) as f64,
                        |val| {
                            *match val {
                                Some(val) => {
                                    values.i64.insert(namespaced_name.clone(), (val as i64).clamp(*storage_min, *storage_max));
                                    let v = values.i64.get(&namespaced_name).unwrap();
                                    modified_variables.insert(namespaced_name.clone(), livemod::StructDataValue::SignedInt(*v));
                                    v
                                }
                                None => values.i64.entry(namespaced_name.clone()).or_default(),
                            } as f64
                        },
                    )
                    .integer(),
                );
            }
            livemod::StructDataType::UnsignedSlider {
                storage_min,
                storage_max,
                suggested_min,
                suggested_max,
            } => {
                ui.add(
                    egui::Slider::from_get_set(
                        (*suggested_min) as f64..=(*suggested_max) as f64,
                        |val| {
                            *match val {
                                Some(val) => {
                                    values.u64.insert(namespaced_name.clone(), (val as u64).clamp(*storage_min, *storage_max));
                                    let v = values.u64.get(&namespaced_name).unwrap();
                                    modified_variables.insert(namespaced_name.clone(), livemod::StructDataValue::UnsignedInt(*v));
                                    v
                                }
                                None => values.u64.entry(namespaced_name.clone()).or_default(),
                            } as f64
                        },
                    )
                    .integer(),
                );
            }
        }
        ui.end_row();
    }
}

fn reader_thread(sender: Sender<Message>) {
    for line in BufReader::new(std::io::stdin()).lines() {
        let line = line.unwrap();
        let line = line.as_bytes();
        match line[0] {
            b'\0' => break,
            b'n' => {
                // New variable to track
                sender
                    .send(Message::NewData(
                        StructData::deserialize_bin(
                            &base64::decode_config(&line[1..], base64::STANDARD_NO_PAD).unwrap(),
                        )
                        .unwrap(),
                    ))
                    .unwrap();
            }
            _ => {
                debug_assert!(false, "Unexpected input")
            }
        }
    }
}
