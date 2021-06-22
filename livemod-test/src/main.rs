use livemod::{LiveModData, LiveModHandle, ModVar, StructData, StructDataValue};

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

impl LiveModData for Data {
    fn get_data_types(&self) -> StructData {
        StructData {
            name: "value".to_owned(),
            data_type: livemod::StructDataType::UnsignedSlider {
                storage_min: u32::MIN as u64,
                storage_max: u32::MAX as u64,
                suggested_min: 1,
                suggested_max: 100,
            },
        }
    }

    fn set_by_name(&mut self, name: &str, value: StructDataValue) {
        match name {
            "value" => {
                self.value = *value.as_unsigned_int().unwrap() as u32;
            }
            _ => panic!(),
        }
    }
}
