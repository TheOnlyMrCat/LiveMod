use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Deref, DerefMut};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use nanoserde::{DeBin, SerBin};
use parking_lot::{Mutex, MutexGuard, RwLock};

use crate::{LiveMod, TrackedDataValue, Trigger};

/// A handle to an external livemod viewer.
#[derive(Clone)]
pub struct LiveModHandle {
    sender: Sender<Message>,
    variables: Arc<RwLock<HashMap<String, ModVarHandle>>>,
}

impl LiveModHandle {
    /// Initialise livemod with the external `livemod-gui` user interface
    pub fn new_gui() -> LiveModHandle {
        Self::new_with_ui("livemod-gui")
    }

    /// Initialise livemod with an external user interface, for which the specified command will be run.
    pub fn new_with_ui(command: &str) -> LiveModHandle {
        let mut child = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let (sender, recv) = mpsc::channel();
        let output_sender = sender.clone();

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let child_arc1 = Arc::new(child);
        let child_arc2 = child_arc1.clone();

        let variables_arc1 = Arc::new(RwLock::new(HashMap::new()));
        let variables_arc2 = variables_arc1.clone();
        let variables_arc3 = variables_arc1.clone();


        std::thread::Builder::new()
            .name("livemod_input".to_owned())
            .spawn(|| {
                input_thread(stdin, recv, variables_arc2);
                drop(child_arc1);
            })
            .unwrap();
        std::thread::Builder::new()
            .name("livemod_output".to_owned())
            .spawn(|| {
                output_thread(stdout, output_sender, variables_arc3);
                drop(child_arc2);
            })
            .unwrap();

        LiveModHandle {
            sender,
            variables: variables_arc1,
        }
    }

    pub fn track_variable<T: LiveMod + 'static>(&self, name: &str, var: &'static StaticModVar<T>) {
        let var_handle = ModVarHandle {
            var: NonNull::from(&var.value),
        };
        self.sender
            .send(Message::NewVariable(name.to_owned(), var_handle))
            .unwrap();
    }

    /// Create a variable and send it to the external viewer to be tracked.
    pub fn create_variable<T: LiveMod + 'static>(&self, name: &str, var: T) -> ModVar<T> {
        let mod_var = ModVar {
            name: name.to_owned(),
            handle: self.clone(),
            value: Box::new(Mutex::new(var)),
        };
        let var_handle = ModVarHandle {
            var: NonNull::from(&*mod_var.value),
        };
        self.sender
            .send(Message::NewVariable(name.to_owned(), var_handle))
            .unwrap();
        //TODO: Duplicate name prevention
        mod_var
    }
}

/// A variable tracked by an external livemod viewer
///
/// A `ModVar` cannot be created directly, and must be created using the [`LiveModHandle::create_variable`] method.
pub struct ModVar<T> {
    name: String,
    handle: LiveModHandle,
    value: Box<Mutex<T>>,
}

impl<T: LiveMod + 'static> ModVar<T> {
    pub fn lock(&self) -> ModVarGuard<T> {
        ModVarGuard(self.value.lock())
    }

    pub fn lock_mut(&mut self) -> ModVarMutGuard<T> {
        ModVarMutGuard(self.value.lock(), Some(UpdateMessage::new(&self)))
    }
}

impl<T> Drop for ModVar<T> {
    fn drop(&mut self) {
        self.handle
            .sender
            .send(Message::RemoveVariable(self.name.clone()))
            .unwrap();
        self.handle.variables.write().remove(&self.name);
    }
}

/// A static trackable livemod variable
pub struct StaticModVar<T> {
    value: Mutex<T>,
}

impl<T> StaticModVar<T> {
    pub const fn new(value: T) -> StaticModVar<T> {
        StaticModVar {
            value: parking_lot::const_mutex(value),
        }
    }

    pub fn lock(&self) -> ModVarGuard<T> {
        ModVarGuard(self.value.lock())
    }
}

pub struct ModVarGuard<'a, T>(MutexGuard<'a, T>);

impl<'a, T> Deref for ModVarGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct ModVarMutGuard<'a, T>(MutexGuard<'a, T>, Option<UpdateMessage>);

impl<'a, T> Deref for ModVarMutGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, T> DerefMut for ModVarMutGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(msg) = self.1.take() {
            msg.send();
        }
        &mut *self.0
    }
}

struct UpdateMessage {
    name: String,
    handle: ModVarHandle,
    sender: Sender<Message>,
}

impl UpdateMessage {
    fn new<T: LiveMod + 'static>(var: &ModVar<T>) -> UpdateMessage {
        UpdateMessage {
            name: var.name.clone(),
            handle: ModVarHandle {
                var: NonNull::from(&*var.value),
            },
            sender: var.handle.sender.clone(),
        }
    }

    fn send(self) {
        self.sender
            .send(Message::UpdatedVariable(self.name, self.handle))
            .unwrap();
    }
}

#[derive(Clone, Copy)]
struct ModVarHandle {
    var: NonNull<Mutex<dyn LiveMod>>,
}

unsafe impl Send for ModVarHandle {}
unsafe impl Sync for ModVarHandle {}

enum Message {
    NewVariable(String, ModVarHandle),
    UpdatedVariable(String, ModVarHandle),
    RemoveVariable(String),
    UpdatedRepr(Vec<String>),
}

