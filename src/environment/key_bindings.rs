use bidir_map::BidirMap;
use piston_window::{
    Button,
    Key,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum BindRoles {
    BT_A,
    BT_B,
    BT_C,
    BT_D,
    FX_L,
    FX_R,
    KN_L_CW,
    KN_L_CCW,
    KN_R_CW,
    KN_R_CCW,
    START,

    BACK,
}

impl BindRoles {
    pub fn default_keyboard_binding() -> BidirMap<BindRoles, ComposedKeystroke>
    {
        use piston_window::keyboard::Key;
        use BindRoles::*;

        let mut map = BidirMap::new();

        for (role, ks) in [
            (BT_A, ComposedKeystroke::new(Key::D)),
            (BT_B, ComposedKeystroke::new(Key::F)),
            (BT_C, ComposedKeystroke::new(Key::J)),
            (BT_D, ComposedKeystroke::new(Key::K)),
            (FX_L, ComposedKeystroke::new(Key::V)),
            (FX_R, ComposedKeystroke::new(Key::N)),
            (KN_L_CW, ComposedKeystroke::new(Key::R)),
            (KN_L_CCW, ComposedKeystroke::new(Key::E)),
            (KN_R_CW, ComposedKeystroke::new(Key::I)),
            (KN_R_CCW, ComposedKeystroke::new(Key::U)),
            (START, ComposedKeystroke::new(Key::Return)),
            (BACK, ComposedKeystroke::new(Key::Escape)),
        ]
        .into_iter()
        {
            map.insert(role, ks);
        }

        map
    }
}

#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum GeneralizedKeystroke {
    Keyboard(Key),
    Controller(u8),
}

impl GeneralizedKeystroke {
    pub fn is(
        &self,
        button: &Button,
    ) -> bool
    {
        use Button as B;
        use GeneralizedKeystroke as GK;

        match (self, button) {
            (GK::Keyboard(k1), B::Keyboard(k2)) => k1 == k2,
            (GK::Controller(k), B::Controller(c)) => k == c.button,
            _ => false,
        }
    }

    pub fn from_button(button: &Button) -> Option<GeneralizedKeystroke> {
        use Button as B;
        use GeneralizedKeystroke as GK;

        match button {
            B::Keyboard(k) => Some(GK::Keyboard(k)),
            B::Controller(c) => Some(GK::Controller(c.button)),
            _ => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ComposedKeystroke(Vec<GeneralizedKeystroke>);

impl ComposedKeystroke {
    pub fn new(key: GeneralizedKeystroke) -> ComposedKeystroke {
        ComposedKeystroke::new(vec![key])
    }

    pub fn add(
        self,
        key: GeneralizedKeystroke,
    ) -> ComposedKeystroke
    {
        let bin_srch_idx = self.0.iter().binary_search(&key);

        // check if the key already exists
        match bin_srch_idx {
            Ok(_) => return,
            Err(e) => self.0.insert(e, key),
        }
    }

    pub fn contains(
        &self,
        key: GeneralizedKeystroke,
    ) -> bool
    {
        self.0.binary_search(&key).is_ok()
    }
}
