use livemod::{
    livemod_static, LiveMod, LiveModHandle, Multiline, Slider, TrackedDataRepr, TrackedDataValue,
    Trigger, TriggerFn,
};

livemod_static! {
    static STRAIGHT_VALUE: f32 = 0.0;
    static NON_DERIVED: Data = Data { value: 64 };
}

fn main() {
    let livemod = LiveModHandle::new_gui();

    livemod.track_variable("Float", &STRAIGHT_VALUE);
    livemod.track_variable("Non-derived", &NON_DERIVED);
    let mut derived = livemod.create_variable("Derived", DerivedData::default());
    let mut can_remove = Some(livemod.create_variable("Remove me", false));
    let _trigger =
        livemod.create_variable("Trigger", TriggerFn::new((), |()| println!("Triggered!")));

    let mut prev_float = *STRAIGHT_VALUE.lock();
    let mut prev_nonderived = NON_DERIVED.lock().value;
    let mut prev_derived = derived.lock().clone();
    println!("Float: {}", prev_float);
    println!("Non-derived: {}", prev_nonderived);
    println!("Derived: {:?}", prev_derived);
    loop {
        let cur_float = *STRAIGHT_VALUE.lock();
        let cur_nonderived = NON_DERIVED.lock().value;
        let mut cur_derived = derived.lock_mut();
        if cur_float != prev_float {
            println!("Float: {}", cur_float);
            prev_float = cur_float;
        }
        if cur_nonderived != prev_nonderived {
            println!("Non-derived: {}", cur_nonderived);
            prev_nonderived = cur_nonderived;
        }
        if *cur_derived != prev_derived {
            println!("Derived: {:?}", *cur_derived);
            prev_derived = cur_derived.clone();
            if cur_derived.floating_point != 3.2 {
                cur_derived.floating_point = 3.2;
            }
        }
        if let Some(r) = can_remove {
            if *r.lock() {
                can_remove = None;
                println!("Checkbox removed");
            } else {
                can_remove = Some(r);
            }
        }
    }
}

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

    fn trigger(&mut self, trigger: Trigger) {
        self.value = *trigger.try_into_set().unwrap().as_unsigned_int().unwrap() as u32;
    }

    fn get_self(&self) -> TrackedDataValue {
        TrackedDataValue::UnsignedInt(self.value as u64)
    }
}

#[derive(Debug, LiveMod, PartialEq, Clone)]
struct DerivedData {
    #[livemod(repr = Slider(0..=500))]
    value_1: u32,
    #[livemod(rename = "signed value")]
    value_2: i64,
    floating_point: f32,
    #[livemod(repr = Slider(-5.0..=10.0))]
    double_float: f64,
    #[livemod(skip)]
    runtime_flag: bool,
    toggleable_flag: bool,
    singleline_string: String,
    #[livemod(repr = Multiline)]
    multiline_string: String,
}

impl Default for DerivedData {
    fn default() -> Self {
        DerivedData {
            value_1: 1,
            value_2: 2,
            floating_point: 3.2,
            double_float: 6.4,
            runtime_flag: false,
            toggleable_flag: true,
            singleline_string: "One line".to_owned(),
            multiline_string: "Multiple\nlines".to_owned(),
        }
    }
}
