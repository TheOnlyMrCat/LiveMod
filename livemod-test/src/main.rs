use livemod::{LiveMod, LiveModHandle, StructDataType, StructDataValue};

fn main() {
    let livemod = LiveModHandle::new_gui();

    let data = livemod.create_variable("Non-derived", Data::default());

    let mut prev_data = data.lock().value;
    println!("Non-derived: {}", prev_data);
    loop {
        let cur_value = data.lock().value;
        if cur_value != prev_data {
            println!("Non-derived: {}", cur_value);
            prev_data = cur_value;
        }
    }
}

#[derive(Default)]
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

    fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn set_self(&mut self, value: StructDataValue) {
        self.value = *value.as_unsigned_int().unwrap() as u32;
    }
}

// #[derive(Default, LiveMod)]
struct DerivedData {
    value_1: u32,
    value_2: i64,
}
