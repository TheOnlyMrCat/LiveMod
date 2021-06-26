use livemod::{Slider, LiveMod, LiveModHandle, TrackedDataRepr, TrackedDataValue};

fn main() {
    let livemod = LiveModHandle::new_gui();

    let nonderived = livemod.create_variable("Non-derived", Data::default());
    let derived = livemod.create_variable("Derived", DerivedData::default());

    let mut prev_nonderived = nonderived.lock().value;
    let mut prev_derived = *derived.lock();
    println!("Non-derived: {}", prev_nonderived);
    println!("Derived: {:?}", prev_derived);
    loop {
        let cur_nonderived = nonderived.lock().value;
        let cur_derived = *derived.lock();
        if cur_nonderived != prev_nonderived {
            println!("Non-derived: {}", cur_nonderived);
            prev_nonderived = cur_nonderived;
        }
        if cur_derived != prev_derived {
            println!("Derived: {:?}", cur_derived);
            prev_derived = cur_derived;
        }
    }
}

#[derive(Default)]
struct Data {
    value: u32,
}

impl LiveMod for Data {
    fn data_type(&self) -> TrackedDataRepr {
        livemod::TrackedDataRepr::UnsignedSlider {
            storage_min: u32::MIN as u64,
            storage_max: u32::MAX as u64,
            suggested_min: 1,
            suggested_max: 100,
        }
    }

    fn get_named_value(&mut self, _: &str) -> &mut dyn LiveMod {
        unimplemented!()
    }

    fn set_self(&mut self, value: TrackedDataValue) {
        self.value = *value.as_unsigned_int().unwrap() as u32;
    }
}

#[derive(Default, Debug, LiveMod, PartialEq, Clone, Copy)]
struct DerivedData {
    #[livemod(repr = Slider(0..=500))]
    value_1: u32,
    #[livemod(rename = "signed value")]
    value_2: i64,
    #[livemod(preserve_case)]
    keep_me_lowercase: u32,
    #[livemod(skip)]
    runtime_flag: bool,
}