struct ChildDropper {
    child: Child,
}

impl Drop for ChildDropper {
    fn drop(&mut self) {
        self.child.wait().unwrap();
    }
}

fn input_thread(
    mut input: ChildStdin,
    recv: Receiver<Message>,
    variables: Arc<RwLock<HashMap<String, ModVarHandle>>>,
) {
    loop {
        match recv.recv() {
            Ok(message) => match message {
                Message::NewVariable(name, handle) => {
                    let var = unsafe { handle.var.as_ref() }.lock();
                    writeln!(
                        input,
                        "n{};{};{}",
                        &name,
                        base64::encode_config(
                            var.repr_default().serialize_bin(),
                            base64::STANDARD_NO_PAD
                        ),
                        base64::encode_config(
                            var.get_self().serialize_bin(),
                            base64::STANDARD_NO_PAD
                        ),
                    )
                    .unwrap();
                    variables.write().insert(name, handle);
                }
                Message::UpdatedVariable(name, handle) => {
                    let var = unsafe { handle.var.as_ref() }.lock();
                    writeln!(
                        input,
                        "s{};{}",
                        &name,
                        base64::encode_config(
                            var.get_self().serialize_bin(),
                            base64::STANDARD_NO_PAD
                        ),
                    )
                    .unwrap();
                }
                Message::UpdatedRepr(path) => {
                    // Get the 'base' variable from our HashMap
                    let base = path.first().unwrap();
                    let mut var_handle =
                        unsafe { &mut *variables.read().get(base).unwrap().var.as_ref().lock() };

                    // Follow the 'path' of field names, if needed
                    if path.len() > 2 {
                        for name in &path[1..=path.len() - 2] {
                            var_handle = var_handle.get_named_value(name);
                        }
                    }

                    writeln!(
                        input,
                        "u{};{};{}",
                        path.into_iter().reduce(|acc, v| format!("{}:{}", acc, v)).unwrap(),
                        base64::encode_config(
                            var_handle.repr_default().serialize_bin(),
                            base64::STANDARD_NO_PAD
                        ),
                        base64::encode_config(
                            var_handle.get_self().serialize_bin(),
                            base64::STANDARD_NO_PAD
                        ),                       
                    )
                    .unwrap();
                }
                Message::RemoveVariable(name) => {
                    writeln!(input, "r{}", &name).unwrap();
                }
            },
            Err(mpsc::RecvError) => {
                // The LiveModHandle which spawned this thread has
                // been destroyed, so quit and clean up now.
                break;
            }
        }
    }
    // Tell the child we're finished, so it can tell the output thread
    write!(input, "\0").unwrap();
}

fn output_thread(
    output: ChildStdout,
    sender: Sender<Message>,
    variables: Arc<RwLock<HashMap<String, ModVarHandle>>>,
) {
    for line in BufReader::new(output).lines() {
        let line = line.as_ref().unwrap().as_bytes();
        match line[0] {
            b'\0' => {
                // The LiveModHandle which spawned this thread has
                // been destroyed, the child informed of it, and the
                // child terminated, so quit the loop now.
                break;
            }
            b's' => {
                // Data is to be changed
                let namespaced_name = line[2..] // Not [1..], because the first character of the name will be ':'
                    .split(|&b| b == b':' || b == b';')
                    .map(std::str::from_utf8)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();

                // Get the 'base' variable from our HashMap
                let base = namespaced_name.first().unwrap();
                let mut var_handle =
                    unsafe { &mut *variables.read().get(*base).unwrap().var.as_ref().lock() };

                // Follow the 'path' of field names, if needed, ignoring the part after ';'
                if namespaced_name.len() > 2 {
                    for name in &namespaced_name[1..=namespaced_name.len() - 2] {
                        var_handle = var_handle.get_named_value(name);
                    }
                }

                // Set the variable
                if var_handle.trigger(Trigger::Set(
                    TrackedDataValue::deserialize_bin(
                        &base64::decode_config(
                            namespaced_name.last().unwrap(),
                            base64::STANDARD_NO_PAD,
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                )) {
                    sender.send(Message::UpdatedRepr(namespaced_name.into_iter().map(str::to_owned).collect())).unwrap();
                }
            }
            b't' => {
                // A trigger on an object
                let namespaced_name = line[2..] // Not [1..], because the first character of the name will be ':'
                    .split(|&b| b == b':' || b == b';')
                    .map(std::str::from_utf8)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();

                // Get the 'base' variable from our HashMap
                let base = namespaced_name.first().unwrap();
                let mut var_handle =
                    unsafe { &mut *variables.read().get(*base).unwrap().var.as_ref().lock() };

                // Follow the 'path' of field names, if needed, ignoring the part after ';'
                if namespaced_name.len() > 2 {
                    for name in &namespaced_name[1..=namespaced_name.len() - 2] {
                        var_handle = var_handle.get_named_value(name);
                    }
                }

                // Trigger the action denoted by the last element of the name
                if var_handle.trigger(Trigger::Trigger(
                    (*namespaced_name.last().unwrap()).to_owned(),
                )) {
                    let len = namespaced_name.len() - 1;
                    sender.send(Message::UpdatedRepr(namespaced_name.into_iter().take(len).map(str::to_owned).collect())).unwrap();
                }
            }
            _ => {
                debug_assert!(false, "Unexpected output from child process")
            }
        }
    }
}
