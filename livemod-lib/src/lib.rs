//! # livemod

#[cfg(feature = "livemod-derive")]
pub use livemod_derive::LiveMod;

use std::any::Any;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Deref, Range};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use nanoserde::{DeBin, SerBin};
use parking_lot::RwLock;

#[derive(Clone)]
pub struct LiveModHandle {
    sender: Sender<ModVarHandle>,
    variables: Arc<RwLock<HashMap<String, ModVarHandle>>>,
}

impl LiveModHandle {
    /// Initialise livemod with the external `livemod-gui` user interface
    pub fn new_gui() -> LiveModHandle {
        Self::new_with_ui("livemod-gui")
    }

    /// Initialise livemod with the external `livemod-tui` user interface
    pub fn new_term() -> LiveModHandle {
        Self::new_with_ui("livemod-tui")
    }

    /// Initialise livemod with an external user interface, for which the specified command will be run.
    pub fn new_with_ui<'a>(command: &str) -> LiveModHandle {
        let mut child = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let (sender, recv) = mpsc::channel();

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

    pub fn track_variable(&self, var: ModVarHandle) {
        self.sender.send(var).unwrap();
    }
}

pub trait LiveMod {
    fn data_type(&self) -> StructDataType;
    fn get_named_value(&mut self, name: &str) -> &mut dyn LiveMod;
    fn set_self(&mut self, value: StructDataValue);
}

#[derive(SerBin, DeBin)]
pub struct StructData {
    pub name: String,
    pub data_type: StructDataType,
}

#[derive(SerBin, DeBin)]
pub enum StructDataType {
    SignedSlider {
        storage_min: i64,
        storage_max: i64,
        suggested_min: i64,
        suggested_max: i64,
    },
    UnsignedSlider {
        storage_min: u64,
        storage_max: u64,
        suggested_min: u64,
        suggested_max: u64,
    },
    Struct {
        name: String,
        fields: Vec<StructData>,
    },
}

#[derive(SerBin, DeBin)]
pub enum StructDataValue {
    SignedInt(i64),
    UnsignedInt(u64),
}

impl StructDataValue {
    pub fn as_signed_int(&self) -> Option<&i64> {
        if let Self::SignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_unsigned_int(&self) -> Option<&u64> {
        if let Self::UnsignedInt(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ModVarHandle {
    var: &'static UnsafeCell<dyn LiveMod>,
}

unsafe impl Send for ModVarHandle {}
unsafe impl Sync for ModVarHandle {}

#[repr(transparent)]
pub struct ModVar<T> {
    cell: UnsafeCell<T>,
}

impl<T> ModVar<T> {
    pub const fn new(value: T) -> ModVar<T> {
        ModVar {
            cell: UnsafeCell::new(value),
        }
    }
}

impl<T: LiveMod> ModVar<T> {
    pub fn get_handle(&'static self) -> ModVarHandle {
        ModVarHandle { var: &self.cell }
    }
}

impl<T> Deref for ModVar<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.get() }
    }
}

unsafe impl<T> Send for ModVar<T> {}
unsafe impl<T> Sync for ModVar<T> {}

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
    recv: Receiver<ModVarHandle>,
    variables: Arc<RwLock<HashMap<String, ModVarHandle>>>,
) {
    loop {
        match recv.try_recv() {
            Ok(handle) => {
                let data_type = unsafe { (*handle.var.get()).data_type() };
                writeln!(
                    input,
                    "n{};{}",
                    "value",
                    base64::encode_config(data_type.serialize_bin(), base64::STANDARD_NO_PAD)
                )
                .unwrap();
                variables.write().insert("value".to_owned(), handle);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
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
                    unsafe { &mut *(*variables.read().get(base).unwrap()).var.get() };
                if namespaced_name.len() > 2 {
                    for name in &namespaced_name[1..namespaced_name.len()-2] {
                        let name = std::str::from_utf8(name).unwrap();
                        var_handle = var_handle.get_named_value(name);
                    }
                }
                var_handle.set_self(
                    StructDataValue::deserialize_bin(
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
