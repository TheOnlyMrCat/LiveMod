use livemod::{LiveMod, LiveModHandle, ModVar, StructDataType, StructDataValue};

fn main() {
    let livemod = LiveModHandle::new_gui();

    static DATA: ModVar<Data> = ModVar::new(Data { value: 0 });
    livemod.track_variable(DATA.get_handle());

    println!("Value: {}", DATA.value);
    let mut prev_data = DATA.value;
    loop {
        if DATA.value != prev_data {
            println!("Changed: {}", DATA.value);
            prev_data = DATA.value;
        }
    }
}

struct Data {
    value: u32,
}

impl LiveMod for Data {
    fn data_type(&self) -> StructDataType {
        livemod::StructDataType::UnsignedSlider {
            storage_min: u32::MIN as u64,
            storage_max: u32::MAX as u64,
            suggested_min: 1,
            suggested_max: 100,
        }
    }

    fn get_named_value(&mut self, name: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn set_self(&mut self, value: StructDataValue) {
        self.value = *value.as_unsigned_int().unwrap() as u32;
    }
}

#[derive(LiveMod)]
struct DerivedData {
    value_1: u32,
    value_2: i64,
}
