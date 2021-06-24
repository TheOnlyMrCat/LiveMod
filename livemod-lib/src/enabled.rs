use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Deref, DerefMut};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;

use nanoserde::{DeBin, SerBin};
use parking_lot::{Mutex, MutexGuard, RwLock};

use crate::{LiveMod, TrackedDataValue};

/// A handle to an external livemod viewer.
#[derive(Clone)]
pub struct LiveModHandle {
    sender: SyncSender<Message>,
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
        let (sender, recv) = mpsc::sync_channel(1);

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
                output_thread(stdout, variables_arc3);
                drop(child_arc2);
            })
            .unwrap();

        LiveModHandle {
            sender,
            variables: variables_arc1,
        }
    }

    /// Create a variable and send it to the external viewer to be tracked.
    ///
    /// TODO: Remove the variable from the external viewer when dropped
    pub fn create_variable<T: 'static + LiveMod>(&self, name: &str, var: T) -> ModVar<T> {
        let mod_var = ModVar {
            name: name.to_owned(),
            handle: self.clone(),
            value: Box::new(Mutex::new(var)),
        };
        let var_handle = ModVarHandle {
            var: &*mod_var.value as *const _,
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

impl<T> ModVar<T> {
    pub fn lock(&self) -> ModVarGuard<T> {
        ModVarGuard(self.value.lock())
    }

    pub fn lock_mut(&mut self) -> ModVarMutGuard<T> {
        ModVarMutGuard(self.value.lock())
        //TODO: Update value in GUI
    }
}

impl<T> Drop for ModVar<T> {
    fn drop(&mut self) {
        self.handle.variables.write().remove(&self.name);
    }
}

pub struct ModVarGuard<'a, T>(MutexGuard<'a, T>);

impl<'a, T> Deref for ModVarGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct ModVarMutGuard<'a, T>(MutexGuard<'a, T>);

impl<'a, T> Deref for ModVarMutGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, T> DerefMut for ModVarMutGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

#[derive(Clone, Copy)]
struct ModVarHandle {
    var: *const Mutex<dyn LiveMod>,
}

unsafe impl Send for ModVarHandle {}
unsafe impl Sync for ModVarHandle {}

enum Message {
    NewVariable(String, ModVarHandle),
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
                    let data_type = unsafe { (*handle.var).lock() }.data_type();
                    writeln!(
                        input,
                        "n{};{}",
                        &name,
                        base64::encode_config(data_type.serialize_bin(), base64::STANDARD_NO_PAD)
                    )
                    .unwrap();
                    variables.write().insert(name, handle);
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

fn output_thread(output: ChildStdout, variables: Arc<RwLock<HashMap<String, ModVarHandle>>>) {
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
                    .split(|&b| b == b':' || b == b'=')
                    .collect::<Vec<_>>();
                let base = std::str::from_utf8(namespaced_name.first().unwrap()).unwrap();
                let mut var_handle =
                    unsafe { &mut *(*variables.read().get(base).unwrap().var).lock() };
                if namespaced_name.len() > 2 {
                    for name in &namespaced_name[1..=namespaced_name.len() - 2] {
                        let name = std::str::from_utf8(name).unwrap();
                        var_handle = var_handle.get_named_value(name);
                    }
                }
                var_handle.set_self(
                    TrackedDataValue::deserialize_bin(
                        &base64::decode_config(
                            line[line.iter().position(|&b| b == b'=').unwrap() + 1..].to_owned(),
                            base64::STANDARD_NO_PAD,
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                )
            }
            _ => {
                debug_assert!(false, "Unexpected output from child process")
            }
        }
    }
}