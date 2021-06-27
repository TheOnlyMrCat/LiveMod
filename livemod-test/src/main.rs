use livemod::{LiveMod, LiveModHandle, Multiline, Slider, TrackedDataRepr, TrackedDataValue, livemod_static};

livemod_static! {
    static STRAIGHT_VALUE: f32 = 0.0;
    static NON_DERIVED: Data = Data { value: 0 };
}

fn main() {
    let livemod = LiveModHandle::new_gui();

    livemod.track_variable("Float", &STRAIGHT_VALUE);
    livemod.track_variable("Non-derived", &NON_DERIVED);
    let derived = livemod.create_variable("Derived", DerivedData::default());

    let mut prev_float = *STRAIGHT_VALUE.lock();
    let mut prev_nonderived = NON_DERIVED.lock().value;
    let mut prev_derived = derived.lock().clone();
    println!("Float: {}", prev_float);
    println!("Non-derived: {}", prev_nonderived);
    println!("Derived: {:?}", prev_derived);
    loop {
        let cur_float = *STRAIGHT_VALUE.lock();
        let cur_nonderived = NON_DERIVED.lock().value;
        let cur_derived = derived.lock().clone();
        if cur_float != prev_float {
            println!("Float: {}", cur_float);
            prev_float = cur_float;
        }
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

#[derive(Default, Debug, LiveMod, PartialEq, Clone)]
struct DerivedData {
    #[livemod(repr = Slider(0..=500))]
    value_1: u32,
    #[livemod(rename = "signed value")]
    value_2: i64,
    floating_point: f32,
    #[livemod(repr = Slider(-5.0..=10.0))]
    double_float: f64,
    #[livemod(preserve_case)]
    keep_me_lowercase: u32,
    #[livemod(skip)]
    runtime_flag: bool,
    toggleable_flag: bool,
    singleline_string: String,
    #[livemod(repr = Multiline)]
    multiline_string: String,
}
